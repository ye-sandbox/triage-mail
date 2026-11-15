<script lang="ts">
	import { goto } from '$app/navigation';
	import { KeyRound, Lock, ShieldCheck, Unlock } from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { FolderDto } from '../inbox/types';
	import { appStore } from '../stores/app.svelte';
	import type { AvatarTransport } from './avatar-types';
	import type { DiscoveryTransport } from './discovery';
	import {
		ONBOARDING_ACCOUNTS_FIXTURES,
		type DemoConnectedAccountDto,
		type OnboardingPhase
	} from './onboarding.fixtures';
	import { onboardingSession, type OnboardingSession } from './session.svelte';
	import WdgAccountDiscoveryForm from './WdgAccountDiscoveryForm.svelte';
	import WdgConnectedAccountList from './WdgConnectedAccountList.svelte';
	import WdgVaultUnlockCard from './WdgVaultUnlockCard.svelte';

	interface Props {
		session?: OnboardingSession;
		discoveryTransport?: DiscoveryTransport;
		avatarTransport?: AvatarTransport;
		listFolders?: (accountId: string) => Promise<FolderDto[]>;
		onOpenInbox?: () => void;
	}

	let {
		session = onboardingSession,
		discoveryTransport,
		avatarTransport,
		listFolders,
		onOpenInbox
	}: Props = $props();

	const t = $derived(i18n.t.onboarding);
	const isNative = $derived(session.isNative);

	// Demo-only state (shown strictly outside Tauri)
	const DEMO_PHASES: { id: OnboardingPhase; label: string }[] = [
		{ id: 'vault-locked', label: '1. Vault Locked Gate' },
		{ id: 'discovery', label: '2. Discovery & Auth' },
		{ id: 'directory', label: '3. Connected Accounts' }
	];
	let demoPhase = $state<OnboardingPhase>('vault-locked');
	let demoAccounts = $state<DemoConnectedAccountDto[]>(structuredClone(ONBOARDING_ACCOUNTS_FIXTURES));

	// Native state
	let showNativeDiscovery = $state(false);

	$effect(() => {
		if (!session.booted && !session.booting) {
			void session.boot();
		}
	});

	$effect(() => {
		if (isNative) {
			session.syncShellState();
		}
	});

	function handleOpenInbox() {
		if (onOpenInbox) {
			onOpenInbox();
			return;
		}
		void goto('/').catch(() => {});
	}

	function selectDemoPhase(next: OnboardingPhase) {
		demoPhase = next;
		appStore.vaultUnlocked = next !== 'vault-locked';
	}

	function handleDemoUnlock(method: 'passphrase' | 'keyring' | 'backup', detail?: string) {
		appStore.vaultUnlocked = true;
		if (method === 'passphrase') {
			toast.success('SQLCipher vault unlocked via Argon2id + AES-GCM');
			demoPhase = 'discovery';
		} else if (method === 'keyring') {
			toast.success('OS Secret Service / Keychain unlocked local vault');
			demoPhase = 'directory';
		} else {
			toast.success(`Restored vault metadata from ${detail ?? 'backup'}`);
			demoPhase = 'directory';
		}
	}

	function handleDemoAccountAdded(account: DemoConnectedAccountDto) {
		demoAccounts = [account, ...demoAccounts];
		appStore.addAccount({
			account_id: account.account_id,
			display_name: account.email_display.split('@')[0] ?? account.email_display,
			address: account.email_display,
			imap_host: account.imap_host,
			imap_port: account.imap_port,
			tls_mode: account.tls_mode === 'STARTTLS' ? 'STARTTLS' : 'IMAPS',
			auth_method: account.auth_method === 'AppPassword' ? 'AppPassword-Vault' : 'OAuth2-PKCE',
			vault_locked: false,
			folders: [
				{ id: 'INBOX', label: 'INBOX' },
				{ id: 'Archive', label: 'Archive' },
				{ id: 'Trash', label: 'Trash' }
			]
		});
		toast.success(`Connected ${account.email_display} (${account.account_id})`);
		demoPhase = 'directory';
	}

	function handleDemoUpdateAvatar(accountId: string, avatarUrl: string | null) {
		demoAccounts = demoAccounts.map((acct) =>
			acct.account_id === accountId ? { ...acct, avatar_url: avatarUrl } : acct
		);
	}

	function handleDemoRemoveAccount(accountId: string) {
		const target = demoAccounts.find((a) => a.account_id === accountId);
		demoAccounts = demoAccounts.filter((a) => a.account_id !== accountId);
		if (target) {
			toast.info(`Disconnected ${target.email_display} (${accountId})`);
		}
	}

	const nativeUnlocked = $derived(
		isNative && !session.recovery && session.accounts.phase === 'ready'
	);
	const indexedCount = $derived(
		isNative ? session.accounts.accounts.length : demoAccounts.length
	);
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto bg-background text-foreground">
	<div class="border-b border-separator bg-toolbar px-4 py-3 md:px-6">
		<div class="mx-auto flex w-full max-w-4xl flex-wrap items-center justify-between gap-3">
			<div>
				<h1 class="font-display text-lg font-semibold text-foreground">{t.pageTitle}</h1>
				<div class="mt-0.5 flex flex-wrap items-center gap-2 text-xs text-secondary">
					<span>SQLCipher + OS Keyring</span>
					<span aria-hidden="true">·</span>
					<span class="font-mono tabular-nums">{indexedCount} {t.accountsIndexed}</span>
					<span aria-hidden="true">·</span>
					{#if isNative}
						<span
							class="inline-flex items-center gap-1 font-medium {nativeUnlocked
								? 'text-status-healthy'
								: 'text-status-warning'}"
						>
							{#if nativeUnlocked}
								<ShieldCheck class="h-3.5 w-3.5" />
								<span>{t.vaultUnlockedBadge}</span>
							{:else}
								<Lock class="h-3.5 w-3.5" />
								<span>{session.recovery ? t.recoveryModeBadge : t.vaultLockedBadge}</span>
							{/if}
						</span>
					{:else}
						<span
							class="inline-flex items-center gap-1 font-medium {appStore.vaultUnlocked &&
							demoPhase !== 'vault-locked'
								? 'text-status-healthy'
								: 'text-status-warning'}"
						>
							{#if appStore.vaultUnlocked && demoPhase !== 'vault-locked'}
								<ShieldCheck class="h-3.5 w-3.5" />
								<span>{t.vaultUnlockedBadge}</span>
							{:else}
								<Lock class="h-3.5 w-3.5" />
								<span>{t.vaultLockedBadge}</span>
							{/if}
						</span>
					{/if}
				</div>
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<div
					role="radiogroup"
					aria-label={i18n.t.language.selectLabel}
					class="inline-flex items-center rounded-md border border-separator bg-field p-0.5"
				>
					<button
						type="button"
						role="radio"
						aria-checked={i18n.locale === 'en'}
						aria-label="English"
						onclick={() => i18n.setLocale('en')}
						class="min-h-[32px] rounded px-2.5 text-xs font-medium transition-colors {i18n.locale ===
						'en'
							? 'bg-surface text-foreground shadow-xs'
							: 'text-secondary hover:text-foreground'}"
					>
						EN
					</button>
					<button
						type="button"
						role="radio"
						aria-checked={i18n.locale === 'pt-BR'}
						aria-label="Português"
						onclick={() => i18n.setLocale('pt-BR')}
						class="min-h-[32px] rounded px-2.5 text-xs font-medium transition-colors {i18n.locale ===
						'pt-BR'
							? 'bg-surface text-foreground shadow-xs'
							: 'text-secondary hover:text-foreground'}"
					>
						PT
					</button>
				</div>

				{#if !isNative}
					<div
						role="tablist"
						aria-label="Onboarding lifecycle phases"
						class="flex items-center gap-1 rounded-md border border-separator bg-field p-1"
					>
						{#each DEMO_PHASES as item (item.id)}
							<button
								type="button"
								role="tab"
								aria-selected={demoPhase === item.id}
								onclick={() => selectDemoPhase(item.id)}
								class="min-h-[36px] rounded px-2.5 py-1 text-xs font-medium whitespace-nowrap shrink-0 transition-colors {demoPhase ===
								item.id
									? 'bg-surface text-foreground shadow-xs'
									: 'text-secondary hover:text-foreground'}"
							>
								{item.label}
							</button>
						{/each}
					</div>

					{#if demoPhase !== 'vault-locked'}
						<button
							type="button"
							onclick={() => selectDemoPhase('vault-locked')}
							class="flex min-h-[38px] items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-foreground whitespace-nowrap shrink-0 hover:bg-field"
						>
							<Lock class="h-3.5 w-3.5 text-secondary" />
							<span>Lock Vault</span>
						</button>
					{:else}
						<button
							type="button"
							onclick={() => selectDemoPhase('directory')}
							class="flex min-h-[38px] items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-foreground whitespace-nowrap shrink-0 hover:bg-field"
						>
							<Unlock class="h-3.5 w-3.5 text-secondary" />
							<span>Preview Directory</span>
						</button>
					{/if}
				{:else if nativeUnlocked && session.accounts.accounts.length > 0}
					<button
						type="button"
						onclick={handleOpenInbox}
						class="flex min-h-[38px] items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground whitespace-nowrap hover:opacity-95"
					>
						<span>{t.openInbox}</span>
					</button>
				{/if}
			</div>
		</div>
	</div>

	<div class="flex-1 px-4 py-6 md:px-6 md:py-8">
		{#if !isNative}
			{#if demoPhase === 'vault-locked'}
				<WdgVaultUnlockCard
					demoMode={true}
					vaultUnlocked={appStore.vaultUnlocked && demoPhase !== 'vault-locked'}
					onDemoUnlock={handleDemoUnlock}
				/>
			{:else if demoPhase === 'discovery'}
				<div class="flex flex-col gap-4">
					<WdgAccountDiscoveryForm
						controller={session.accounts}
						{discoveryTransport}
						demoMode={true}
						onAccountAdded={handleDemoAccountAdded}
					/>
					<div class="mx-auto flex w-full max-w-3xl items-center justify-between text-xs text-secondary">
						<span class="flex items-center gap-1.5">
							<KeyRound class="h-3.5 w-3.5" />
							<span>{t.credentialsZeroized}</span>
						</span>
						<button
							type="button"
							onclick={() => (demoPhase = 'directory')}
							class="font-medium text-accent whitespace-nowrap hover:underline"
						>
							{t.viewConnectedAccounts} ({demoAccounts.length})
						</button>
					</div>
				</div>
			{:else}
				<WdgConnectedAccountList
					demoMode={true}
					{demoAccounts}
					onAddAccount={() => (demoPhase = 'discovery')}
					onOpenInbox={handleOpenInbox}
					onUpdateDemoAvatar={handleDemoUpdateAvatar}
					onRemoveDemoAccount={handleDemoRemoveAccount}
				/>
			{/if}
		{:else if session.booting || (!session.recovery && session.accounts.phase === 'loading')}
			<div class="mx-auto max-w-lg rounded-lg border border-separator bg-surface p-6">
				<p role="status" class="text-sm text-secondary">{t.openingMail}</p>
			</div>
		{:else if session.recovery}
			<WdgVaultUnlockCard
				recovery={true}
				recoveryTransport={session.recoveryTransport}
				onSkipRecovery={() => void session.continueWithoutRecovery()}
				onRestored={() => void session.completeRecovery()}
			/>
		{:else if session.accounts.phase === 'locked'}
			<WdgVaultUnlockCard
				controller={session.accounts}
				vaultUnlocked={false}
				recoveryTransport={session.recoveryTransport}
			/>
		{:else if session.accounts.phase === 'error'}
			<div class="mx-auto max-w-lg space-y-4 rounded-lg border border-separator bg-surface p-6">
				{#if session.accounts.error}
					<p role="alert" class="text-sm font-medium text-status-danger">
						{session.accounts.error}
					</p>
				{/if}
				<button
					type="button"
					onclick={() => void session.boot(true)}
					class="flex min-h-[40px] items-center justify-center rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-foreground hover:bg-selection-soft"
				>
					{t.tryAgain}
				</button>
			</div>
		{:else if session.accounts.accounts.length === 0 || showNativeDiscovery}
			<div class="flex flex-col gap-4">
				<WdgAccountDiscoveryForm
					controller={session.accounts}
					{discoveryTransport}
					onSuccess={() => {
						session.syncShellState();
						showNativeDiscovery = false;
					}}
				/>
				<div class="mx-auto flex w-full max-w-3xl items-center justify-between text-xs text-secondary">
					<span class="flex items-center gap-1.5">
						<KeyRound class="h-3.5 w-3.5" />
						<span>{t.credentialsZeroized}</span>
					</span>
					{#if session.accounts.accounts.length > 0}
						<button
							type="button"
							onclick={() => (showNativeDiscovery = false)}
							class="font-medium text-accent whitespace-nowrap hover:underline"
						>
							{t.viewConnectedAccounts} ({session.accounts.accounts.length})
						</button>
					{/if}
				</div>
			</div>
		{:else}
			<WdgConnectedAccountList
				accounts={session.accounts.accounts}
				busy={session.accounts.busy}
				{avatarTransport}
				{listFolders}
				onAddAccount={() => (showNativeDiscovery = true)}
				onOpenInbox={handleOpenInbox}
			/>
		{/if}
	</div>
</div>
