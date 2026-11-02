<script lang="ts">
	import { labels } from '../rules/labels';
	import { i18n } from '../i18n/index.svelte';
	import type { MessageRowDto } from './types';
	import type { TriageAction } from './triage';
	let {
		selected,
		disabled,
		openPalette,
		createRule,
		mutate
	}: {
		selected?: MessageRowDto;
		disabled: boolean;
		createRule?: (id: string) => void;
		openPalette: (tag?: boolean, snooze?: boolean) => void;
		mutate: (action: TriageAction) => void;
	} = $props();
	const t = $derived(i18n.t.mailboxes);
</script>

<div
	class="flex shrink-0 flex-wrap items-center gap-2 border-b border-[var(--separator)] bg-[var(--toolbar)] px-4 py-2"
>
	<button
		type="button"
		onclick={() => openPalette()}
		disabled={disabled || !selected}
		aria-haspopup="dialog"
		aria-keyshortcuts="Meta+K Control+K"
		class="rounded px-2 py-2 text-sm text-[var(--accent)] disabled:opacity-40"
		>{t.actions}<kbd class="ml-2 text-xs">⌘K / Ctrl+K</kbd></button
	>
	{#if selected}
		{#if createRule}<button
				type="button"
				{disabled}
				onclick={() => selected && createRule?.(selected.id)}
				class="rounded px-2 py-2 text-sm text-[var(--accent)] disabled:opacity-40"
				>{labels[i18n.locale].createFromMessage}</button
			>{/if}
		<button
			type="button"
			{disabled}
			aria-pressed={!!selected.is_starred}
			onclick={() => mutate(selected?.is_starred ? 'Unstar' : 'Star')}
			class="rounded px-2 py-2 text-sm text-[var(--accent)] disabled:opacity-40"
			>{selected.is_starred ? t.unstar : t.star}</button
		>
		<button
			type="button"
			{disabled}
			onclick={() => {
				if ((selected?.snooze_until_epoch_ms ?? 0) > Date.now()) mutate('Unsnooze');
				else openPalette(false, true);
			}}
			class="rounded px-2 py-2 text-sm text-[var(--accent)] disabled:opacity-40"
			>{(selected.snooze_until_epoch_ms ?? 0) > Date.now() ? t.unsnooze : t.snooze}</button
		>
	{/if}
</div>
