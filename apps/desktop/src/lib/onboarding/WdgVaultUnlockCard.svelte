<script lang="ts">
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import Lock from '@lucide/svelte/icons/lock';
	import { transfer } from '../backup/transport';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountController } from './controller.svelte';
	import type { RecoveryTransport, VaultStatus } from './types';
	import WdgVaultPassphraseForm from './WdgVaultPassphraseForm.svelte';
	import WdgVaultRecoveryForm from './WdgVaultRecoveryForm.svelte';

	interface Props {
		controller?: AccountController;
		vaultUnlocked?: boolean;
		vaultStatus?: VaultStatus | null;
		recovery?: boolean;
		demoMode?: boolean;
		recoveryTransport?: RecoveryTransport;
		onSkipRecovery?: () => void;
		onRestored?: () => void;
		onDemoUnlock?: (method: 'passphrase' | 'keyring' | 'backup', detail?: string) => void;
	}

	let {
		controller,
		vaultUnlocked = false,
		vaultStatus = null,
		recovery = false,
		demoMode = false,
		recoveryTransport = transfer,
		onSkipRecovery,
		onRestored,
		onDemoUnlock
	}: Props = $props();

	const t = $derived(i18n.t.onboarding);
	const isFirstRun = $derived(!recovery && vaultStatus?.exists === false);
	const keyringUnavailable = $derived(vaultStatus?.keyring_available === false);
</script>

<div class="mx-auto w-full max-w-lg py-4">
	<section
		aria-labelledby="vault-gate-title"
		class="rounded-lg border border-separator bg-surface p-6"
	>
		<div class="flex items-start justify-between gap-4 border-b border-separator pb-4">
			<div class="flex items-start gap-3">
				<div
					class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-separator bg-field text-status-warning"
				>
					{#if recovery}
						<FileArchive class="h-5 w-5" aria-hidden="true" />
					{:else}
						<Lock class="h-5 w-5" aria-hidden="true" />
					{/if}
				</div>
				<div>
					<h2 id="vault-gate-title" class="font-display text-base font-semibold text-foreground">
						{recovery ? t.restoreBackupTitle : isFirstRun ? t.createVaultTitle : t.unlockTitle}
					</h2>
					<p class="mt-1 text-xs text-secondary">
						Argon2id + AES-GCM Fallback · Local SQLCipher Storage
					</p>
				</div>
			</div>
			<span class="shrink-0 whitespace-nowrap font-mono text-[11px] font-medium text-status-warning">
				{recovery
					? t.recoveryModeBadge
					: isFirstRun
						? t.createVaultBadge
						: vaultUnlocked
							? t.vaultUnlockedBadge
							: t.vaultLockedBadge}
			</span>
		</div>

		{#if recovery}
			<WdgVaultRecoveryForm {recoveryTransport} {onSkipRecovery} {onRestored} />
		{:else}
			<WdgVaultPassphraseForm
				{controller}
				{isFirstRun}
				{keyringUnavailable}
				{demoMode}
				{onDemoUnlock}
			/>
		{/if}
	</section>
</div>
