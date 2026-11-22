<script lang="ts">
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import { i18n } from '../i18n/index.svelte';
	import { SettingsFormGuard } from '../settings/guards';
	import { clearCredentialInputs } from './forms';
	import type { RecoveryTransport } from './types';

	interface Props {
		recoveryTransport: RecoveryTransport;
		onSkipRecovery?: () => void;
		onRestored?: () => void;
	}

	let { recoveryTransport, onSkipRecovery, onRestored }: Props = $props();

	let recoveryForm = $state<HTMLFormElement>();
	let recoveryBusy = $state(false);
	let recoveryConsent = $state(false);
	let recoveryFinished = $state(false);
	let recoveryMessage = $state('');
	const guard = new SettingsFormGuard();
	let active = true;

	const t = $derived(i18n.t.onboarding);

	$effect(() => {
		active = true;
		const currentForm = recoveryForm;
		return () => {
			active = false;
			clearCredentialInputs(currentForm);
			guard.reset();
		};
	});

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
</script>

<div class="mt-4 space-y-4">
	<p class="text-xs leading-relaxed text-secondary">{t.restoreBackupHelp}</p>
	{#if !recoveryFinished}
		<form
			bind:this={recoveryForm}
			onsubmit={handleRecoverySubmit}
			oninput={() => guard.markDirty()}
			class="flex flex-col gap-4"
			autocomplete="off"
		>
			<label class="flex flex-col gap-2 text-xs font-medium text-foreground">
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
			<label class="flex flex-col gap-2 text-xs font-medium text-foreground">
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
					class="mt-1 rounded border-separator"
				/>
				<span>{t.restoreConsentLabel}</span>
			</label>
			<div class="flex flex-wrap items-center gap-3 pt-1">
				<button
					type="submit"
					disabled={recoveryBusy || !recoveryConsent}
					class="flex min-h-[40px] flex-1 items-center justify-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap hover:opacity-95 disabled:opacity-50"
				>
					<FileArchive class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
					<span>{recoveryBusy ? t.unlocking : t.restoreChooseFile}</span>
				</button>
				{#if onSkipRecovery}
					<button
						type="button"
						disabled={recoveryBusy}
						onclick={onSkipRecovery}
						class="flex min-h-[40px] items-center justify-center rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
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
