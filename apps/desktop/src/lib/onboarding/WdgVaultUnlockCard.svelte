<script lang="ts">
	import { Eye, EyeOff, FileArchive, KeyRound, Lock, RefreshCw, ShieldAlert } from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { transfer } from '../backup/transport';
	import { i18n } from '../i18n/index.svelte';
	import { SettingsFormGuard } from '../settings/guards';
	import type { AccountController } from './controller.svelte';
	import { clearCredentialInputs } from './forms';
	import type { RecoveryTransport } from './types';

	interface Props {
		controller?: AccountController;
		vaultUnlocked?: boolean;
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
		recovery = false,
		demoMode = false,
		recoveryTransport = transfer,
		onSkipRecovery,
		onRestored,
		onDemoUnlock
	}: Props = $props();

	let unlockForm = $state<HTMLFormElement>();
	let recoveryForm = $state<HTMLFormElement>();
	let backupInputEl = $state<HTMLInputElement>();
	let showPassphrase = $state(false);
	let demoBusy = $state(false);
	let recoveryBusy = $state(false);
	let recoveryConsent = $state(false);
	let recoveryFinished = $state(false);
	let recoveryMessage = $state('');
	let localError = $state('');
	let selectedBackupMeta = $state<{ name: string; bytes: number } | null>(null);
	const guard = new SettingsFormGuard();
	let active = true;

	const t = $derived(i18n.t.onboarding);
	const busy = $derived(demoMode ? demoBusy : Boolean(controller?.busy) || recoveryBusy);
	const displayError = $derived(localError || (!demoMode ? (controller?.error ?? '') : ''));

	$effect(() => {
		active = true;
		const currentUnlock = unlockForm;
		const currentRecovery = recoveryForm;
		return () => {
			active = false;
			clearCredentialInputs(currentUnlock);
			clearCredentialInputs(currentRecovery);
			guard.reset();
		};
	});

	async function handlePassphraseSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !unlockForm) return;
		const input = unlockForm.elements.namedItem('passphrase') as HTMLInputElement | null;
		const value = input?.value ?? '';
		clearCredentialInputs(unlockForm);

		if (demoMode) {
			if (value.trim().length < 4) {
				localError = t.vaultUnlockError;
				return;
			}
			localError = '';
			demoBusy = true;
			setTimeout(() => {
				if (!active) return;
				demoBusy = false;
				onDemoUnlock?.('passphrase');
			}, 180);
			return;
		}

		if (!controller || !value) return;
		localError = '';
		await controller.unlock(value);
	}

	async function handleRetryKeyring() {
		if (busy) return;
		localError = '';
		if (demoMode) {
			demoBusy = true;
			setTimeout(() => {
				if (!active) return;
				demoBusy = false;
				onDemoUnlock?.('keyring');
			}, 180);
			return;
		}
		await controller?.load(true);
	}

	async function handleRecoverySubmit(event: SubmitEvent) {
		event.preventDefault();
		if (!recoveryForm || recoveryBusy || guard.isBlocked() || !recoveryConsent || recoveryFinished)
			return;
		const secret = recoveryForm.elements.namedItem('passphrase') as HTMLInputElement | null;
		const confirm = recoveryForm.elements.namedItem('confirmation') as HTMLInputElement | null;
		const passphrase = secret?.value ?? '';
		const confirmation = confirm?.value ?? '';
		if (passphrase.length < 12 || passphrase !== confirmation) {
			clearCredentialInputs(recoveryForm);
			recoveryMessage = t.restorePassphraseMismatch;
			return;
		}

		recoveryBusy = true;
		guard.setBusy(true);
		recoveryMessage = '';
		try {
			const path = await recoveryTransport.choose(false);
			if (!active || !path) {
				clearCredentialInputs(recoveryForm);
				return;
			}
			const pending = recoveryTransport.restore(path, passphrase, recoveryConsent);
			clearCredentialInputs(recoveryForm);
			await pending;
			if (active) {
				recoveryFinished = true;
				recoveryConsent = false;
				guard.markDirty(false);
				recoveryMessage = t.restoreSuccess;
				onRestored?.();
			}
		} catch {
			if (active) recoveryMessage = t.restoreError;
		} finally {
			clearCredentialInputs(recoveryForm);
			if (active) {
				recoveryBusy = false;
				guard.setBusy(false);
				guard.markDirty(false);
			}
		}
	}

	function handleDemoBackupChange(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		if (!file.name.endsWith('.triagebak')) {
			localError = t.restoreError;
			return;
		}
		selectedBackupMeta = { name: file.name, bytes: file.size };
		localError = '';
		toast.success(file.name);
		onDemoUnlock?.('backup', file.name);
	}
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
						<FileArchive class="h-5 w-5" />
					{:else}
						<Lock class="h-5 w-5" />
					{/if}
				</div>
				<div>
					<h2 id="vault-gate-title" class="font-display text-base font-semibold text-foreground">
						{recovery ? t.restoreBackupTitle : t.unlockTitle}
					</h2>
					<p class="mt-0.5 text-xs text-secondary">
						Argon2id + AES-GCM Fallback · Local SQLCipher Storage
					</p>
				</div>
			</div>
			<span class="shrink-0 whitespace-nowrap font-mono text-[11px] font-medium text-status-warning">
				{recovery ? t.recoveryModeBadge : vaultUnlocked ? t.vaultUnlockedBadge : t.vaultLockedBadge}
			</span>
		</div>

		{#if recovery}
			<div class="mt-4 space-y-4">
				<p class="text-xs leading-relaxed text-secondary">{t.restoreBackupHelp}</p>
				{#if !recoveryFinished}
					<form
						bind:this={recoveryForm}
						onsubmit={handleRecoverySubmit}
						oninput={() => guard.markDirty()}
						class="flex flex-col gap-3.5"
						autocomplete="off"
					>
						<label class="flex flex-col gap-1.5 text-xs font-medium text-foreground">
							<span>{t.restorePassphraseLabel}</span>
							<input
								name="passphrase"
								type="password"
								data-credential
								required
								minlength="12"
								maxlength="4096"
								autocomplete="off"
								disabled={recoveryBusy}
								class="h-10 w-full rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
							/>
						</label>
						<label class="flex flex-col gap-1.5 text-xs font-medium text-foreground">
							<span>{t.restoreConfirmLabel}</span>
							<input
								name="confirmation"
								type="password"
								data-credential
								required
								minlength="12"
								maxlength="4096"
								autocomplete="off"
								disabled={recoveryBusy}
								class="h-10 w-full rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
							/>
						</label>
						<label class="flex items-start gap-2 text-xs text-secondary">
							<input
								type="checkbox"
								bind:checked={recoveryConsent}
								disabled={recoveryBusy}
								class="mt-0.5 rounded border-separator"
							/>
							<span>{t.restoreConsentLabel}</span>
						</label>
						<div class="flex flex-wrap items-center gap-2.5 pt-1">
							<button
								type="submit"
								disabled={recoveryBusy || !recoveryConsent}
								class="flex min-h-[40px] flex-1 items-center justify-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap hover:opacity-95 disabled:opacity-50"
							>
								<FileArchive class="h-3.5 w-3.5 shrink-0" />
								<span>{recoveryBusy ? t.unlocking : t.restoreChooseFile}</span>
							</button>
							{#if onSkipRecovery}
								<button
									type="button"
									disabled={recoveryBusy}
									onclick={onSkipRecovery}
									class="flex min-h-[40px] items-center justify-center rounded-md border border-separator bg-surface px-3.5 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
								>
									{t.continueWithoutRestore}
								</button>
							{/if}
						</div>
					</form>
				{:else if onRestored}
					<button
						type="button"
						onclick={onRestored}
						class="flex min-h-[40px] w-full items-center justify-center rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground"
					>
						{t.openInbox}
					</button>
				{/if}
				{#if recoveryMessage}
					<p role="status" class="text-xs font-medium text-foreground">{recoveryMessage}</p>
				{/if}
			</div>
		{:else}
			<div class="mt-4 rounded-md border border-separator bg-field px-3.5 py-2.5 text-xs text-secondary">
				<div class="flex items-start gap-2">
					<ShieldAlert class="mt-0.5 h-3.5 w-3.5 shrink-0 text-status-warning" />
					<p class="leading-relaxed">{t.vaultLockedHelp}</p>
				</div>
			</div>

			<form
				bind:this={unlockForm}
				onsubmit={handlePassphraseSubmit}
				class="mt-5 flex flex-col gap-4"
				autocomplete="off"
			>
				<div class="flex flex-col gap-1.5">
					<label for="vault-passphrase-input" class="text-xs font-medium text-foreground">
						{t.vaultPassphraseLabel}
					</label>
					<div class="relative flex items-center">
						<input
							id="vault-passphrase-input"
							name="passphrase"
							type={showPassphrase ? 'text' : 'password'}
							data-credential
							required
							maxlength="4096"
							autocomplete="off"
							spellcheck="false"
							disabled={busy}
							class="h-10 w-full rounded-md border border-separator bg-field pl-3 pr-10 font-mono text-xs text-foreground placeholder:font-sans placeholder:text-secondary focus:border-accent focus:outline-none"
						/>
						<button
							type="button"
							onclick={() => (showPassphrase = !showPassphrase)}
							aria-label={showPassphrase ? t.hidePassphrase : t.showPassphrase}
							class="absolute right-1.5 flex h-8 w-8 items-center justify-center rounded text-secondary hover:text-foreground"
						>
							{#if showPassphrase}
								<EyeOff class="h-3.5 w-3.5" />
							{:else}
								<Eye class="h-3.5 w-3.5" />
							{/if}
						</button>
					</div>
				</div>

				{#if displayError}
					<p role="alert" class="text-xs font-medium text-status-danger">
						{displayError}
					</p>
				{/if}

				<div class="flex flex-wrap items-center gap-2.5 pt-1">
					<button
						type="submit"
						disabled={busy}
						class="flex min-h-[44px] flex-1 items-center justify-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap shrink-0 transition-transform duration-100 ease-out hover:opacity-95 active:scale-[0.98] disabled:opacity-60 md:min-h-[38px]"
					>
						<KeyRound class="h-3.5 w-3.5 shrink-0" />
						<span>{busy ? t.unlocking : t.unlockVault}</span>
					</button>

					<button
						type="button"
						onclick={() => void handleRetryKeyring()}
						disabled={busy}
						class="flex min-h-[44px] items-center justify-center gap-2 rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground whitespace-nowrap shrink-0 transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-60 md:min-h-[38px]"
					>
						<RefreshCw class="h-3.5 w-3.5 shrink-0 {busy ? 'animate-spin' : ''}" />
						<span>{t.retryKeyring}</span>
					</button>
				</div>
			</form>

			{#if demoMode}
				<div class="mt-5 flex flex-wrap items-center justify-between gap-2 border-t border-separator pt-4">
					<input
						bind:this={backupInputEl}
						type="file"
						accept=".triagebak"
						onchange={handleDemoBackupChange}
						class="hidden"
						aria-label={t.restoreBackupAction}
					/>
					<button
						type="button"
						onclick={() => backupInputEl?.click()}
						class="flex min-h-[40px] items-center gap-2 rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground whitespace-nowrap shrink-0 hover:bg-selection-soft md:min-h-[34px]"
					>
						<FileArchive class="h-3.5 w-3.5 shrink-0 text-secondary" />
						<span>{t.restoreBackupAction}</span>
					</button>
					{#if selectedBackupMeta}
						<span class="max-w-[200px] truncate font-mono text-[11px] tabular-nums text-status-healthy">
							{selectedBackupMeta.name} ({selectedBackupMeta.bytes} B)
						</span>
					{/if}
				</div>
			{/if}
		{/if}
	</section>
</div>
