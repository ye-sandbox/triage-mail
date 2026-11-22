<script lang="ts">
	import { goto } from '$app/navigation';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { FolderDto } from '../inbox/types';
	import { appStore } from '../stores/app.svelte';
	import type { AvatarTransport } from './avatar-types';
	import type { DiscoveryTransport } from './discovery';
	import { buildShellAccountFromDemo, notifyDemoUnlock } from './onboarding-demo';
	import {
		ONBOARDING_ACCOUNTS_FIXTURES,
		type DemoConnectedAccountDto,
		type OnboardingPhase
	} from './onboarding.fixtures';
	import { onboardingSession, type OnboardingSession } from './session.svelte';
	import WdgAccountDiscoveryForm from './WdgAccountDiscoveryForm.svelte';
	import WdgConnectedAccountList from './WdgConnectedAccountList.svelte';
	import WdgOnboardingHeader from './WdgOnboardingHeader.svelte';
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

	let demoPhase = $state<OnboardingPhase>('vault-locked');
	let demoAccounts = $state<DemoConnectedAccountDto[]>(structuredClone(ONBOARDING_ACCOUNTS_FIXTURES));
	let showNativeDiscovery = $state(false);

	$effect(() => {
		if (!session.booted && !session.booting) void session.boot();
	});

	$effect(() => {
		if (isNative) session.syncShellState();
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
		demoPhase = notifyDemoUnlock(method, detail);
	}

	function handleDemoAccountAdded(account: DemoConnectedAccountDto) {
		demoAccounts = [account, ...demoAccounts];
		appStore.addAccount(buildShellAccountFromDemo(account));
		toast.success(`Connected ${account.email_display} (${account.account_id})`);
		demoPhase = 'directory';
	}

	const nativeUnlocked = $derived(
		isNative && !session.recovery && session.accounts.phase === 'ready'
	);
	const indexedCount = $derived(
		isNative ? session.accounts.accounts.length : demoAccounts.length
	);
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto bg-background text-foreground">
	<WdgOnboardingHeader
		{isNative}
		{indexedCount}
		{nativeUnlocked}
		recovery={session.recovery}
		hasAccounts={session.accounts.accounts.length > 0}
		{demoPhase}
		onSelectDemoPhase={selectDemoPhase}
		onOpenInbox={handleOpenInbox}
	/>

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
						<span class="flex items-center gap-2">
							<KeyRound class="h-3.5 w-3.5" aria-hidden="true" />
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
					onUpdateDemoAvatar={(id, url) =>
						(demoAccounts = demoAccounts.map((a) =>
							a.account_id === id ? { ...a, avatar_url: url } : a
						))}
					onRemoveDemoAccount={(id) => {
						const target = demoAccounts.find((a) => a.account_id === id);
						demoAccounts = demoAccounts.filter((a) => a.account_id !== id);
						if (target) toast.info(`Disconnected ${target.email_display} (${id})`);
					}}
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
				vaultStatus={session.vaultStatus}
				recoveryTransport={session.recoveryTransport}
			/>
		{:else if session.accounts.phase === 'error'}
			<div class="mx-auto max-w-lg space-y-4 rounded-lg border border-separator bg-surface p-6">
				{#if session.accounts.error}
					<p role="alert" class="text-sm font-medium text-status-danger">{session.accounts.error}</p>
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
					<span class="flex items-center gap-2">
						<KeyRound class="h-3.5 w-3.5" aria-hidden="true" />
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
				authorizing={session.accounts.authorizing}
				reauthorizingAccountId={session.accounts.reauthorizingAccountId}
				accountDiagnostics={session.accounts.accountDiagnostics}
				subscriptions={session.accounts.subscriptions}
				badgeError={session.accounts.badgeError}
				onSetBadge={async (id, override) => {
					await session.accounts.setBadge(id, override);
					appStore.accountSubscriptions = { ...session.accounts.subscriptions };
				}}
				aliases={session.accounts.aliases}
				aliasError={session.accounts.aliasError}
				onSetAliases={!session.accounts.transport.setAliases
					? undefined
					: async (id, next) => {
							const ok = await session.accounts.setAliases(id, next);
							if (ok) appStore.accountAliases = { ...session.accounts.aliases };
							return ok;
						}}
				diagnostic={session.accounts.diagnostic}
				error={session.accounts.error}
				{avatarTransport}
				{listFolders}
				onAddAccount={() => (showNativeDiscovery = true)}
				onOpenInbox={handleOpenInbox}
				onReauthorize={(input) => session.accounts.reauthorize(input)}
				onCancelReauthorize={() => session.accounts.cancel()}
			/>
		{/if}
	</div>
</div>
