<script lang="ts">
	import { goto } from '$app/navigation';
	import { untrack } from 'svelte';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import { toast } from 'svelte-sonner';
	import { nativeAlertsTransport } from '$lib/alerts/transport';
	import type { AlertsTransport } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { onboardingSession } from '$lib/onboarding/session.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { appStore } from '$lib/stores/app.svelte';
	import { demoRemindersBackend } from './due-dates.fixtures';
	import { labels } from './labels';
	import { nativeRemindersTransport } from './transport';
	import {
		calculateDueDateStatus,
		countOverdueAndDueToday,
		type DueDateItem,
		type RemindersTransport,
		type ScheduledReminderConfig
	} from './types';
	import WdgBoletoCard from './WdgBoletoCard.svelte';
	import WdgDueDateKpis from './WdgDueDateKpis.svelte';
	import WdgDueDatesHeader from './WdgDueDatesHeader.svelte';
	import WdgDueDatesToolbar, { type DueDateTabValue } from './WdgDueDatesToolbar.svelte';
	import WdgReminderConfigDrawer from './WdgReminderConfigDrawer.svelte';

	interface Props {
		remindersTransport?: RemindersTransport;
		alertsTransport?: AlertsTransport;
		accounts?: AccountConfig[];
		onNavigateToInbox?: (messageId: string) => void | Promise<void>;
	}

	let {
		remindersTransport = nativeAvailable()
			? nativeRemindersTransport
			: demoRemindersBackend.remindersTransport,
		alertsTransport = nativeAvailable()
			? nativeAlertsTransport
			: demoRemindersBackend.alertsTransport,
		accounts,
		onNavigateToInbox
	}: Props = $props();

	const t = $derived(labels[i18n.locale]);

	let items = $state<DueDateItem[]>([]);
	let configs = $state<ScheduledReminderConfig[]>([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab = $state<DueDateTabValue>('All');
	let selectedAccountId = $state<string>('all');
	let searchQuery = $state('');
	let configOpen = $state(false);
	let loadGen = 0;

	const resolvedAccounts = $derived.by<AccountConfig[]>(() => {
		if (accounts) return accounts;
		if (nativeAvailable()) return onboardingSession.accounts.accounts;
		return appStore.accounts.map((a) => ({
			account_id: a.account_id,
			email_display: a.address,
			imap_host: a.imap_host,
			imap_port: a.imap_port,
			auth_method: a.auth_method === 'OAuth2-PKCE' ? 'OAuth2PublicPkce' : 'AppPassword'
		}));
	});

	const enabledRulesCount = $derived(configs.filter((c) => c.enabled).length);

	const tabCounts = $derived.by(() => {
		const counts: Record<DueDateTabValue, number> = {
			All: items.length,
			Overdue: 0,
			DueToday: 0,
			Upcoming: 0,
			Dismissed: 0
		};
		for (const item of items) {
			counts[calculateDueDateStatus(item)] += 1;
		}
		return counts;
	});

	const filteredItems = $derived.by(() => {
		const q = searchQuery.trim().toLowerCase();
		const qDigits = q.replace(/\D/g, '');

		return items
			.filter((item) => {
				const st = calculateDueDateStatus(item);
				if (activeTab !== 'All' && st !== activeTab) return false;
				if (selectedAccountId !== 'all' && item.account_id !== selectedAccountId) return false;
				if (!q) return true;
				const haystack = `${item.sender} ${item.subject} ${item.digitable_line}`.toLowerCase();
				return (
					haystack.includes(q) ||
					(qDigits.length >= 3 && item.digitable_line.replace(/\D/g, '').includes(qDigits))
				);
			})
			.sort((a, b) => {
				if (a.is_dismissed !== b.is_dismissed) return a.is_dismissed ? 1 : -1;
				return a.due_date_iso.localeCompare(b.due_date_iso);
			});
	});

	async function loadHub() {
		const gen = ++loadGen;
		loading = true;
		error = '';
		try {
			const [dueList, cfgList] = await Promise.all([
				remindersTransport.listDueDates(),
				remindersTransport.listConfigs()
			]);
			if (gen !== loadGen) return;
			items = dueList;
			configs = cfgList;
			appStore.overdueAndDueTodayCount = countOverdueAndDueToday(dueList);
		} catch {
			if (gen === loadGen) error = t.error;
		} finally {
			if (gen === loadGen) loading = false;
		}
	}

	$effect(() => {
		untrack(() => {
			if (nativeAvailable() && !accounts && !onboardingSession.booted) {
				void onboardingSession.boot();
			}
			void loadHub();
		});
		return () => {
			loadGen++;
		};
	});

	function accountLabelFor(accountId: string): string {
		const fromShell = appStore.accounts.find((a) => a.account_id === accountId);
		if (fromShell) return fromShell.display_name;
		const fromConfig = resolvedAccounts.find((a) => a.account_id === accountId);
		return fromConfig?.email_display ?? accountId;
	}

	async function handleDismiss(messageId: string) {
		try {
			await remindersTransport.dismissDueDate(messageId);
			items = items.map((it) =>
				it.message_id === messageId ? { ...it, is_dismissed: true } : it
			);
			appStore.overdueAndDueTodayCount = countOverdueAndDueToday(items);
			toast.success(t.dismissedToast);
		} catch {
			error = t.error;
			toast.error(t.error);
		}
	}

	async function handleViewEmail(item: DueDateItem) {
		appStore.setScope({ kind: 'all' });
		appStore.requestSelectMessage(item.message_id);
		if (onNavigateToInbox) {
			await onNavigateToInbox(item.message_id);
			return;
		}
		try {
			await goto('/');
		} catch {
			// Unit tests mount WdgDueDatesPage without booting SvelteKit's client router.
		}
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-4 md:p-6">
	<div class="mx-auto flex w-full max-w-5xl flex-col gap-5">
		<WdgDueDatesHeader {enabledRulesCount} onOpenConfig={() => (configOpen = true)} />

		{#if error}
			<div
				role="alert"
				class="rounded-md border border-status-danger/30 bg-status-danger/10 px-4 py-3 text-xs text-status-danger"
			>
				{error}
			</div>
		{/if}

		<WdgDueDateKpis {items} activeStatus={activeTab} onSelectStatus={(s) => (activeTab = s)} />

		<WdgDueDatesToolbar
			bind:activeTab
			bind:selectedAccountId
			bind:searchQuery
			{tabCounts}
			accounts={resolvedAccounts}
		/>

		{#if loading}
			<div class="flex h-48 items-center justify-center rounded-lg border border-separator bg-surface text-xs text-secondary">
				{t.loadingBills}
			</div>
		{:else if filteredItems.length === 0}
			<div
				class="flex flex-col items-center justify-center gap-3 rounded-lg border border-separator bg-surface px-6 py-12 text-center"
			>
				<CalendarClock class="h-8 w-8 text-secondary" aria-hidden="true" />
				<div class="space-y-1">
					<h2 class="font-display text-sm font-semibold text-foreground">{t.emptyState}</h2>
					<p class="max-w-sm text-xs text-secondary">{t.emptyStateHelp}</p>
				</div>
				{#if activeTab !== 'All' || selectedAccountId !== 'all' || searchQuery}
					<button
						type="button"
						onclick={() => {
							activeTab = 'All';
							selectedAccountId = 'all';
							searchQuery = '';
						}}
						class="min-h-[44px] rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-foreground hover:bg-selection-soft md:min-h-[36px]"
					>
						{t.clearFilters}
					</button>
				{/if}
			</div>
		{:else}
			<div class="flex flex-col gap-3" aria-label={t.ledgerAria}>
				{#each filteredItems as item (item.message_id)}
					<WdgBoletoCard
						{item}
						accountLabel={accountLabelFor(item.account_id)}
						onDismiss={handleDismiss}
						onViewEmail={handleViewEmail}
					/>
				{/each}
			</div>
		{/if}
	</div>
</div>

<WdgReminderConfigDrawer
	bind:open={configOpen}
	{remindersTransport}
	{alertsTransport}
	accounts={resolvedAccounts}
	onConfigsChanged={(next) => (configs = next)}
/>
