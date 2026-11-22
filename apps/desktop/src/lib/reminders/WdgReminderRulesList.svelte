<script lang="ts">
	import Bell from '@lucide/svelte/icons/bell';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels } from './labels';
	import type { ScheduledReminderConfig } from './types';

	interface Props {
		configs: ScheduledReminderConfig[];
		loading: boolean;
		busy: boolean;
		onCreate: () => void;
		onEdit: (cfg: ScheduledReminderConfig) => void;
		onToggleEnabled: (cfg: ScheduledReminderConfig) => void | Promise<void>;
		onDelete: (cfg: ScheduledReminderConfig) => void | Promise<void>;
	}

	let { configs, loading, busy, onCreate, onEdit, onToggleEnabled, onDelete }: Props = $props();

	const t = $derived(labels[i18n.locale]);

	function formatMinAmountDisplay(cents?: number): string {
		if (!cents) return '';
		return new Intl.NumberFormat(i18n.locale === 'pt-BR' ? 'pt-BR' : 'en-US', {
			style: 'currency',
			currency: 'BRL'
		}).format(cents / 100);
	}
</script>

<div class="flex items-center justify-between gap-2">
	<span class="font-medium text-foreground">{t.rulesTitle}</span>
	<button
		type="button"
		onclick={onCreate}
		disabled={loading || busy || configs.length >= 50}
		class="flex min-h-[36px] items-center gap-2 rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
	>
		<Plus class="h-3.5 w-3.5" aria-hidden="true" />
		<span>{t.addRule}</span>
	</button>
</div>

{#if loading}
	<div class="rounded-md border border-separator bg-field p-6 text-center text-secondary">
		{t.loadingBills}
	</div>
{:else if configs.length === 0}
	<div class="flex flex-col items-center justify-center gap-2 rounded-md border border-dashed border-separator bg-field py-8 text-center">
		<Bell class="h-6 w-6 text-secondary" aria-hidden="true" />
		<p class="text-xs text-secondary">{t.emptyRules}</p>
	</div>
{:else}
	<div class="flex flex-col gap-2">
		{#each configs as cfg (cfg.id)}
			<div
				class="flex items-center justify-between gap-3 rounded-md border border-separator bg-field p-3"
			>
				<div class="flex min-w-0 items-center gap-3">
					<button
						type="button"
						role="switch"
						aria-checked={cfg.enabled}
						aria-label={cfg.enabled ? t.enabled : t.disabled}
						disabled={busy}
						onclick={() => void onToggleEnabled(cfg)}
						class="min-h-[30px] shrink-0 rounded-md border border-separator px-2 py-1 font-mono text-[11px] font-medium transition-colors {cfg.enabled
							? 'bg-selection-soft text-status-healthy'
							: 'bg-surface text-secondary'}"
					>
						{cfg.enabled ? t.enabled : t.disabled}
					</button>

					<div class="min-w-0">
						<div class="truncate font-medium text-foreground">{cfg.name}</div>
						<div class="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-secondary">
							{#if 'DueDate' in cfg.trigger}
								<span>
									{t.ruleSummaryDueDate(
										cfg.trigger.DueDate.days_before,
										cfg.trigger.DueDate.target_hour
									)}
								</span>
								{#if cfg.trigger.DueDate.min_amount_cents}
									<span aria-hidden="true">·</span>
									<span>
										{t.minAmountSummary(
											formatMinAmountDisplay(cfg.trigger.DueDate.min_amount_cents)
										)}
									</span>
								{/if}
							{:else}
								<span>{t.ruleSummaryUnread(cfg.trigger.UnreadInterval.interval_hours)}</span>
								{#if cfg.trigger.UnreadInterval.min_amount_cents}
									<span aria-hidden="true">·</span>
									<span>
										{t.minAmountSummary(
											formatMinAmountDisplay(cfg.trigger.UnreadInterval.min_amount_cents)
										)}
									</span>
								{/if}
							{/if}
						</div>
					</div>
				</div>

				<div class="flex shrink-0 items-center gap-2">
					<button
						type="button"
						onclick={() => onEdit(cfg)}
						disabled={busy}
						class="rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
					>
						{t.edit}
					</button>
					<button
						type="button"
						onclick={() => void onDelete(cfg)}
						disabled={busy}
						aria-label={`${t.delete} ${cfg.name}`}
						class="rounded-md border border-separator bg-surface p-2 text-status-danger hover:bg-status-danger/10"
					>
						<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
				</div>
			</div>
		{/each}
	</div>
{/if}
