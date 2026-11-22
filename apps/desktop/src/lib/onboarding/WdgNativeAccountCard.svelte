<script lang="ts">
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import { labels as backupLabels } from '../backup/labels';
	import { i18n } from '../i18n/index.svelte';
	import type { ConnectionDiagnosticCode } from '../inbox/types';
	import type { AccountAlias } from './aliases';
	import type { AvatarTransport } from './avatar-types';
	import type { AccountSubscription, BadgeOverride } from './subscriptions';
	import type { AccountConfig, Reauthorization } from './types';
	import WdgAccountAliasesControl from './WdgAccountAliasesControl.svelte';
	import WdgAccountAvatar from './WdgAccountAvatar.svelte';
	import WdgAccountFolderTable, { type FolderCatalogState } from './WdgAccountFolderTable.svelte';
	import WdgAccountPlanBadge from './WdgAccountPlanBadge.svelte';
	import WdgAccountPlanControl from './WdgAccountPlanControl.svelte';
	import WdgAccountReauthForm from './WdgAccountReauthForm.svelte';
	import WdgProviderDiagnosticHint from './WdgProviderDiagnosticHint.svelte';

	interface Props {
		account: AccountConfig;
		authLabel: string;
		expanded: boolean;
		catalog?: FolderCatalogState;
		busy: boolean;
		authorizing: boolean;
		reauthorizingAccountId: string | null;
		reauthOpen: boolean;
		reauthSuccess: boolean;
		reauthFailed: boolean;
		accountDiag?: ConnectionDiagnosticCode;
		subscription?: AccountSubscription;
		badgeError: boolean;
		aliases: AccountAlias[];
		aliasError: boolean;
		diagnostic: ConnectionDiagnosticCode | null;
		error: string;
		avatarTransport?: AvatarTransport;
		onSetBadge?: (accountId: string, override: BadgeOverride) => Promise<void>;
		onSetAliases?: (accountId: string, aliases: AccountAlias[]) => Promise<boolean>;
		onToggleFolders: () => void;
		onRetryFolders: () => void | Promise<void>;
		onToggleReauthorize: () => void;
		onCloseReauthorize: () => void;
		onReauthSuccess: () => void;
		onReauthFailure: () => void;
		onReauthorize?: (input: Reauthorization) => Promise<boolean>;
		onCancelReauthorize?: () => Promise<void>;
	}

	let {
		account,
		authLabel,
		expanded,
		catalog,
		busy,
		authorizing,
		reauthorizingAccountId,
		reauthOpen,
		reauthSuccess,
		reauthFailed,
		accountDiag,
		subscription,
		badgeError,
		aliases,
		aliasError,
		diagnostic,
		error,
		avatarTransport,
		onSetBadge,
		onSetAliases,
		onToggleFolders,
		onRetryFolders,
		onToggleReauthorize,
		onCloseReauthorize,
		onReauthSuccess,
		onReauthFailure,
		onReauthorize,
		onCancelReauthorize
	}: Props = $props();

	const t = $derived(i18n.t.onboarding);
	const formT = $derived(i18n.t.accountForm);
	const bLabels = $derived(backupLabels[i18n.locale]);
	const isOAuth = $derived(
		account.auth_method === 'OAuth2PublicPkce' || account.auth_method === 'OAuth2ByoCredentials'
	);
</script>

<article class="rounded-lg border border-separator bg-surface">
	<div class="flex flex-wrap items-center justify-between gap-4 p-4">
		<div class="flex min-w-0 flex-1 items-center gap-4">
			<div class="min-w-0 space-y-1">
				<div class="flex flex-wrap items-center gap-2">
					<span class="truncate text-sm font-semibold text-foreground">{account.email_display}</span>
					<span class="rounded border border-separator bg-field px-2 py-1 font-mono text-[11px] font-medium text-foreground">
						{authLabel}
					</span>
					<WdgAccountPlanBadge label={subscription?.effective_badge ?? null} quota={subscription?.quota} />
				</div>
				<div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-secondary">
					<span class="font-mono text-foreground">{account.account_id}</span>
					<span aria-hidden="true">·</span>
					{#if account.auth_method === 'LocalArchive'}
						<span>{formT.localArchiveBadge}</span>
					{:else if !account.imap_host}
						<span>{bLabels.disconnected}</span>
					{:else}
						<span class="font-mono tabular-nums">{account.imap_host}:{account.imap_port}</span>
					{/if}
				</div>
				<div class="pt-1">
					<WdgAccountAvatar accountId={account.account_id} label={account.email_display} transport={avatarTransport} />
				</div>
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			{#if isOAuth && onReauthorize}
				<button
					type="button"
					disabled={busy}
					aria-expanded={reauthOpen}
					onclick={onToggleReauthorize}
					class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
				>
					<KeyRound class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{reauthorizingAccountId === account.account_id ? t.reauthorizing : t.reauthorize}</span>
				</button>
			{/if}

			<button
				type="button"
				onclick={onToggleFolders}
				aria-expanded={expanded}
				class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field"
			>
				{#if expanded}
					<ChevronDown class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				{:else}
					<ChevronRight class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				{/if}
				<span>
					{t.foldersCount}
					{#if catalog?.loaded}
						(<span class="font-mono tabular-nums">{catalog.folders.length}</span>)
					{/if}
				</span>
			</button>
		</div>
	</div>

	{#if onSetBadge && subscription && account.auth_method !== 'LocalArchive'}
		<WdgAccountPlanControl
			{subscription}
			failed={badgeError}
			onSetBadge={(override) => onSetBadge(account.account_id, override)}
		/>
	{/if}

	{#if onSetAliases && account.auth_method !== 'LocalArchive'}
		<WdgAccountAliasesControl
			accountId={account.account_id}
			{aliases}
			failed={aliasError}
			onSave={(next) => onSetAliases(account.account_id, next)}
		/>
	{/if}

	{#if accountDiag}
		{@const guidance = formT.diagnostics[accountDiag]}
		<div role="alert" class="flex items-start gap-3 border-t border-separator bg-field/80 px-4 py-3 text-xs">
			<AlertTriangle class="mt-1 h-4 w-4 shrink-0 text-status-warning" aria-hidden="true" />
			<div class="space-y-1">
				<p class="font-semibold text-foreground">{t.accountDiagnosticBanner}: {guidance.title}</p>
				<p class="leading-relaxed text-secondary">{guidance.hint}</p>
				<WdgProviderDiagnosticHint code={accountDiag} host={account.imap_host} method={account.auth_method} />
			</div>
		</div>
	{/if}

	{#if reauthSuccess}
		<div
			role="status"
			class="flex items-center gap-2 border-t border-separator bg-field/70 px-4 py-3 text-xs font-medium text-status-healthy"
		>
			<CheckCircle2 class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
			<span>{t.reauthorizeSuccess}</span>
		</div>
	{/if}

	{#if reauthOpen && onReauthorize}
		<WdgAccountReauthForm
			{account}
			{busy}
			{authorizing}
			{reauthorizingAccountId}
			failed={reauthFailed}
			{diagnostic}
			{error}
			{onReauthorize}
			{onCancelReauthorize}
			onClose={onCloseReauthorize}
			onSuccess={onReauthSuccess}
			onFailure={onReauthFailure}
		/>
	{/if}

	{#if expanded}
		<WdgAccountFolderTable {catalog} onRetry={onRetryFolders} />
	{/if}
</article>
