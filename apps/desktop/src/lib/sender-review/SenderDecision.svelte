<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import type { SenderChoice, ReviewState } from './types';
	let {
		value,
		accountLabel,
		disabled = false,
		onSave
	}: {
		value: SenderChoice;
		accountLabel: string;
		disabled?: boolean;
		onSave: (state: ReviewState) => Promise<void>;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	let consent = $state(false);
	$effect(() => {
		value.revision;
		consent = false;
	});
</script>

<fieldset {disabled} class="space-y-2 rounded border border-[var(--separator)] p-4 text-sm">
	<p class="break-all">{accountLabel} · {value.sender} · {t[value.state]}</p>
	<label class="flex gap-2"><input type="checkbox" bind:checked={consent} />{t.consent}</label>
	<div class="flex flex-wrap gap-4">
		<button disabled={!consent} onclick={() => void onSave('Accepted')}>{t.accept}</button>
		<button disabled={!consent} onclick={() => void onSave('Declined')}>{t.decline}</button>
		<button disabled={!consent || value.state === 'Pending'} onclick={() => void onSave('Pending')}
			>{t.pending}</button
		>
	</div>
</fieldset>
