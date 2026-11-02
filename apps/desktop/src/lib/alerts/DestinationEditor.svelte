<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import { EXPORT_FIELDS, type Destination, type AlertSecret } from './types';
	let {
		draft,
		busy,
		change,
		save,
		cancel
	}: {
		draft: Destination;
		busy: boolean;
		change: (d: Destination) => void;
		save: (secret: AlertSecret | null, consent: boolean) => Promise<void>;
		cancel: () => void;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	const initialKind = untrack(() => draft.kind);
	let urlInput = $state<HTMLInputElement>();
	let bearerInput = $state<HTMLInputElement>();
	let consent = $state(false);
	function clear() {
		if (urlInput) urlInput.value = '';
		if (bearerInput) bearerInput.value = '';
		urlInput?.setCustomValidity('');
		bearerInput?.setCustomValidity('');
	}
	onDestroy(clear);
	function update(d: Destination) {
		consent = false;
		change(d);
	}
	function updateUrl(input: HTMLInputElement) {
		let origin: string | null = null;
		try {
			if (input.value) origin = new URL(input.value).origin;
		} catch {
			/* Native validation remains authoritative. */
		}
		update({
			...draft,
			public_origin: input.value ? origin : draft.public_origin
		});
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		let secret: AlertSecret | null = null;
		if (draft.kind === 'Telegram' && (urlInput?.value || bearerInput?.value)) {
			if (!urlInput?.value || !bearerInput?.value) {
				const missing = !urlInput?.value ? urlInput : bearerInput;
				missing?.setCustomValidity(t.complete);
				missing?.reportValidity();
				return;
			}
			secret = { token: urlInput.value, chat_id: bearerInput.value };
		} else if (draft.kind !== 'Local' && urlInput?.value) {
			secret = {
				url: urlInput.value,
				bearer: draft.kind === 'Webhook' ? bearerInput?.value || null : null
			};
		}
		clear();
		await save(secret, consent);
	}
</script>

<form onsubmit={submit} class="space-y-4">
	<fieldset disabled={busy} class="space-y-4">
		<label class="block"
			>{t.name}<input
				required
				maxlength="120"
				value={draft.name}
				oninput={(e) => update({ ...draft, name: e.currentTarget.value })}
				class="block w-full bg-[var(--surface)] border rounded p-2"
			/></label
		>
		<label class="block"
			>{t.title}<select
				value={draft.kind}
				onchange={(e) => {
					clear();
					update({
						...draft,
						kind: e.currentTarget.value as Destination['kind'],
						public_origin:
							e.currentTarget.value === 'Telegram'
								? 'https://api.telegram.org'
								: e.currentTarget.value === 'Discord'
									? 'https://discord.com'
									: null,
						has_credentials: false,
						fields: []
					});
				}}
				class="block w-full bg-[var(--surface)] border rounded p-2"
				><option value="Local">{t.local}</option><option value="Webhook">{t.webhook}</option><option
					value="Telegram">Telegram</option
				><option value="Discord">Discord</option></select
			></label
		>
		{#if draft.kind !== 'Local'}
			{#if draft.kind === 'Telegram'}<p>
					{t.telegramHelp}
				</p>{:else if draft.kind === 'Discord'}<p>{t.discordHelp}</p>{/if}
			<p>{t.revoke}</p>
			<p>{t.preserve}</p>
			<p>{draft.public_origin ?? '—'}</p>
			<label class="block"
				>{draft.kind === 'Telegram' ? t.token : t.url}<input
					bind:this={urlInput}
					type="password"
					autocomplete="off"
					required={draft.revision === 0 || draft.kind !== initialKind}
					oninput={(e) => {
						e.currentTarget.setCustomValidity('');
						if (draft.kind === 'Webhook') updateUrl(e.currentTarget);
						else consent = false;
					}}
					class="block w-full bg-[var(--surface)] border rounded p-2"
				/></label
			>
			{#if draft.kind !== 'Discord'}<label class="block"
					>{draft.kind === 'Telegram' ? t.chat : t.bearer}<input
						bind:this={bearerInput}
						type="password"
						autocomplete="off"
						required={draft.kind === 'Telegram' &&
							(draft.revision === 0 || draft.kind !== initialKind)}
						oninput={(e) => {
							e.currentTarget.setCustomValidity('');
							consent = false;
						}}
						class="block w-full bg-[var(--surface)] border rounded p-2"
					/></label
				>{/if}
			<fieldset class="space-y-2">
				<legend>{t.fields}</legend>
				{#each EXPORT_FIELDS as field}<label class="flex gap-2"
						><input
							type="checkbox"
							checked={draft.fields.includes(field)}
							onchange={(e) =>
								update({
									...draft,
									fields: e.currentTarget.checked
										? [...draft.fields, field]
										: draft.fields.filter((f) => f !== field)
								})}
						/>{t.fieldNames[field]}</label
					>{/each}
			</fieldset>
			<p>{t.metadata}</p>
			<p>
				{t.preview}: {draft.fields.map((f) => t.fieldNames[f]).join(', ') || '—'}
			</p>
		{/if}
		<label class="flex gap-2"
			><input
				type="checkbox"
				checked={draft.enabled}
				onchange={(e) => update({ ...draft, enabled: e.currentTarget.checked })}
			/>{t.enabled}</label
		>
		{#if draft.enabled}<label class="flex gap-2"
				><input type="checkbox" required bind:checked={consent} />{t.consent}</label
			>{/if}
		<div class="flex gap-4">
			<button type="submit">{t.save}</button><button type="button" onclick={cancel}
				>{t.cancel}</button
			>
		</div>
	</fieldset>
</form>
