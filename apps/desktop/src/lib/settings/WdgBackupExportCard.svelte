<script lang="ts">
	import Download from '@lucide/svelte/icons/download';
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import { toast } from 'svelte-sonner';
	import type { TransferTransport } from '$lib/backup/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import { clearCredentialInputs } from '$lib/onboarding/forms';
	import type { SettingsFormGuard } from './guards';
	import { settingsLabels } from './labels';

	interface Props {
		useNativeFlow: boolean;
		activeTransport: TransferTransport;
		backupGuard: SettingsFormGuard;
		anyBusy: boolean;
		busy: boolean;
		onBusyChange: (busy: boolean) => void;
	}

	let {
		useNativeFlow,
		activeTransport,
		backupGuard,
		anyBusy,
		busy,
		onBusyChange
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].backup);

	let formEl = $state<HTMLFormElement>();
	let consent = $state(false);
	let status = $state('');
	let hasError = $state(false);
	let active = true;

	$effect(() => {
		active = true;
		return () => {
			active = false;
			clearCredentialInputs(formEl);
		};
	});

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (!formEl) return;

		const secret = formEl.elements.namedItem('passphrase') as HTMLInputElement | null;
		const confirm = formEl.elements.namedItem('confirmation') as HTMLInputElement | null;
		const passphrase = secret?.value ?? '';
		const confirmation = confirm?.value ?? '';

		if (!useNativeFlow) {
			clearCredentialInputs(formEl);
			if (passphrase.length < 12 || passphrase !== confirmation) {
				hasError = true;
				status = t.mismatch;
				toast.error(t.mismatch);
				return;
			}
			hasError = false;
			status = t.done;
			toast.success('Demo encrypted backup created locally.');
			return;
		}

		if (anyBusy || !consent) return;
		if (passphrase.length < 12 || passphrase !== confirmation) {
			hasError = true;
			status = t.mismatch;
			return;
		}

		onBusyChange(true);
		hasError = false;
		status = '';
		backupGuard.setBusy(true);
		try {
			const path = await activeTransport.choose(true);
			if (!active || !path) {
				clearCredentialInputs(formEl);
				return;
			}
			// INVARIANT: Zeroize unbound passphrase inputs before awaiting native IPC.
			clearCredentialInputs(formEl);
			await activeTransport.backup(path, passphrase);
			if (active) {
				consent = false;
				status = t.done;
				backupGuard.markDirty(false);
				toast.success(t.done);
			}
		} catch {
			if (active) {
				hasError = true;
				status = t.error;
				toast.error(t.error);
			}
		} finally {
			clearCredentialInputs(formEl);
			if (active) {
				onBusyChange(false);
				backupGuard.setBusy(false);
			}
		}
	}
</script>

<form
	bind:this={formEl}
	onsubmit={(e) => void handleSubmit(e)}
	oninput={() => backupGuard.markDirty(true)}
	aria-label={t.exportTitle}
	autocomplete="off"
	class="flex flex-col justify-between gap-4 rounded-lg border border-separator bg-surface p-4"
>
	<div class="space-y-3">
		<div class="flex items-center gap-2 text-sm font-medium text-foreground">
			<FileArchive class="h-4 w-4 text-secondary" aria-hidden="true" />
			<span>{t.exportTitle}</span>
		</div>
		<p class="text-xs text-secondary">{t.exportDesc}</p>
		<div class="grid grid-cols-1 gap-3">
			<label class="flex flex-col gap-1 text-xs text-foreground">
				<span>{t.passLabel}</span>
				<input
					name="passphrase"
					type="password"
					data-credential
					required
					minlength="12"
					maxlength="4096"
					disabled={anyBusy}
					autocomplete="off"
					class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
				/>
			</label>
			<label class="flex flex-col gap-1 text-xs text-foreground">
				<span>{t.confirmLabel}</span>
				<input
					name="confirmation"
					type="password"
					data-credential
					required
					minlength="12"
					maxlength="4096"
					disabled={anyBusy}
					autocomplete="off"
					class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
				/>
			</label>
			<label class="flex cursor-pointer items-start gap-2 pt-1 text-xs text-secondary">
				<input
					type="checkbox"
					bind:checked={consent}
					disabled={anyBusy}
					class="mt-1 h-3.5 w-3.5 accent-accent"
				/>
				<span>{t.consentLabel}</span>
			</label>
		</div>
	</div>

	<div class="space-y-2">
		{#if status}
			<p role={hasError ? 'alert' : 'status'} class="text-xs {hasError ? 'text-status-danger' : 'text-status-healthy'}">
				{status}
			</p>
		{/if}
		<button
			type="submit"
			disabled={anyBusy || (useNativeFlow && !consent)}
			class="flex min-h-[38px] w-full items-center justify-center gap-2 whitespace-nowrap rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
		>
			<Download class="h-3.5 w-3.5" aria-hidden="true" />
			<span>{busy ? t.busy : t.exportBtn}</span>
		</button>
	</div>
</form>
