<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import { toast } from 'svelte-sonner';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import { newDestination, type AlertsTransport, type Destination, type ExportField } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SettingsFormGuard } from '$lib/settings/guards';
	import { buildDestinationSubmission } from './alert-destination-form';
	import { automationLabels } from './labels';
	import { clonePlain } from './simulator';
	import WdgAlertCredentialsFields from './WdgAlertCredentialsFields.svelte';

	interface Props {
		open: boolean;
		destination: Destination | null;
		alertsTransport: AlertsTransport;
		guard?: SettingsFormGuard;
		busy: boolean;
		onSaved: (saved: Destination, isNew: boolean) => void | Promise<void>;
		onClose: () => void;
	}

	let { open, destination, alertsTransport, guard, busy = $bindable(), onSaved, onClose }: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);

	let draft = $state<Destination>(newDestination());
	let initialKind = $state<Destination['kind']>('Local');
	let destConsent = $state(false);
	let destError = $state('');
	let urlInput = $state<HTMLInputElement>();
	let bearerInput = $state<HTMLInputElement>();

	function clearSecrets() {
		if (urlInput) urlInput.value = '';
		if (bearerInput) bearerInput.value = '';
		urlInput?.setCustomValidity('');
		bearerInput?.setCustomValidity('');
	}

	onDestroy(clearSecrets);

	$effect(() => {
		if (open) {
			untrack(() => {
				clearSecrets();
				destError = '';
				destConsent = false;
				draft = destination ? clonePlain(destination) : newDestination();
				initialKind = draft.kind;
			});
		} else {
			untrack(clearSecrets);
		}
	});

	function updateDraft(next: Destination) {
		destConsent = false;
		destError = '';
		draft = next;
	}

	function handleKindChange(kind: Destination['kind']) {
		clearSecrets();
		updateDraft({
			...draft,
			kind,
			public_origin: kind === 'Telegram' ? 'https://api.telegram.org' : kind === 'Discord' ? 'https://discord.com' : null,
			has_credentials: false,
			fields: []
		});
	}

	function handleUrlInput(input: HTMLInputElement) {
		input.setCustomValidity('');
		if (draft.kind === 'Webhook') {
			let origin: string | null = null;
			try {
				if (input.value) origin = new URL(input.value).origin;
			} catch {
				// Invalid partial URL cleared until validated.
			}
			updateDraft({ ...draft, public_origin: input.value ? origin : draft.public_origin });
		} else {
			destConsent = false;
			destError = '';
		}
	}

	function toggleField(field: ExportField) {
		if (draft.kind === 'Local') return;
		const fields = draft.fields.includes(field) ? draft.fields.filter((f) => f !== field) : [...draft.fields, field];
		updateDraft({ ...draft, fields });
	}

	async function handleSaveDestination(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		destError = '';
		const primaryVal = urlInput?.value.trim() ?? '';
		const secondaryVal = bearerInput?.value.trim() ?? '';
		if (draft.kind === 'Telegram' && (primaryVal || secondaryVal) && (!primaryVal || !secondaryVal)) {
			const missing = !primaryVal ? urlInput : bearerInput;
			missing?.setCustomValidity(alertT.complete);
			missing?.reportValidity();
			return;
		}
		const built = buildDestinationSubmission({
			draft: $state.snapshot(draft),
			initialKind,
			primaryVal,
			secondaryVal,
			destConsent,
			messages: t
		});
		if (!built.ok) {
			destError = built.error;
			return;
		}
		clearSecrets();
		busy = true;
		guard?.setBusy(true);
		try {
			if (built.payload.kind === 'Local' && built.payload.enabled && !(await alertsTransport.available())) {
				destError = alertT.unavailable;
				return;
			}
			const isNew = !destination || destination.revision === 0;
			const saved = await alertsTransport.save(built.payload, built.secret, destConsent);
			guard?.markDirty(false);
			await onSaved(saved, isNew);
			onClose();
			toast.success(isNew ? t.destinationCreatedToast(saved.name) : t.destinationUpdatedToast(saved.name, saved.revision));
		} catch {
			destError = alertT.error;
		} finally {
			busy = false;
			guard?.setBusy(false);
		}
	}
</script>

<form onsubmit={handleSaveDestination} class="text-xs">
	<fieldset disabled={busy} class="flex flex-col gap-4 border-0 p-0">
		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			<label class="flex flex-col gap-1">
				<span class="font-medium text-secondary">{alertT.name}</span>
				<input
					type="text" required maxlength="120" value={draft.name}
					oninput={(e) => updateDraft({ ...draft, name: e.currentTarget.value })}
					placeholder="Ops Triage Channel"
					class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="font-medium text-secondary">{t.kindLabel}</span>
				<select
					value={draft.kind}
					onchange={(e) => handleKindChange(e.currentTarget.value as Destination['kind'])}
					class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
				>
					<option value="Local">{t.kindLocal}</option>
					<option value="Webhook">{t.kindWebhook}</option>
					<option value="Telegram">{t.kindTelegram}</option>
					<option value="Discord">{t.kindDiscord}</option>
				</select>
			</label>
		</div>

		{#if draft.kind === 'Local'}
			<p class="rounded-md border border-separator bg-field/60 p-3 text-[11px] text-secondary">{t.localNoExport}</p>
		{:else}
			<WdgAlertCredentialsFields
				{draft} {initialKind} bind:urlInput bind:bearerInput
				onUrlInput={handleUrlInput}
				onBearerInput={() => { destConsent = false; destError = ''; }}
				onToggleField={toggleField}
			/>
		{/if}

		<label class="flex cursor-pointer items-center gap-2 rounded-md border border-separator bg-field/60 px-3 py-2">
			<input type="checkbox" checked={draft.enabled} onchange={(e) => updateDraft({ ...draft, enabled: e.currentTarget.checked })} class="h-3.5 w-3.5 accent-accent" />
			<span class="font-medium text-foreground">{alertT.enabled}</span>
		</label>

		{#if draft.enabled}
			<label class="flex cursor-pointer items-start gap-3 rounded-md border border-separator bg-selection-soft/60 p-3">
				<input type="checkbox" required bind:checked={destConsent} class="mt-1 h-4 w-4 accent-accent" />
				<span class="text-[11px] leading-relaxed text-foreground">
					<ShieldCheck class="mr-1 inline h-3.5 w-3.5 text-accent" aria-hidden="true" />
					{alertT.consent}
				</span>
			</label>
		{/if}

		{#if destError}
			<p role="alert" class="rounded border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-[11px] text-status-danger">{destError}</p>
		{/if}

		<div class="flex justify-end gap-2 border-t border-separator pt-3">
			<button type="button" onclick={onClose} class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground">
				{alertT.cancel}
			</button>
			<button type="submit" class="min-h-[36px] rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95">
				{alertT.save}
			</button>
		</div>
	</fieldset>
</form>
