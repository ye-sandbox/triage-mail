<script lang="ts">
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import MailPlus from '@lucide/svelte/icons/mail-plus';
	import { i18n } from '../i18n/index.svelte';
	import { nativeTransport } from '../inbox/transport';
	import type { ConnectionDiagnosticCode, FolderDto } from '../inbox/types';
	import type { AccountAlias, AccountAliasMap } from './aliases';
	import type { AvatarTransport } from './avatar-types';
	import type { DemoConnectedAccountDto } from './onboarding.fixtures';
	import type { AccountSubscription, BadgeOverride } from './subscriptions';
	import type { AccountConfig, Reauthorization } from './types';
	import type { FolderCatalogState } from './WdgAccountFolderTable.svelte';
	import WdgDemoAccountCard from './WdgDemoAccountCard.svelte';
	import WdgNativeAccountCard from './WdgNativeAccountCard.svelte';

	interface Props {
		accounts?: AccountConfig[];
		demoAccounts?: DemoConnectedAccountDto[];
		demoMode?: boolean;
		busy?: boolean;
		authorizing?: boolean;
		reauthorizingAccountId?: string | null;
		accountDiagnostics?: Record<string, ConnectionDiagnosticCode>;
		subscriptions?: Record<string, AccountSubscription>;
		badgeError?: boolean;
		onSetBadge?: (accountId: string, override: BadgeOverride) => Promise<void>;
		aliases?: AccountAliasMap;
		aliasError?: boolean;
		onSetAliases?: (accountId: string, aliases: AccountAlias[]) => Promise<boolean>;
		diagnostic?: ConnectionDiagnosticCode | null;
		error?: string;
		avatarTransport?: AvatarTransport;
		listFolders?: (accountId: string) => Promise<FolderDto[]>;
		onAddAccount: () => void;
		onOpenInbox?: () => void;
		onReauthorize?: (input: Reauthorization) => Promise<boolean>;
		onCancelReauthorize?: () => Promise<void>;
		onUpdateDemoAvatar?: (accountId: string, avatarUrl: string | null) => void;
		onRemoveDemoAccount?: (accountId: string) => void;
	}

	let {
		accounts = [],
		demoAccounts = [],
		demoMode = false,
		busy = false,
		authorizing = false,
		reauthorizingAccountId = null,
		accountDiagnostics = {},
		subscriptions = {},
		badgeError = false,
		onSetBadge,
		aliases = {},
		aliasError = false,
		onSetAliases,
		diagnostic = null,
		error = '',
		avatarTransport,
		listFolders = nativeTransport.listFolders,
		onAddAccount,
		onOpenInbox,
		onReauthorize,
		onCancelReauthorize,
		onUpdateDemoAvatar,
		onRemoveDemoAccount
	}: Props = $props();

	let expandedIds = $state<Record<string, boolean>>({});
	let folderCatalogs = $state<Record<string, FolderCatalogState>>({});
	let reauthTargetId = $state<string | null>(null);
	let reauthSuccessAccountId = $state<string | null>(null);
	let reauthFailedAccountId = $state<string | null>(null);
	let active = true;

	const t = $derived(i18n.t.onboarding);
	const formT = $derived(i18n.t.accountForm);
	const totalCount = $derived(demoMode ? demoAccounts.length : accounts.length);

	$effect(() => {
		active = true;
		return () => {
			active = false;
		};
	});

	async function loadAccountFolders(accountId: string) {
		if (!listFolders) return;
		folderCatalogs = {
			...folderCatalogs,
			[accountId]: {
				loading: true,
				error: false,
				folders: folderCatalogs[accountId]?.folders ?? [],
				loaded: false
			}
		};
		try {
			const folders = await listFolders(accountId);
			if (!active) return;
			folderCatalogs = {
				...folderCatalogs,
				[accountId]: { loading: false, error: false, folders, loaded: true }
			};
		} catch {
			if (!active) return;
			folderCatalogs = {
				...folderCatalogs,
				[accountId]: { loading: false, error: true, folders: [], loaded: false }
			};
		}
	}

	function toggleFolders(accountId: string, loadNative: boolean) {
		const next = !expandedIds[accountId];
		expandedIds = { ...expandedIds, [accountId]: next };
		if (next && loadNative) {
			const existing = folderCatalogs[accountId];
			if (!existing || (!existing.loading && !existing.loaded)) {
				void loadAccountFolders(accountId);
			}
		}
	}

	function authLabel(method: AccountConfig['auth_method']): string {
		if (method === 'AppPassword') return formT.methodAppPassword;
		if (method === 'ProtonBridgeLocal') return formT.methodProton;
		if (method === 'OAuth2PublicPkce') return formT.methodOAuthPublic;
		if (method === 'LocalArchive') return formT.methodLocalArchive;
		return formT.methodOAuthByo;
	}
</script>

<section aria-label={t.connectedAccounts} class="mx-auto flex w-full max-w-4xl flex-col gap-4">
	<div class="flex flex-wrap items-center justify-between gap-3 border-b border-separator pb-3">
		<div>
			<h2 class="font-display text-base font-semibold text-foreground">{t.connectedAccounts}</h2>
			<p class="mt-1 text-xs text-secondary">{t.accountsTitle}</p>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			{#if totalCount > 0 && onOpenInbox}
				<button
					type="button"
					disabled={busy}
					onclick={onOpenInbox}
					class="flex min-h-[40px] items-center gap-2 rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
				>
					<FolderOpen class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{t.openInbox}</span>
				</button>
			{/if}
			<button
				type="button"
				disabled={busy}
				onclick={onAddAccount}
				class="flex min-h-[40px] items-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap shrink-0 hover:opacity-95 active:scale-[0.98] disabled:opacity-50"
			>
				<MailPlus class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{t.addAccountAction}</span>
			</button>
		</div>
	</div>

	{#if totalCount === 0}
		<div class="rounded-lg border border-separator bg-surface p-8 text-center">
			<p class="font-display text-sm font-semibold text-foreground">{t.noAccountsTitle}</p>
			<p class="mt-1 text-xs text-secondary">{t.noAccountsHelp}</p>
			<button
				type="button"
				onclick={onAddAccount}
				class="mt-4 inline-flex min-h-[40px] items-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap"
			>
				<MailPlus class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{t.configureFirstAccount}</span>
			</button>
		</div>
	{:else if demoMode}
		<div class="flex flex-col gap-3">
			{#each demoAccounts as account (account.account_id)}
				<WdgDemoAccountCard
					{account}
					expanded={Boolean(expandedIds[account.account_id])}
					authLabel={authLabel(account.auth_method)}
					onToggleFolders={() => toggleFolders(account.account_id, false)}
					onUpdateAvatar={onUpdateDemoAvatar}
					onRemove={onRemoveDemoAccount}
				/>
			{/each}
		</div>
	{:else}
		<div class="flex flex-col gap-3">
			{#each accounts as account (account.account_id)}
				<WdgNativeAccountCard
					{account}
					authLabel={authLabel(account.auth_method)}
					expanded={Boolean(expandedIds[account.account_id])}
					catalog={folderCatalogs[account.account_id]}
					{busy}
					{authorizing}
					{reauthorizingAccountId}
					reauthOpen={reauthTargetId === account.account_id}
					reauthSuccess={reauthSuccessAccountId === account.account_id}
					reauthFailed={reauthFailedAccountId === account.account_id}
					accountDiag={accountDiagnostics[account.account_id]}
					subscription={subscriptions[account.account_id]}
					{badgeError}
					aliases={aliases[account.account_id] ?? []}
					{aliasError}
					{diagnostic}
					{error}
					{avatarTransport}
					{onSetBadge}
					{onSetAliases}
					onToggleFolders={() => toggleFolders(account.account_id, true)}
					onRetryFolders={() => loadAccountFolders(account.account_id)}
					onToggleReauthorize={() => {
						if (reauthTargetId === account.account_id) {
							reauthTargetId = null;
						} else {
							reauthSuccessAccountId = null;
							reauthFailedAccountId = null;
							reauthTargetId = account.account_id;
						}
					}}
					onCloseReauthorize={() => (reauthTargetId = null)}
					onReauthSuccess={() => {
						reauthTargetId = null;
						reauthSuccessAccountId = account.account_id;
					}}
					onReauthFailure={() => (reauthFailedAccountId = account.account_id)}
					{onReauthorize}
					{onCancelReauthorize}
				/>
			{/each}
		</div>
	{/if}
</section>
