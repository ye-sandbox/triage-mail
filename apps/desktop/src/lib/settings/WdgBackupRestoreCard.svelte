<script lang="ts">
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import ShieldAlert from '@lucide/svelte/icons/shield-alert';
	import Upload from '@lucide/svelte/icons/upload';
	import { toast } from 'svelte-sonner';
	import type { TransferTransport } from '$lib/backup/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import { clearCredentialInputs } from '$lib/onboarding/forms';
	import type { SettingsFormGuard } from './guards';
	import { settingsLabels } from './labels';

	interface Props {
		useNativeFlow: boolean;
		restore: boolean;
		recoveryAvailable: boolean;
		activeTransport: TransferTransport;
		backupGuard: SettingsFormGuard;
		anyBusy: boolean;
		busy: boolean;
		onBusyChange: (busy: boolean) => void;
		ondone: () => void;
	}

	let {
		useNativeFlow,
		restore,
		recoveryAvailable,
		activeTransport,
		backupGuard,
		anyBusy,
		busy,
		onBusyChange,
		ondone
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].backup);

	let formEl = $state<HTMLFormElement>();
	let fileEl = $state<HTMLInputElement>();
	let demoFile = $state<string | null>(null);
	let consent = $state(false);
	let finished = $state(false);
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

	function handleDemoFile(event: Event) {
		const file = event.currentTarget instanceof HTMLInputElement ? event.currentTarget.files?.[0] : null;
		if (!file) return;
		if (!file.name.endsWith('.triagebak')) {
			toast.error(t.error);
			if (fileEl) fileEl.value = '';
			return;
		}
		demoFile = file.name;
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (!formEl) return;

		const secret = formEl.elements.namedItem('passphrase') as HTMLInputElement | null;
		const confirm = formEl.elements.namedItem('confirmation') as HTMLInputElement | null;
		const passphrase = secret?.value ?? '';
		const confirmation = confirm?.value ?? '';

		if (!useNativeFlow) {
			clearCredentialInputs(formEl);
			if (!demoFile || passphrase.length < 12) {
				toast.error(t.error);
				return;
			}
			toast.success(`Verified demo snapshot "${demoFile}"`);
			demoFile = null;
			if (fileEl) fileEl.value = '';
			return;
		}

		if (anyBusy || !consent || finished) return;
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
			const path = await activeTransport.choose(false);
			if (!active || !path) {
				clearCredentialInputs(formEl);
				return;
			}
			// INVARIANT: Zeroize unbound passphrase inputs before awaiting native IPC.
			clearCredentialInputs(formEl);
			await activeTransport.restore(path, passphrase, true);
			if (active) {
				finished = true;
				consent = false;
				status = t.restored;
				backupGuard.markDirty(false);
				toast.success(t.restored);
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

{#if restore || !useNativeFlow}
	<form
		bind:this={formEl}
		onsubmit={(e) => void handleSubmit(e)}
		oninput={() => backupGuard.markDirty(true)}
		aria-label={t.restoreTitle}
		autocomplete="off"
		class="flex flex-col justify-between gap-4 rounded-lg border border-separator bg-surface p-4"
	>
		<div class="space-y-3">
			<div class="flex items-center gap-2 text-sm font-medium text-foreground">
				<Upload class="h-4 w-4 text-secondary" aria-hidden="true" />
				<span>{t.restoreTitle}</span>
			</div>
			<p class="text-xs text-secondary">{t.recovery}</p>

			{#if !finished}
				{#if !useNativeFlow}
					<input
						bind:this={fileEl}
						type="file"
						accept=".triagebak"
						onchange={handleDemoFile}
						class="hidden"
						aria-label="Select .triagebak file"
					/>
					<button
						type="button"
						onclick={() => fileEl?.click()}
						class="flex h-9 w-full items-center justify-between rounded-md border border-separator bg-field px-3 text-xs text-foreground hover:bg-selection-soft"
					>
						<span class="truncate font-mono">{demoFile ?? 'Select .triagebak file…'}</span>
						<span class="shrink-0 text-secondary">Browse</span>
					</button>
				{/if}

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

				{#if useNativeFlow}
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
				{/if}
			{/if}
		</div>

		<div class="space-y-2">
			{#if status}
				<p role={hasError ? 'alert' : 'status'} class="text-xs {hasError ? 'text-status-danger' : 'text-status-healthy'}">
					{status}
				</p>
			{/if}

			{#if finished}
				<button
					type="button"
					onclick={ondone}
					class="flex min-h-[38px] w-full items-center justify-center rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95"
				>
					{t.continueBtn}
				</button>
			{:else}
				<button
					type="submit"
					disabled={anyBusy || (useNativeFlow && !consent)}
					class="flex min-h-[38px] w-full items-center justify-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
				>
					<FileArchive class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{busy ? t.busy : t.restoreBtn}</span>
				</button>
			{/if}
		</div>
	</form>
{:else}
	<div class="flex flex-col justify-between gap-4 rounded-lg border border-separator bg-surface p-4">
		<div class="space-y-3">
			<div class="flex items-center gap-2 text-sm font-medium text-foreground">
				<ShieldAlert class="h-4 w-4 text-status-warning" aria-hidden="true" />
				<span>{t.restoreTitle}</span>
			</div>
			<p class="text-xs font-medium text-foreground">{t.restoreUnavailableTitle}</p>
			<p class="text-xs leading-relaxed text-secondary">{t.recovery}</p>
		</div>
		<span class="font-mono text-[11px] text-secondary">
			recoveryAvailable: {recoveryAvailable ? 'true' : 'false'}
		</span>
	</div>
{/if}
