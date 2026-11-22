<script lang="ts">
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import CalendarCheck2 from '@lucide/svelte/icons/calendar-check-2';
	import Clock from '@lucide/svelte/icons/clock';
	import { formatBrl } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import { labels } from './labels';
	import { calculateDueDateStatus, type DueDateItem, type DueDateStatus } from './types';

	interface Props {
		items: DueDateItem[];
		activeStatus: 'All' | DueDateStatus;
		onSelectStatus: (status: 'All' | DueDateStatus) => void;
	}

	let { items, activeStatus, onSelectStatus }: Props = $props();

	const t = $derived(labels[i18n.locale]);

	const buckets = $derived.by(() => {
		const summarize = (status: Exclude<DueDateStatus, 'Dismissed'>) => {
			const matching = items.filter((item) => calculateDueDateStatus(item) === status);
			const totalCents = matching.reduce((sum, item) => sum + (item.amount_cents ?? 0), 0);
			return { count: matching.length, totalCents };
		};

		return [
			{
				status: 'Overdue' as const,
				label: t.overdue,
				hint: t.overdueHint,
				icon: AlertCircle,
				tokenClass: 'text-status-danger',
				badgeBgClass: 'bg-status-danger/10 text-status-danger border-status-danger/25',
				ringClass: 'border-status-danger/50 bg-status-danger/5',
				...summarize('Overdue')
			},
			{
				status: 'DueToday' as const,
				label: t.dueToday,
				hint: t.dueTodayHint,
				icon: Clock,
				tokenClass: 'text-status-warning',
				badgeBgClass: 'bg-status-warning/10 text-status-warning border-status-warning/25',
				ringClass: 'border-status-warning/50 bg-status-warning/5',
				...summarize('DueToday')
			},
			{
				status: 'Upcoming' as const,
				label: t.upcoming,
				hint: t.upcomingHint,
				icon: CalendarCheck2,
				tokenClass: 'text-status-healthy',
				badgeBgClass: 'bg-status-healthy/10 text-status-healthy border-status-healthy/25',
				ringClass: 'border-status-healthy/50 bg-status-healthy/5',
				...summarize('Upcoming')
			}
		];
	});

	function toggleFilter(status: Exclude<DueDateStatus, 'Dismissed'>) {
		onSelectStatus(activeStatus === status ? 'All' : status);
	}
</script>

<section aria-label={t.dueDateTotalsAria} class="grid grid-cols-1 gap-3 sm:grid-cols-3">
	{#each buckets as bucket (bucket.status)}
		{@const BucketIcon = bucket.icon}
		{@const isSelected = activeStatus === bucket.status}
		<button
			type="button"
			aria-pressed={isSelected}
			onclick={() => toggleFilter(bucket.status)}
			class="flex flex-col justify-between gap-3 rounded-lg border p-4 text-left transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent active:scale-[0.99] {isSelected
				? bucket.ringClass
				: 'border-separator bg-surface hover:bg-field'}"
		>
			<div class="flex items-center justify-between gap-2">
				<span class="flex items-center gap-2 text-xs font-semibold {bucket.tokenClass}">
					<BucketIcon class="h-4 w-4 shrink-0" />
					<span class="whitespace-nowrap">{bucket.label}</span>
				</span>
				<span
					class="rounded-md border px-2 py-1 font-mono text-xs font-semibold tabular-nums {bucket.badgeBgClass}"
				>
					{bucket.count}
				</span>
			</div>

			<div class="flex items-baseline justify-between gap-2">
				<span class="font-display text-xl font-bold tabular-nums text-foreground">
					{formatBrl(bucket.totalCents, appStore.locale)}
				</span>
				<span class="text-[11px] whitespace-nowrap text-secondary">{bucket.hint}</span>
			</div>
		</button>
	{/each}
</section>
