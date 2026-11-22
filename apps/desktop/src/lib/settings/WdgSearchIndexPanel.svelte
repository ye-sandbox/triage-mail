<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import Database from '@lucide/svelte/icons/database';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';
	import type { Settings, SettingsTransport, Tokenizer } from './types';
	import WdgOnnxModelCard from './WdgOnnxModelCard.svelte';
	import WdgSemanticSearchCard from './WdgSemanticSearchCard.svelte';

	interface Props {
		settings: Settings;
		transport: SettingsTransport;
		busy?: boolean;
		onReload?: () => void | Promise<void>;
	}

	let { settings, transport, busy = false, onReload }: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].search);

	let selectedTokenizer = $state<Tokenizer>(untrack(() => settings.tokenizer));
	let rebuildingFts = $state(false);
	let ftsFeedback = $state<'ok' | 'error' | ''>('');
	let semanticWorking = $state(false);
	let downloadingOnnx = $state(false);
	let disposed = false;

	onDestroy(() => {
		disposed = true;
	});

	$effect(() => {
		const nextTokenizer = settings.tokenizer;
		if (!rebuildingFts) selectedTokenizer = nextTokenizer;
	});

	const isBusy = $derived(busy || rebuildingFts || semanticWorking || downloadingOnnx);

	async function handleApplyTokenizer(event: SubmitEvent) {
		event.preventDefault();
		if (isBusy) return;
		rebuildingFts = true;
		ftsFeedback = '';
		try {
			await transport.tokenizer(selectedTokenizer);
			if (!disposed) {
				ftsFeedback = 'ok';
				toast.success(t.tokenizerUpdated);
				await onReload?.();
			}
		} catch {
			if (!disposed) {
				ftsFeedback = 'error';
				toast.error(t.tokenizerError);
			}
		} finally {
			if (!disposed) rebuildingFts = false;
		}
	}
</script>

<section class="flex flex-col gap-5" aria-label={t.heading} aria-labelledby="search-index-heading">
	<div class="border-b border-separator pb-3">
		<h2 id="search-index-heading" class="font-display text-base font-semibold text-foreground">
			{t.heading}
		</h2>
		<p class="mt-1 text-xs text-secondary">{t.subtitle}</p>
	</div>

	<form
		onsubmit={handleApplyTokenizer}
		aria-label={t.tokenizerTitle}
		class="flex flex-col gap-4 rounded-lg border border-separator bg-surface p-4"
	>
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div class="space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<Database class="h-4 w-4 text-secondary" aria-hidden="true" />
					<span>{t.tokenizerTitle}</span>
				</div>
				<p class="text-xs text-secondary">{t.tokenizerDesc}</p>
			</div>
			<button
				type="submit"
				disabled={isBusy}
				class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
			>
				<RefreshCw class="h-3.5 w-3.5 {rebuildingFts ? 'animate-spin' : ''}" aria-hidden="true" />
				<span>{rebuildingFts ? t.rebuildingTokenizer : t.applyTokenizer}</span>
			</button>
		</div>

		<div role="radiogroup" aria-label={t.tokenizerSelectLabel} class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			<button
				type="button"
				role="radio"
				aria-checked={selectedTokenizer === 'Unicode61RemoveDiacritics'}
				disabled={isBusy}
				onclick={() => (selectedTokenizer = 'Unicode61RemoveDiacritics')}
				class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:opacity-50 {selectedTokenizer ===
				'Unicode61RemoveDiacritics'
					? 'border-accent bg-selection-soft'
					: 'border-separator bg-field hover:bg-selection-soft/40'}"
			>
				<span class="font-mono text-xs font-semibold text-foreground">Unicode61RemoveDiacritics</span>
				<span class="text-[11px] text-foreground">{t.unicodeLabel}</span>
				<span class="text-[11px] text-secondary">{t.unicodeDesc}</span>
			</button>

			<button
				type="button"
				role="radio"
				aria-checked={selectedTokenizer === 'Trigram'}
				disabled={isBusy}
				onclick={() => (selectedTokenizer = 'Trigram')}
				class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:opacity-50 {selectedTokenizer ===
				'Trigram'
					? 'border-accent bg-selection-soft'
					: 'border-separator bg-field hover:bg-selection-soft/40'}"
			>
				<span class="font-mono text-xs font-semibold text-foreground">Trigram</span>
				<span class="text-[11px] text-foreground">{t.trigramLabel}</span>
				<span class="text-[11px] text-secondary">{t.trigramDesc}</span>
			</button>
		</div>

		{#if selectedTokenizer === 'Trigram'}
			<div class="flex items-start gap-2 rounded-md border border-status-warning/40 bg-status-warning/10 px-3 py-2 text-xs text-status-warning">
				<AlertTriangle class="mt-1 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span>{t.trigramWarning}</span>
			</div>
		{/if}

		{#if ftsFeedback === 'ok'}
			<p role="status" class="text-xs text-status-healthy">{t.tokenizerUpdated}</p>
		{:else if ftsFeedback === 'error'}
			<p role="alert" class="text-xs text-status-danger">{t.tokenizerError}</p>
		{/if}
	</form>

	<WdgSemanticSearchCard
		{settings}
		{transport}
		{busy}
		{semanticWorking}
		onWorkingChange={(w) => (semanticWorking = w)}
		{onReload}
	/>

	<WdgOnnxModelCard
		{settings}
		{transport}
		{busy}
		{downloadingOnnx}
		onDownloadingChange={(d) => (downloadingOnnx = d)}
		{onReload}
	/>
</section>
