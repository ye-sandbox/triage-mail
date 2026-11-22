<script lang="ts">
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import { scheduleLabels } from '$lib/alerts/schedule';
	import type { Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { automationLabels } from './labels';

	interface Props {
		pending: { kind: 'test' | 'replay'; destination: Destination; ids: string[] };
		busy: boolean;
		onConfirm: () => void | Promise<void>;
		onCancel: () => void;
	}

	let { pending, busy, onConfirm, onCancel }: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);
	const schedT = $derived(scheduleLabels[i18n.locale]);
</script>

<div
	role="group"
	aria-label={alertT.send}
	class="flex flex-col gap-3 rounded-lg border border-accent bg-selection-soft/70 p-4 text-xs"
>
	<p class="font-medium text-foreground">
		{pending.kind === 'test' ? alertT.confirmTest : alertT.confirmReplay}
		<span class="font-mono">
			({pending.destination.name} · {pending.destination.public_origin ?? alertT.local})
		</span>
	</p>
	<p class="text-[11px] text-secondary">{schedT.bypass}</p>
	<p class="font-mono text-[11px] text-foreground">
		{alertT.fields}:
		{pending.destination.fields.map((f) => alertT.fieldNames[f]).join(', ') || t.localNoExport}
	</p>
	<div class="flex flex-wrap gap-2">
		<button
			type="button"
			disabled={busy}
			onclick={() => void onConfirm()}
			class="min-h-[34px] rounded-md bg-accent px-4 py-1 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-45"
		>
			{alertT.send}
		</button>
		<button
			type="button"
			disabled={busy}
			onclick={onCancel}
			class="min-h-[34px] rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-secondary hover:text-foreground"
		>
			{alertT.cancel}
		</button>
	</div>
</div>
