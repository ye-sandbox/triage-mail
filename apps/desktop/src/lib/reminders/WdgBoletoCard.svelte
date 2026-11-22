<script lang="ts">
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import Archive from '@lucide/svelte/icons/archive';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import Check from '@lucide/svelte/icons/check';
	import Clock from '@lucide/svelte/icons/clock';
	import Copy from '@lucide/svelte/icons/copy';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import XCircle from '@lucide/svelte/icons/circle-x';
	import { toast } from 'svelte-sonner';
	import { formatBrl } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { parseSender } from '$lib/inbox/workspace.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import { labels } from './labels';
	import { calculateDueDateStatus, type DueDateItem, type DueDateStatus } from './types';

	interface Props {
		item: DueDateItem;
		accountLabel: string;
		onDismiss: (messageId: string) => void;
		onViewEmail: (item: DueDateItem) => void;
	}

	let { item, accountLabel, onDismiss, onViewEmail }: Props = $props();

	const t = $derived(labels[i18n.locale]);
	const sender = $derived(parseSender(item.sender));
	const status = $derived(calculateDueDateStatus(item));

	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		return () => {
			if (copyTimer) clearTimeout(copyTimer);
		};
	});

	const statusMeta = $derived.by(() => {
		const map: Record<
			DueDateStatus,
			{ label: string; icon: typeof AlertCircle; textClass: string; badgeClass: string }
		> = {
			Overdue: {
				label: t.status.Overdue,
				icon: AlertCircle,
				textClass: 'text-status-danger',
				badgeClass: 'border-status-danger/30 bg-status-danger/10 text-status-danger'
			},
			DueToday: {
				label: t.status.DueToday,
				icon: Clock,
				textClass: 'text-status-warning',
				badgeClass: 'border-status-warning/30 bg-status-warning/10 text-status-warning'
			},
			Upcoming: {
				label: t.status.Upcoming,
				icon: CalendarClock,
				textClass: 'text-status-healthy',
				badgeClass: 'border-status-healthy/30 bg-status-healthy/10 text-status-healthy'
			},
			Dismissed: {
				label: t.status.Dismissed,
				icon: Archive,
				textClass: 'text-secondary',
				badgeClass: 'border-separator bg-field text-secondary'
			}
		};
		return map[status];
	});

	const relativeDelta = $derived.by(() => {
		const today = new Date().toISOString().slice(0, 10);
		const dueTime = new Date(`${item.due_date_iso}T00:00:00Z`).getTime();
		const todayTime = new Date(`${today}T00:00:00Z`).getTime();
		const diffDays = Math.round((dueTime - todayTime) / 86_400_000);
		if (status === 'DueToday' || diffDays === 0) return t.today;
		if (diffDays < 0) return t.daysOverdue(Math.abs(diffDays));
		return t.daysRemaining(diffDays);
	});

	const amountDisplay = $derived(
		item.amount_cents !== null && item.amount_cents !== undefined
			? formatBrl(item.amount_cents, appStore.locale)
			: t.noAmount
	);

	const extractedDateIso = $derived(new Date(item.received_at_epoch_ms).toISOString().slice(0, 10));

	async function copyBarcode() {
		try {
			await navigator.clipboard.writeText(item.digitable_line);
		} catch {
			// Fallback when clipboard permission is unavailable in headless environments
		}
		copied = true;
		if (copyTimer) clearTimeout(copyTimer);
		copyTimer = setTimeout(() => {
			copied = false;
		}, 1800);
		toast.success(t.copiedToast, {
			description: `${sender.name || item.sender} · ${amountDisplay}`
		});
	}
</script>

<article
	class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4 transition-colors duration-150 {status ===
	'Dismissed'
		? 'opacity-75'
		: ''}"
>
	<div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
		<div class="min-w-0 flex-1 space-y-1">
			<div class="flex flex-wrap items-center gap-2">
				<h2 class="truncate font-display text-sm font-semibold text-foreground">
					{sender.name || item.sender}
				</h2>
				<span
					class="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-[11px] font-medium whitespace-nowrap {statusMeta.badgeClass}"
				>
					<statusMeta.icon class="h-3 w-3 shrink-0" />
					<span>{statusMeta.label}</span>
				</span>
			</div>

			<p class="truncate text-xs font-medium text-foreground">
				{item.subject}
			</p>

			<div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-secondary">
				<span class="font-mono text-[11px]">{sender.email || item.sender}</span>
				<span aria-hidden="true">·</span>
				<span>{accountLabel}</span>
			</div>
		</div>

		<div class="flex shrink-0 flex-row items-baseline justify-between gap-3 sm:flex-col sm:items-end sm:gap-1">
			<div class="font-display text-lg font-bold tabular-nums text-foreground">
				{amountDisplay}
			</div>
			<div class="flex items-center gap-2 font-mono text-xs tabular-nums">
				<span class="text-secondary">{item.due_date_iso}</span>
				<span aria-hidden="true" class="text-secondary">·</span>
				<span class="font-medium {statusMeta.textClass}">{relativeDelta}</span>
			</div>
		</div>
	</div>

	{#if item.digitable_line}
		<div
			class="flex flex-col justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2 sm:flex-row sm:items-center"
		>
			<code
				class="min-w-0 flex-1 break-all font-mono text-xs tabular-nums text-foreground select-all sm:truncate"
				title={item.digitable_line}
			>
				{item.digitable_line}
			</code>
			<button
				type="button"
				onclick={copyBarcode}
				aria-label={t.copyLine}
				class="flex min-h-[44px] shrink-0 items-center justify-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium whitespace-nowrap text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft focus-visible:outline-2 focus-visible:outline-accent active:scale-[0.98] md:min-h-[34px]"
			>
				{#if copied}
					<Check class="h-3.5 w-3.5 text-status-healthy" aria-hidden="true" />
					<span class="text-status-healthy">{t.copied}</span>
				{:else}
					<Copy class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{t.copyLine}</span>
				{/if}
			</button>
		</div>
	{/if}

	<div class="flex flex-wrap items-center justify-between gap-2 border-t border-separator pt-3 text-xs">
		<span class="font-mono text-[11px] tabular-nums text-secondary">
			{t.extractedOn(extractedDateIso)} · {t.localParser}
		</span>

		<div class="flex flex-wrap items-center gap-2">
			{#if !item.is_dismissed}
				<button
					type="button"
					onclick={() => onDismiss(item.message_id)}
					aria-label={t.dismissAria}
					class="flex min-h-[44px] items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium whitespace-nowrap text-secondary transition-colors hover:bg-field hover:text-foreground focus-visible:outline-2 focus-visible:outline-accent active:scale-[0.98] md:min-h-[34px]"
				>
					<XCircle class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t.dismiss}</span>
				</button>
			{/if}

			<button
				type="button"
				onclick={() => onViewEmail(item)}
				class="flex min-h-[44px] items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium whitespace-nowrap text-foreground transition-colors hover:bg-selection-soft focus-visible:outline-2 focus-visible:outline-accent active:scale-[0.98] md:min-h-[34px]"
			>
				<ExternalLink class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
				<span>{t.viewEmail}</span>
			</button>
		</div>
	</div>
</article>
