<script lang="ts">
	import Eye from '@lucide/svelte/icons/eye';
	import EyeOff from '@lucide/svelte/icons/eye-off';
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import ShieldAlert from '@lucide/svelte/icons/shield-alert';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountController } from './controller.svelte';
	import { clearCredentialInputs } from './forms';

	interface Props {
		controller?: AccountController;
		isFirstRun: boolean;
		keyringUnavailable: boolean;
		demoMode: boolean;
		onDemoUnlock?: (method: 'passphrase' | 'keyring' | 'backup', detail?: string) => void;
	}

	let { controller, isFirstRun, keyringUnavailable, demoMode, onDemoUnlock }: Props = $props();

	let unlockForm = $state<HTMLFormElement>();
	let backupInputEl = $state<HTMLInputElement>();
	let showPassphrase = $state(false);
	let demoBusy = $state(false);
	let localError = $state('');
	let selectedBackupMeta = $state<{ name: string; bytes: number } | null>(null);
	let active = true;

	const t = $derived(i18n.t.onboarding);
	const busy = $derived(demoMode ? demoBusy : Boolean(controller?.busy));
	const displayError = $derived(localError || (!demoMode ? (controller?.error ?? '') : ''));

	$effect(() => {
		active = true;
		const currentForm = unlockForm;
		return () => {
			active = false;
			clearCredentialInputs(currentForm);
		};
	});

	async function handlePassphraseSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !unlockForm) return;
		const input = unlockForm.elements.namedItem('passphrase') as HTMLInputElement | null;
		const confirmInput = unlockForm.elements.namedItem('confirmation') as HTMLInputElement | null;
		const value = input?.value ?? '';
		const confirmValue = confirmInput?.value ?? '';
		clearCredentialInputs(unlockForm);

		if (isFirstRun && (value.length < 8 || value !== confirmValue)) {
			localError = t.vaultPassphraseMismatch;
			return;
		}

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

<div class="mt-4 rounded-md border border-separator bg-field px-4 py-3 text-xs text-secondary">
	<div class="flex items-start gap-2">
		<ShieldAlert class="mt-1 h-3.5 w-3.5 shrink-0 text-status-warning" aria-hidden="true" />
		<p class="leading-relaxed">
			{isFirstRun
				? keyringUnavailable
					? t.vaultCreateKeyringUnavailableHelp
					: t.vaultCreateHelp
				: t.vaultLockedHelp}
		</p>
	</div>
</div>

<form
	bind:this={unlockForm}
	onsubmit={handlePassphraseSubmit}
	class="mt-5 flex flex-col gap-4"
	autocomplete="off"
>
	<div class="flex flex-col gap-2">
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
				minlength={isFirstRun ? 8 : undefined}
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
					<EyeOff class="h-3.5 w-3.5" aria-hidden="true" />
				{:else}
					<Eye class="h-3.5 w-3.5" aria-hidden="true" />
				{/if}
			</button>
		</div>
		{#if isFirstRun}
			<span class="text-[11px] text-secondary">{t.vaultPassphraseMinHint}</span>
		{/if}
	</div>

	{#if isFirstRun}
		<div class="flex flex-col gap-2">
			<label for="vault-passphrase-confirm-input" class="text-xs font-medium text-foreground">
				{t.vaultConfirmPassphraseLabel}
			</label>
			<input
				id="vault-passphrase-confirm-input"
				name="confirmation"
				type={showPassphrase ? 'text' : 'password'}
				data-credential
				required
				minlength="8"
				maxlength="4096"
				autocomplete="off"
				spellcheck="false"
				disabled={busy}
				class="h-10 w-full rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground placeholder:font-sans placeholder:text-secondary focus:border-accent focus:outline-none"
			/>
		</div>
	{/if}

	{#if displayError}
		<p role="alert" class="text-xs font-medium text-status-danger">{displayError}</p>
	{/if}

	<div class="flex flex-wrap items-center gap-3 pt-1">
		<button
			type="submit"
			disabled={busy}
			class="flex min-h-[44px] flex-1 items-center justify-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap shrink-0 transition-transform duration-100 ease-out hover:opacity-95 active:scale-[0.98] disabled:opacity-60 md:min-h-[38px]"
		>
			<KeyRound class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
			<span>
				{isFirstRun
					? busy
						? t.creatingVault
						: t.createVaultAction
					: busy
						? t.unlocking
						: t.unlockVault}
			</span>
		</button>

		{#if !isFirstRun}
			<button
				type="button"
				onclick={() => void handleRetryKeyring()}
				disabled={busy}
				class="flex min-h-[44px] items-center justify-center gap-2 rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground whitespace-nowrap shrink-0 transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-60 md:min-h-[38px]"
			>
				<RefreshCw class="h-3.5 w-3.5 shrink-0 {busy ? 'animate-spin' : ''}" aria-hidden="true" />
				<span>{t.retryKeyring}</span>
			</button>
		{/if}
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
			class="flex min-h-[40px] items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap shrink-0 hover:bg-selection-soft md:min-h-[34px]"
		>
			<FileArchive class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
			<span>{t.restoreBackupAction}</span>
		</button>
		{#if selectedBackupMeta}
			<span class="max-w-[200px] truncate font-mono text-[11px] tabular-nums text-status-healthy">
				{selectedBackupMeta.name} ({selectedBackupMeta.bytes} B)
			</span>
		{/if}
	</div>
{/if}
