<script lang="ts">
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import type { Destination, History } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { HistoryVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';

	interface Props {
		history: History[];
		destinations: Destination[];
		historyMeta: Record<string, HistoryVisualMeta>;
		busy: boolean;
		onRefresh: () => void | Promise<void>;
		onReplayRow: (row: History) => void;
	}

	let { history, destinations, historyMeta, busy, onRefresh, onReplayRow }: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);

	function statusBadgeClass(status: History['status']): string {
		switch (status) {
			case 'Accepted':
				return 'border-status-healthy/40 bg-status-healthy/10 text-status-healthy';
			case 'Pending':
				return 'border-status-warning/40 bg-status-warning/10 text-status-warning';
			case 'Failed':
			case 'Expired':
			case 'Unavailable':
				return 'border-status-danger/40 bg-status-danger/10 text-status-danger';
			case 'Canceled':
				return 'border-separator bg-field text-secondary';
		}
	}
</script>

<section class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<h3 class="font-display text-sm font-semibold text-foreground">{t.historyTitle}</h3>
			<p class="text-xs text-secondary">{t.historySubtitle}</p>
		</div>
		<button
			type="button"
			disabled={busy}
			onclick={() => void onRefresh()}
			class="flex min-h-[34px] items-center gap-2 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
		>
			<RefreshCw class="h-3 w-3" aria-hidden="true" />
			<span>{t.refreshHistory}</span>
		</button>
	</div>

	{#if history.length === 0}
		<p class="rounded border border-separator bg-field p-4 text-center text-xs text-secondary">
			{t.emptyHistory}
		</p>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full border-collapse text-left text-xs">
				<thead>
					<tr class="border-b border-separator text-[11px] font-medium text-secondary">
						<th class="py-2 pr-3 font-mono">{t.colEventId}</th>
						<th class="py-2 pr-3 font-mono">{t.colDestination}</th>
						<th class="py-2 pr-3">{t.colStatus}</th>
						<th class="py-2 pr-3 text-right">{t.colAttempts}</th>
						<th class="py-2 pr-3">{t.colTimestamp}</th>
						<th class="py-2 pr-3">{t.colDetail}</th>
						<th class="py-2 text-right">{t.colAction}</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-separator">
					{#each history as row (row.event_id)}
						{@const dest = destinations.find((d) => d.id === row.destination_id)}
						{@const meta = historyMeta[row.event_id]}
						<tr class="hover:bg-field/50">
							<td class="py-3 pr-3 font-mono text-[11px] tabular-nums text-foreground">
								{row.event_id}
							</td>
							<td class="py-3 pr-3 font-mono text-[11px] text-secondary">
								{dest?.name ?? row.destination_id}
							</td>
							<td class="py-3 pr-3">
								<span
									class="inline-block rounded-md border px-2 py-1 font-mono text-[11px] font-medium {statusBadgeClass(
										row.status
									)}"
								>
									{alertT.status[row.status]}
								</span>
							</td>
							<td class="py-3 pr-3 text-right font-mono tabular-nums text-foreground">
								{row.attempts}
							</td>
							<td class="py-3 pr-3 font-mono text-[11px] tabular-nums text-secondary">
								{new Date(row.created).toLocaleString(i18n.locale)}
							</td>
							<td class="py-3 pr-3 text-secondary">{meta?.detail ?? '—'}</td>
							<td class="py-3 text-right">
								<button
									type="button"
									disabled={busy || !dest?.enabled}
									onclick={() => onReplayRow(row)}
									class="inline-flex min-h-[30px] items-center gap-1 rounded border border-separator bg-surface px-3 py-1 text-[11px] font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
								>
									<RotateCcw class="h-3 w-3" aria-hidden="true" />
									<span>{t.replayButton}</span>
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</section>
