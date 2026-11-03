<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import Icon from '../inbox/Icon.svelte';
	import type { AccountConfig } from '../onboarding/types';
	import { labels } from './labels';
	import {
		calculateDueDateStatus,
		type DueDateItem,
		type DueDateStatus,
		type RemindersTransport
	} from './types';
	import { nativeRemindersTransport } from './transport';

	let {
		open = true,
		close,
		onSelectMessage,
		onConfigureReminders,
		transport = nativeRemindersTransport,
		accounts = []
	}: {
		open?: boolean;
		close: () => void;
		onSelectMessage?: (messageId: string) => void;
		onConfigureReminders?: () => void;
		transport?: RemindersTransport;
		accounts?: AccountConfig[];
	} = $props();

	const t = $derived(labels[i18n.locale]);

	let items = $state<DueDateItem[]>([]);
	let loading = $state(false);
	let error = $state('');
	let statusFilter = $state<'All' | DueDateStatus>('All');
	let accountFilter = $state<string>('All');
	let searchQuery = $state('');
	let copiedId = $state<string | null>(null);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;
	let active = true;

	async function load() {
		loading = true;
		error = '';
		try {
			const res = await transport.listDueDates(
				accountFilter === 'All' ? undefined : accountFilter
			);
			if (active) {
				items = res;
			}
		} catch {
			if (active) error = t.error;
		} finally {
			if (active) loading = false;
		}
	}

	$effect(() => {
		if (open) {
			untrack(() => void load());
		}
	});

	$effect(() => () => {
		active = false;
		if (copyTimer) clearTimeout(copyTimer);
	});

	function formatBrl(cents: number | null): string {
		if (cents === null || cents === undefined) return t.noAmount;
		return new Intl.NumberFormat(i18n.locale, {
			style: 'currency',
			currency: 'BRL'
		}).format(cents / 100);
	}

	function getRelativeDueLabel(dueDateIso: string, status: DueDateStatus): string {
		const today = new Date().toISOString().slice(0, 10);
		const dueTime = new Date(dueDateIso + 'T00:00:00Z').getTime();
		const todayTime = new Date(today + 'T00:00:00Z').getTime();
		const diffDays = Math.round((dueTime - todayTime) / 86400000);

		if (status === 'DueToday' || diffDays === 0) return t.today;
		if (diffDays < 0) return t.daysOverdue(Math.abs(diffDays));
		return t.daysRemaining(diffDays);
	}

	async function copyDigitableLine(id: string, line: string) {
		try {
			await navigator.clipboard.writeText(line);
			copiedId = id;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => {
				copiedId = null;
			}, 2000);
		} catch {
			// Clipboard permission fallback
		}
	}

	async function handleDismiss(messageId: string) {
		try {
			await transport.dismissDueDate(messageId);
			items = items.map((it) => (it.message_id === messageId ? { ...it, is_dismissed: true } : it));
		} catch {
			error = t.error;
		}
	}

	const metrics = $derived.by(() => {
		let overdueCount = 0;
		let overdueCents = 0;
		let dueTodayCount = 0;
		let dueTodayCents = 0;
		let upcomingCount = 0;
		let upcomingCents = 0;

		for (const it of items) {
			if (it.is_dismissed) continue;
			const st = calculateDueDateStatus(it);
			const amt = it.amount_cents ?? 0;
			if (st === 'Overdue') {
				overdueCount++;
				overdueCents += amt;
			} else if (st === 'DueToday') {
				dueTodayCount++;
				dueTodayCents += amt;
			} else if (st === 'Upcoming') {
				upcomingCount++;
				upcomingCents += amt;
			}
		}

		return {
			overdueCount,
			overdueCents,
			dueTodayCount,
			dueTodayCents,
			upcomingCount,
			upcomingCents
		};
	});

	const filteredItems = $derived.by(() => {
		const query = searchQuery.trim().toLowerCase();
		return items.filter((it) => {
			const st = calculateDueDateStatus(it);
			if (statusFilter !== 'All' && st !== statusFilter) return false;
			if (accountFilter !== 'All' && it.account_id !== accountFilter) return false;
			if (query.length > 0) {
				const matchSubject = it.subject.toLowerCase().includes(query);
				const matchSender = it.sender.toLowerCase().includes(query);
				const matchBarcode = it.digitable_line.toLowerCase().includes(query);
				if (!matchSubject && !matchSender && !matchBarcode) return false;
			}
			return true;
		});
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) {
			close();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4"
		onclick={(event) => {
			if (event.target === event.currentTarget) close();
		}}
		role="presentation"
	>
		<div
			role="dialog"
			aria-modal="true"
			aria-labelledby="due-date-dialog-title"
			class="relative flex h-[85vh] max-h-[820px] w-full max-w-5xl flex-col rounded-xl border border-[var(--separator)] bg-[var(--reader)] shadow-2xl overflow-hidden text-[var(--text)]"
		>
			<!-- Header -->
			<header
				class="flex items-center justify-between border-b border-[var(--separator)] px-6 py-4 shrink-0 bg-[var(--sidebar)]"
			>
				<div>
					<div class="flex items-center gap-2.5">
						<Icon name="calendar" size={22} />
						<h1 id="due-date-dialog-title" class="text-xl font-semibold tracking-tight">
							{t.title}
						</h1>
					</div>
					<p class="text-xs text-[var(--secondary)] mt-0.5">{t.subtitle}</p>
				</div>
				<div class="flex items-center gap-3">
					{#if onConfigureReminders}
						<button
							type="button"
							onclick={onConfigureReminders}
							class="flex items-center gap-1.5 rounded-lg border border-[var(--separator)] px-3 py-1.5 text-xs font-medium text-[var(--text)] hover:bg-[var(--surface)] transition-colors"
						>
							<Icon name="bell" size={14} />
							{t.configureReminders}
						</button>
					{/if}
					<button
						type="button"
						aria-label={t.close}
						onclick={close}
						class="rounded-lg p-1.5 text-[var(--secondary)] hover:bg-[var(--surface)] hover:text-[var(--text)] transition-colors"
					>
						✕
					</button>
				</div>
			</header>

			<!-- Content Scroll Area -->
			<div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-6 space-y-6">
				<!-- Non-Payment Disclaimer Banner -->
				<div
					class="flex items-start gap-3 rounded-lg border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-xs text-amber-800 dark:text-amber-200"
					role="note"
				>
					<div class="mt-0.5 shrink-0 text-amber-600 dark:text-amber-400">
						<Icon name="shield" size={16} />
					</div>
					<div class="leading-relaxed font-medium">
						{t.disclaimer}
					</div>
				</div>

				<!-- Metrics Cards Grid -->
				<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
					<!-- Overdue Card -->
					<div
						class="flex flex-col justify-between rounded-xl border border-red-500/20 bg-red-500/5 p-4"
					>
						<div class="flex items-center justify-between">
							<span class="text-xs font-semibold uppercase tracking-wider text-red-600 dark:text-red-400">
								{t.overdue}
							</span>
							<span
								class="rounded-full bg-red-500/15 px-2 py-0.5 text-xs font-bold text-red-700 dark:text-red-300"
							>
								{metrics.overdueCount}
							</span>
						</div>
						<div class="mt-3 text-lg font-bold text-red-700 dark:text-red-300">
							{metrics.overdueCents > 0 ? formatBrl(metrics.overdueCents) : '—'}
						</div>
					</div>

					<!-- Due Today Card -->
					<div
						class="flex flex-col justify-between rounded-xl border border-amber-500/20 bg-amber-500/5 p-4"
					>
						<div class="flex items-center justify-between">
							<span
								class="text-xs font-semibold uppercase tracking-wider text-amber-600 dark:text-amber-400"
							>
								{t.dueToday}
							</span>
							<span
								class="rounded-full bg-amber-500/15 px-2 py-0.5 text-xs font-bold text-amber-700 dark:text-amber-300"
							>
								{metrics.dueTodayCount}
							</span>
						</div>
						<div class="mt-3 text-lg font-bold text-amber-700 dark:text-amber-300">
							{metrics.dueTodayCents > 0 ? formatBrl(metrics.dueTodayCents) : '—'}
						</div>
					</div>

					<!-- Upcoming Card -->
					<div
						class="flex flex-col justify-between rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-4"
					>
						<div class="flex items-center justify-between">
							<span
								class="text-xs font-semibold uppercase tracking-wider text-emerald-600 dark:text-emerald-400"
							>
								{t.upcoming}
							</span>
							<span
								class="rounded-full bg-emerald-500/15 px-2 py-0.5 text-xs font-bold text-emerald-700 dark:text-emerald-300"
							>
								{metrics.upcomingCount}
							</span>
						</div>
						<div class="mt-3 text-lg font-bold text-emerald-700 dark:text-emerald-300">
							{metrics.upcomingCents > 0 ? formatBrl(metrics.upcomingCents) : '—'}
						</div>
					</div>
				</div>

				<!-- Filter & Search Toolbar -->
				<div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between pt-1">
					<!-- Status Filter Tabs -->
					<div class="flex flex-wrap gap-1 rounded-lg bg-[var(--surface)] p-1">
						{#each (['All', 'Overdue', 'DueToday', 'Upcoming', 'Dismissed'] as const) as tab}
							<button
								type="button"
								class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors"
								class:bg-[var(--reader)]={statusFilter === tab}
								class:text-[var(--text)]={statusFilter === tab}
								class:shadow-xs={statusFilter === tab}
								class:text-[var(--secondary)]={statusFilter !== tab}
								onclick={() => (statusFilter = tab)}
							>
								{tab === 'All' ? t.all : t.status[tab]}
							</button>
						{/each}
					</div>

					<!-- Account Filter & Search Input -->
					<div class="flex items-center gap-2">
						{#if accounts.length > 1}
							<select
								bind:value={accountFilter}
								aria-label={t.filterAccount}
								class="rounded-lg border border-[var(--separator)] bg-[var(--surface)] px-2.5 py-1.5 text-xs text-[var(--text)] focus:outline-hidden"
							>
								<option value="All">{t.allAccounts}</option>
								{#each accounts as acc}
									<option value={acc.account_id}>{acc.email_display}</option>
								{/each}
							</select>
						{/if}

						<div class="relative">
							<input
								type="search"
								bind:value={searchQuery}
								placeholder={t.searchPlaceholder}
								class="w-48 sm:w-64 rounded-lg border border-[var(--separator)] bg-[var(--surface)] px-3 py-1.5 text-xs text-[var(--text)] placeholder:text-[var(--secondary)] focus:outline-hidden"
							/>
						</div>
					</div>
				</div>

				<!-- Items List -->
				{#if loading}
					<div class="flex h-48 items-center justify-center text-sm text-[var(--secondary)]">
						Loading bills and due dates…
					</div>
				{:else if filteredItems.length === 0}
					<div
						class="flex flex-col items-center justify-center rounded-xl border border-dashed border-[var(--separator)] py-12 text-center"
					>
						<div class="mb-2 text-[var(--secondary)]">
							<Icon name="calendar" size={32} />
						</div>
						<h3 class="text-sm font-semibold text-[var(--text)]">{t.emptyState}</h3>
						<p class="mt-1 max-w-sm text-xs text-[var(--secondary)]">{t.emptyStateHelp}</p>
					</div>
				{:else}
					<div class="space-y-3">
						{#each filteredItems as item (item.message_id)}
							{@const st = calculateDueDateStatus(item)}
							<div
								class="flex flex-col gap-3 rounded-xl border border-[var(--separator)] bg-[var(--surface)] p-4 transition-all hover:border-[var(--secondary)]/40"
							>
								<!-- Top row: Amount + Status badge + Due Date -->
								<div class="flex flex-wrap items-center justify-between gap-2">
									<div class="flex items-baseline gap-2.5">
										<span class="text-lg font-bold tracking-tight text-[var(--text)]">
											{formatBrl(item.amount_cents)}
										</span>
										<span
											class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {st === 'Overdue' ? 'bg-red-500/15 text-red-700 dark:text-red-300' : st === 'DueToday' ? 'bg-amber-500/15 text-amber-700 dark:text-amber-300' : st === 'Upcoming' ? 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-300' : 'bg-zinc-500/15 text-zinc-600 dark:text-zinc-400'}"
										>
											{t.status[st]}
										</span>
									</div>
									<div class="flex items-center gap-2 text-xs text-[var(--secondary)]">
										<span>{t.dueOn} <strong>{item.due_date_iso}</strong></span>
										<span>•</span>
										<span>{getRelativeDueLabel(item.due_date_iso, st)}</span>
									</div>
								</div>

								<!-- Middle row: Sender and Subject -->
								<div class="flex flex-col text-xs">
									<span class="font-medium text-[var(--text)]">{item.sender}</span>
									<span class="truncate text-[var(--secondary)]">{item.subject}</span>
								</div>

								<!-- Digitable Line (Linha Digitável / Barcode) -->
								{#if item.digitable_line}
									<div
										class="flex items-center justify-between gap-2 rounded-lg bg-[var(--reader)] border border-[var(--separator)] px-3 py-2 text-xs"
									>
										<code
											class="font-mono text-[11px] tracking-wide text-[var(--text)] select-all truncate"
										>
											{item.digitable_line}
										</code>
										<button
											type="button"
											onclick={() => copyDigitableLine(item.message_id, item.digitable_line)}
											class="flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors"
											class:bg-emerald-600={copiedId === item.message_id}
											class:text-white={copiedId === item.message_id}
											class:bg-[var(--surface)]={copiedId !== item.message_id}
											class:text-[var(--text)]={copiedId !== item.message_id}
											class:hover:bg-[var(--selection-soft)]={copiedId !== item.message_id}
											aria-label={t.copyLine}
										>
											<Icon
												name={copiedId === item.message_id ? 'check' : 'copy'}
												size={13}
											/>
											{copiedId === item.message_id ? t.copied : t.copyLine}
										</button>
									</div>
								{/if}

								<!-- Bottom action buttons -->
								<div class="flex items-center justify-end gap-2 pt-1 border-t border-[var(--separator)]/40">
									{#if !item.is_dismissed}
										<button
											type="button"
											onclick={() => handleDismiss(item.message_id)}
											class="rounded-lg px-2.5 py-1 text-xs text-[var(--secondary)] hover:text-red-500 hover:bg-red-500/10 transition-colors"
											aria-label={t.dismissAria}
										>
											{t.dismiss}
										</button>
									{/if}

									{#if onSelectMessage}
										<button
											type="button"
											onclick={() => {
												close();
												onSelectMessage(item.message_id);
											}}
											class="rounded-lg bg-[var(--surface)] border border-[var(--separator)] px-3 py-1 text-xs font-medium text-[var(--text)] hover:bg-[var(--selection-soft)] transition-colors"
										>
											{t.viewEmail}
										</button>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
