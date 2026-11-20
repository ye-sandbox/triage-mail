<script lang="ts">
	import {
		AlertTriangle,
		CheckCircle2,
		ChevronDown,
		ChevronRight,
		Clock,
		FolderOpen,
		ImagePlus,
		KeyRound,
		MailPlus,
		RefreshCw,
		Trash2,
		X
	} from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { labels as backupLabels } from '../backup/labels';
	import { i18n } from '../i18n/index.svelte';
	import { nativeTransport } from '../inbox/transport';
	import type { ConnectionDiagnosticCode, FolderDto } from '../inbox/types';
	import type { AvatarTransport } from './avatar-types';
	import { clearCredentialInputs } from './forms';
	import type { DemoAccountHealth, DemoConnectedAccountDto } from './onboarding.fixtures';
	import { oauthDefaultsForHost } from './presets';
	import type { AccountConfig, Reauthorization } from './types';
	import WdgAccountAvatar from './WdgAccountAvatar.svelte';
	import WdgProviderDiagnosticHint from './WdgProviderDiagnosticHint.svelte';
	import WdgAccountPlanBadge from './WdgAccountPlanBadge.svelte';
	import WdgAccountPlanControl from './WdgAccountPlanControl.svelte';
	import type { AccountSubscription, BadgeOverride } from './subscriptions';

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

	type FolderCatalogState = {
		loading: boolean;
		error: boolean;
		folders: FolderDto[];
		loaded: boolean;
	};

	let expandedIds = $state<Record<string, boolean>>({});
	let folderCatalogs = $state<Record<string, FolderCatalogState>>({});
	let reauthTargetId = $state<string | null>(null);
	let reauthAuthorize = $state('');
	let reauthToken = $state('');
	let reauthScopes = $state('');
	let reauthSuccessAccountId = $state<string | null>(null);
	let reauthFailedAccountId = $state<string | null>(null);
	let reauthForm = $state<HTMLFormElement>();
	const demoFileInputs = new Map<string, HTMLInputElement>();
	let active = true;

	const INPUT_CLS =
		'h-9 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-60';

	function registerDemoInput(node: HTMLInputElement, accountId: string) {
		demoFileInputs.set(accountId, node);
		return {
			update(nextId: string) {
				if (nextId !== accountId) {
					if (demoFileInputs.get(accountId) === node) {
						demoFileInputs.delete(accountId);
					}
					accountId = nextId;
					demoFileInputs.set(accountId, node);
				}
			},
			destroy() {
				if (demoFileInputs.get(accountId) === node) {
					demoFileInputs.delete(accountId);
				}
			}
		};
	}

	const t = $derived(i18n.t.onboarding);
	const formT = $derived(i18n.t.accountForm);
	const bLabels = $derived(backupLabels[i18n.locale]);
	const totalCount = $derived(demoMode ? demoAccounts.length : accounts.length);

	$effect(() => {
		active = true;
		return () => {
			active = false;
			clearCredentialInputs(reauthForm);
		};
	});

	function isOAuthAccount(method: AccountConfig['auth_method']): boolean {
		return method === 'OAuth2PublicPkce' || method === 'OAuth2ByoCredentials';
	}

	function closeReauthorize() {
		clearCredentialInputs(reauthForm);
		reauthTargetId = null;
	}

	function toggleReauthorize(account: AccountConfig) {
		if (reauthTargetId === account.account_id) {
			closeReauthorize();
			return;
		}
		clearCredentialInputs(reauthForm);
		reauthSuccessAccountId = null;
		reauthFailedAccountId = null;
		const defaults = oauthDefaultsForHost(account.imap_host);
		reauthAuthorize = defaults?.authEndpoint ?? '';
		reauthToken = defaults?.tokenEndpoint ?? '';
		reauthScopes = defaults?.scopes ?? '';
		reauthTargetId = account.account_id;
	}

	async function submitReauthorize(event: SubmitEvent, account: AccountConfig) {
		event.preventDefault();
		if (busy || !onReauthorize || !reauthForm) return;
		const data = new FormData(reauthForm);
		const value = (key: string) => String(data.get(key) ?? '').trim();
		const tokenEndpoint = value('token');
		const clientId = value('client');
		const clientSecret =
			account.auth_method === 'OAuth2ByoCredentials' ? String(data.get('secret') ?? '').trim() : '';
		let payload: Reauthorization = {
			account_id: account.account_id,
			authorization_endpoint: value('authorize'),
			...(tokenEndpoint ? { token_endpoint: tokenEndpoint } : {}),
			scopes: value('scopes'),
			...(clientId ? { client_id: clientId } : {}),
			...(clientSecret ? { client_secret: clientSecret } : {})
		};
		reauthSuccessAccountId = null;
		reauthFailedAccountId = null;
		const pending = onReauthorize(payload);
		payload = { account_id: '', authorization_endpoint: '', scopes: '' };
		for (const key of ['secret', 'client']) data.delete(key);
		clearCredentialInputs(reauthForm);
		const ok = await pending;
		if (!active) return;
		if (ok) {
			reauthTargetId = null;
			reauthSuccessAccountId = account.account_id;
			toast.success(t.reauthorizeSuccess);
		} else {
			reauthFailedAccountId = account.account_id;
		}
	}

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

	function initialsFor(email: string): string {
		const local = email.split('@')[0] ?? 'AC';
		const parts = local.split(/[._-]/).filter(Boolean);
		return parts.length >= 2
			? `${parts[0][0]}${parts[1][0]}`.toUpperCase()
			: local.slice(0, 2).toUpperCase();
	}

	function healthClass(health: DemoAccountHealth): string {
		if (health === 'Healthy') return 'text-status-healthy';
		if (health === 'Syncing') return 'text-status-info';
		if (health === 'Idle') return 'text-secondary';
		return 'text-status-warning';
	}

	function handleDemoAvatarSelect(accountId: string, event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file || !file.type.startsWith('image/')) return;
		const reader = new FileReader();
		reader.onload = () => {
			if (typeof reader.result === 'string') {
				onUpdateDemoAvatar?.(accountId, reader.result);
				toast.success(`Updated avatar for ${accountId}`);
			}
		};
		reader.readAsDataURL(file);
	}
</script>

<section aria-label={t.connectedAccounts} class="mx-auto flex w-full max-w-4xl flex-col gap-4">
	<div class="flex flex-wrap items-center justify-between gap-3 border-b border-separator pb-3">
		<div>
			<h2 class="font-display text-base font-semibold text-foreground">{t.connectedAccounts}</h2>
			<p class="mt-0.5 text-xs text-secondary">{t.accountsTitle}</p>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			{#if totalCount > 0 && onOpenInbox}
				<button
					type="button"
					disabled={busy}
					onclick={onOpenInbox}
					class="flex min-h-[40px] items-center gap-1.5 rounded-md border border-separator bg-surface px-3.5 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
				>
					<FolderOpen class="h-3.5 w-3.5 text-secondary" />
					<span>{t.openInbox}</span>
				</button>
			{/if}
			<button
				type="button"
				disabled={busy}
				onclick={onAddAccount}
				class="flex min-h-[40px] items-center gap-1.5 rounded-md bg-accent px-3.5 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap shrink-0 hover:opacity-95 active:scale-[0.98] disabled:opacity-50"
			>
				<MailPlus class="h-3.5 w-3.5" />
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
				class="mt-4 inline-flex min-h-[40px] items-center gap-1.5 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap"
			>
				<MailPlus class="h-3.5 w-3.5" />
				<span>{t.configureFirstAccount}</span>
			</button>
		</div>
	{:else if demoMode}
		<div class="flex flex-col gap-3">
			{#each demoAccounts as account (account.account_id)}
				{@const expanded = Boolean(expandedIds[account.account_id])}
				<article class="rounded-lg border border-separator bg-surface">
					<div class="flex flex-wrap items-center justify-between gap-4 p-4">
						<div class="flex min-w-0 items-center gap-3.5">
							<div
								class="relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-md border border-separator bg-field font-display text-xs font-semibold text-foreground"
							>
								{#if account.avatar_url}
									<img
										src={account.avatar_url}
										alt={`Avatar for ${account.email_display}`}
										referrerpolicy="no-referrer"
										class="h-full w-full object-cover"
									/>
								{:else}
									<span>{initialsFor(account.email_display)}</span>
								{/if}
							</div>
							<div class="min-w-0 space-y-1">
								<div class="flex flex-wrap items-center gap-2">
									<span class="truncate text-sm font-semibold text-foreground"
										>{account.email_display}</span
									>
									<span
										class="rounded border border-separator bg-field px-1.5 py-0.5 font-mono text-[11px] font-medium text-foreground"
										>{authLabel(account.auth_method)}</span
									>
								</div>
								<div class="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-secondary">
									<span class="font-mono text-foreground">{account.account_id}</span>
									<span aria-hidden="true">·</span>
									{#if account.auth_method === 'LocalArchive'}
										<span>{formT.localArchiveBadge}</span>
									{:else}
										<span class="font-mono">{account.imap_host}:{account.imap_port}</span>
										<span aria-hidden="true">·</span>
										<span>{account.tls_mode}</span>
									{/if}
								</div>
							</div>
						</div>

						<div class="flex flex-wrap items-center gap-2">
							<span
								class="flex items-center gap-1.5 pr-1 text-xs font-medium whitespace-nowrap {healthClass(
									account.health
								)}"
							>
								{#if account.health === 'Healthy'}
									<CheckCircle2 class="h-3.5 w-3.5 shrink-0" />
								{:else if account.health === 'Syncing'}
									<RefreshCw class="h-3.5 w-3.5 shrink-0 animate-spin" />
								{:else if account.health === 'Idle'}
									<Clock class="h-3.5 w-3.5 shrink-0" />
								{:else}
									<AlertTriangle class="h-3.5 w-3.5 shrink-0" />
								{/if}
								<span>{account.health}</span>
							</span>

							<input
								use:registerDemoInput={account.account_id}
								type="file"
								accept="image/*"
								onchange={(e) => handleDemoAvatarSelect(account.account_id, e)}
								class="hidden"
								aria-label={`Upload avatar for ${account.email_display}`}
							/>
							<button
								type="button"
								onclick={() => demoFileInputs.get(account.account_id)?.click()}
								class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-field px-2.5 py-1 text-xs text-foreground whitespace-nowrap hover:bg-selection-soft"
							>
								<ImagePlus class="h-3.5 w-3.5 text-secondary" />
								<span>{account.avatar_url ? i18n.t.accountPhoto.replace : i18n.t.accountPhoto.choose}</span>
							</button>
							{#if account.avatar_url}
								<button
									type="button"
									onclick={() => onUpdateDemoAvatar?.(account.account_id, null)}
									class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs text-secondary whitespace-nowrap hover:text-status-danger"
								>
									<Trash2 class="h-3.5 w-3.5" />
									<span>{i18n.t.accountPhoto.remove}</span>
								</button>
							{/if}

							<button
								type="button"
								onclick={() => toggleFolders(account.account_id, false)}
								aria-expanded={expanded}
								class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field"
							>
								{#if expanded}
									<ChevronDown class="h-3.5 w-3.5 text-secondary" />
								{:else}
									<ChevronRight class="h-3.5 w-3.5 text-secondary" />
								{/if}
								<span
									>{t.foldersCount} (<span class="font-mono tabular-nums"
										>{account.folders.length}</span
									>)</span
								>
							</button>

							{#if onRemoveDemoAccount}
								<button
									type="button"
									onclick={() => onRemoveDemoAccount?.(account.account_id)}
									aria-label={`Disconnect ${account.email_display}`}
									class="flex min-h-[36px] min-w-[36px] items-center justify-center rounded-md border border-separator bg-surface text-secondary hover:text-status-danger"
								>
									<Trash2 class="h-3.5 w-3.5" />
								</button>
							{/if}
						</div>
					</div>

					{#if expanded}
						<div class="overflow-x-auto border-t border-separator bg-toolbar/50 px-4 py-3">
							<table class="w-full text-left text-xs">
								<thead>
									<tr class="border-b border-separator text-[11px] font-medium text-secondary">
										<th class="py-1.5 pr-4">{t.folderColumn}</th>
										<th class="py-1.5 pr-4">{t.roleColumn}</th>
										<th class="py-1.5 pr-4 text-right">UIDVALIDITY</th>
										<th class="py-1.5 pr-4 text-right">{t.totalColumn}</th>
										<th class="py-1.5 text-right">{t.unreadColumn}</th>
									</tr>
								</thead>
								<tbody class="divide-y divide-separator">
									{#each account.folders as folder (folder.id)}
										<tr class="h-9 hover:bg-field/60">
											<td class="py-1.5 pr-4 font-medium text-foreground">{folder.name}</td>
											<td class="py-1.5 pr-4 font-mono text-[11px] text-secondary">{folder.role}</td>
											<td class="py-1.5 pr-4 text-right font-mono tabular-nums text-secondary"
												>{folder.uid_validity}</td
											>
											<td class="py-1.5 pr-4 text-right font-mono tabular-nums text-foreground"
												>{folder.total_messages.toLocaleString()}</td
											>
											<td
												class="py-1.5 text-right font-mono tabular-nums {folder.unread_messages > 0
													? 'font-semibold text-accent'
													: 'text-secondary'}">{folder.unread_messages.toLocaleString()}</td
											>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</article>
			{/each}
		</div>
	{:else}
		<div class="flex flex-col gap-3">
			{#each accounts as account (account.account_id)}
				{@const expanded = Boolean(expandedIds[account.account_id])}
				{@const catalog = folderCatalogs[account.account_id]}
				{@const accountDiag = accountDiagnostics[account.account_id]}
				{@const subscription = subscriptions[account.account_id]}
				{@const reauthOpen = reauthTargetId === account.account_id}
				<article class="rounded-lg border border-separator bg-surface">
					<div class="flex flex-wrap items-center justify-between gap-4 p-4">
						<div class="flex min-w-0 flex-1 items-center gap-3.5">
							<div class="min-w-0 space-y-1">
								<div class="flex flex-wrap items-center gap-2">
									<span class="truncate text-sm font-semibold text-foreground"
										>{account.email_display}</span
									>
									<span
										class="rounded border border-separator bg-field px-1.5 py-0.5 font-mono text-[11px] font-medium text-foreground"
										>{authLabel(account.auth_method)}</span
									>
									<WdgAccountPlanBadge
										label={subscription?.effective_badge ?? null}
										quota={subscription?.quota}
									/>
								</div>
								<div class="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-secondary">
									<span class="font-mono text-foreground">{account.account_id}</span>
									<span aria-hidden="true">·</span>
									{#if account.auth_method === 'LocalArchive'}
										<span>{formT.localArchiveBadge}</span>
									{:else if !account.imap_host}
										<span>{bLabels.disconnected}</span>
									{:else}
										<span class="font-mono tabular-nums"
											>{account.imap_host}:{account.imap_port}</span
										>
									{/if}
								</div>
								<div class="pt-1">
									<WdgAccountAvatar
										accountId={account.account_id}
										label={account.email_display}
										transport={avatarTransport}
									/>
								</div>
							</div>
						</div>

						<div class="flex flex-wrap items-center gap-2">
							{#if isOAuthAccount(account.auth_method) && onReauthorize}
								<button
									type="button"
									disabled={busy}
									aria-expanded={reauthOpen}
									onclick={() => toggleReauthorize(account)}
									class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
								>
									<KeyRound class="h-3.5 w-3.5 text-secondary" />
									<span>
										{reauthorizingAccountId === account.account_id ? t.reauthorizing : t.reauthorize}
									</span>
								</button>
							{/if}

							<button
								type="button"
								onclick={() => toggleFolders(account.account_id, true)}
								aria-expanded={expanded}
								class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field"
							>
								{#if expanded}
									<ChevronDown class="h-3.5 w-3.5 text-secondary" />
								{:else}
									<ChevronRight class="h-3.5 w-3.5 text-secondary" />
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

					{#if accountDiag}
						{@const guidance = formT.diagnostics[accountDiag]}
						<div
							role="alert"
							class="flex items-start gap-2.5 border-t border-separator bg-field/80 px-4 py-3 text-xs"
						>
							<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-status-warning" />
							<div class="space-y-1">
								<p class="font-semibold text-foreground">
									{t.accountDiagnosticBanner}: {guidance.title}
								</p>
								<p class="leading-relaxed text-secondary">{guidance.hint}</p>
								<WdgProviderDiagnosticHint code={accountDiag} host={account.imap_host} method={account.auth_method} />
							</div>
						</div>
					{/if}

					{#if reauthSuccessAccountId === account.account_id}
						<div
							role="status"
							class="flex items-center gap-2 border-t border-separator bg-field/70 px-4 py-2.5 text-xs font-medium text-status-healthy"
						>
							<CheckCircle2 class="h-3.5 w-3.5 shrink-0" />
							<span>{t.reauthorizeSuccess}</span>
						</div>
					{/if}

					{#if reauthOpen}
						<form
							bind:this={reauthForm}
							onsubmit={(e) => void submitReauthorize(e, account)}
							autocomplete="off"
							class="space-y-3 border-t border-separator bg-field/50 px-4 py-3"
						>
							<p class="text-xs text-secondary">{t.reauthorizeHelp}</p>
							<fieldset disabled={busy} class="space-y-3">
								<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
									<label class="flex flex-col gap-1">
										<span class="text-xs font-medium text-secondary">{formT.authEndpointLabel}</span>
										<input
											name="authorize"
											type="url"
											required
											pattern="https://.*"
											maxlength="2048"
											bind:value={reauthAuthorize}
											class="{INPUT_CLS} font-mono"
										/>
									</label>
									<label class="flex flex-col gap-1">
										<span class="text-xs font-medium text-secondary">{formT.tokenEndpointLabel}</span>
										<input
											name="token"
											type="url"
											pattern="https://.*"
											maxlength="2048"
											bind:value={reauthToken}
											class="{INPUT_CLS} font-mono"
										/>
									</label>
									<label class="flex flex-col gap-1 sm:col-span-2">
										<span class="text-xs font-medium text-secondary">{formT.scopesLabel}</span>
										<input
											name="scopes"
											type="text"
											required
											maxlength="4096"
											bind:value={reauthScopes}
											class="{INPUT_CLS} font-mono"
										/>
									</label>
								</div>

								<p class="text-[11px] text-secondary">{t.reauthorizeClientOverrideHint}</p>

								<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
									<label
										class="flex flex-col gap-1 {account.auth_method === 'OAuth2PublicPkce'
											? 'sm:col-span-2'
											: ''}"
									>
										<span class="text-xs font-medium text-secondary">{formT.clientIdLabel}</span>
										<input
											name="client"
											type="text"
											data-credential
											maxlength="4096"
											autocomplete="off"
											class="{INPUT_CLS} font-mono"
										/>
									</label>
									{#if account.auth_method === 'OAuth2ByoCredentials'}
										<label class="flex flex-col gap-1">
											<span class="text-xs font-medium text-secondary"
												>{formT.clientSecretLabel}</span
											>
											<input
												name="secret"
												type="password"
												data-credential
												maxlength="8192"
												autocomplete="off"
												class="{INPUT_CLS} font-mono"
											/>
										</label>
									{/if}
								</div>

								<div class="flex flex-wrap items-center justify-end gap-2">
									<button
										type="button"
										onclick={closeReauthorize}
										class="flex min-h-[34px] items-center rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-field"
									>
										{t.reauthorizeCancel}
									</button>
									<button
										type="submit"
										disabled={busy}
										class="flex min-h-[34px] items-center gap-1.5 rounded-md bg-accent px-3.5 py-1 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-60"
									>
										<KeyRound class="h-3.5 w-3.5" />
										<span>
											{reauthorizingAccountId === account.account_id
												? t.reauthorizing
												: t.reauthorizeSubmit}
										</span>
									</button>
								</div>
							</fieldset>

							{#if authorizing && reauthorizingAccountId === account.account_id}
								<div
									role="status"
									class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-separator bg-surface px-3 py-2 text-xs"
								>
									<span class="text-foreground">{formT.browserAuthPending}</span>
									{#if onCancelReauthorize}
										<button
											type="button"
											onclick={() => void onCancelReauthorize?.()}
											class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-field px-2.5 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
										>
											<X class="h-3.5 w-3.5" />
											<span>{formT.cancelAuth}</span>
										</button>
									{/if}
								</div>
							{/if}

							{#if reauthFailedAccountId === account.account_id}
								{#if diagnostic}
									{@const guidance = formT.diagnostics[diagnostic]}
									<div
										role="alert"
										class="flex items-start gap-2.5 rounded-md border border-status-warning/50 bg-surface px-3 py-2.5 text-xs"
									>
										<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-status-warning" />
										<div class="space-y-1">
											<p class="font-semibold text-foreground">{guidance.title}</p>
											<p class="leading-relaxed text-secondary">{guidance.hint}</p>
										</div>
									</div>
								{:else if error}
									<p
										role="alert"
										class="rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-status-danger"
									>
										{error}
									</p>
								{/if}
							{/if}
						</form>
					{/if}

					{#if expanded}
						<div class="overflow-x-auto border-t border-separator bg-toolbar/50 px-4 py-3">
							{#if catalog?.loading}
								<p role="status" class="text-xs text-secondary">{t.loadingFolders}</p>
							{:else if catalog?.error}
								<div class="flex items-center justify-between gap-2 text-xs text-status-warning">
									<span>{t.foldersUnavailable}</span>
									<button
										type="button"
										onclick={() => void loadAccountFolders(account.account_id)}
										class="rounded border border-separator bg-surface px-2 py-1 text-foreground hover:bg-field"
									>
										{i18n.t.mailboxes.retry}
									</button>
								</div>
							{:else if !catalog?.folders.length}
								<p class="text-xs text-secondary">{t.noFolders}</p>
							{:else}
								<table class="w-full text-left text-xs">
									<thead>
										<tr class="border-b border-separator text-[11px] font-medium text-secondary">
											<th class="py-1.5 pr-4">{t.folderColumn}</th>
											<th class="py-1.5 pr-4">{t.roleColumn}</th>
											<th class="py-1.5 pr-4 text-right">{t.totalColumn}</th>
											<th class="py-1.5 text-right">{t.unreadColumn}</th>
										</tr>
									</thead>
									<tbody class="divide-y divide-separator">
										{#each catalog.folders as folder (folder.folder_name)}
											<tr class="h-9 hover:bg-field/60">
												<td class="py-1.5 pr-4 font-medium text-foreground"
													>{folder.display_name || folder.folder_name}</td
												>
												<td class="py-1.5 pr-4 font-mono text-[11px] text-secondary"
													>{folder.role}</td
												>
												<td class="py-1.5 pr-4 text-right font-mono tabular-nums text-foreground"
													>{folder.total_messages ?? '—'}</td
												>
												<td
													class="py-1.5 text-right font-mono tabular-nums {(folder.unread_messages ?? 0) >
													0
														? 'font-semibold text-accent'
														: 'text-secondary'}">{folder.unread_messages ?? '—'}</td
												>
											</tr>
										{/each}
									</tbody>
								</table>
							{/if}
						</div>
					{/if}
				</article>
			{/each}
		</div>
	{/if}
</section>
