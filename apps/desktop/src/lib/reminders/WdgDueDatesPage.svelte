<script lang="ts">
	import { goto } from '$app/navigation';
	import { untrack } from 'svelte';
	import { Tabs } from 'bits-ui';
	import { Bell, CalendarClock, Search, ShieldAlert, X } from 'lucide-svelte';
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
		type DueDateStatus,
		type RemindersTransport,
		type ScheduledReminderConfig
	} from './types';
	import WdgBoletoCard from './WdgBoletoCard.svelte';
	import WdgDueDateKpis from './WdgDueDateKpis.svelte';
	import WdgReminderConfigDrawer from './WdgReminderConfigDrawer.svelte';

	type TabValue = 'All' | DueDateStatus;

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
	let activeTab = $state<TabValue>('All');
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

	const statusTabs = $derived<{ value: TabValue; label: string }[]>([
		{ value: 'All', label: t.all },
		{ value: 'Overdue', label: t.status.Overdue },
		{ value: 'DueToday', label: t.status.DueToday },
		{ value: 'Upcoming', label: t.status.Upcoming },
		{ value: 'Dismissed', label: t.status.Dismissed }
	]);

	const enabledRulesCount = $derived(configs.filter((c) => c.enabled).length);

	const tabCounts = $derived.by(() => {
		const counts: Record<TabValue, number> = {
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
		<header class="flex flex-wrap items-start justify-between gap-4 border-b border-separator pb-4">
			<div class="space-y-1">
				<h1 class="font-display text-lg font-semibold text-foreground">{t.title}</h1>
				<p class="text-xs text-secondary">{t.subtitle}</p>
			</div>

			<button
				type="button"
				onclick={() => (configOpen = true)}
				class="flex min-h-[44px] shrink-0 items-center gap-2 rounded-md border border-separator bg-surface px-3.5 py-2 text-xs font-medium whitespace-nowrap text-foreground transition-colors hover:bg-selection-soft focus-visible:outline-2 focus-visible:outline-accent active:scale-[0.98] md:min-h-[36px]"
			>
				<Bell class="h-3.5 w-3.5 text-accent" />
				<span>{t.configureSchedule}</span>
				<span class="font-mono text-[11px] tabular-nums text-secondary">
					({enabledRulesCount > 0 ? t.activeRulesCount(enabledRulesCount) : t.paused})
				</span>
			</button>
		</header>

		<div
			role="note"
			aria-label={t.disclaimerTitle}
			class="flex items-start gap-3 rounded-lg border border-status-warning/35 bg-status-warning/10 px-3.5 py-3 text-xs text-foreground"
		>
			<ShieldAlert class="mt-0.5 h-4 w-4 shrink-0 text-status-warning" />
			<p class="leading-relaxed">
				<strong class="font-semibold">{t.disclaimerTitle}</strong>
				{t.disclaimer}
			</p>
		</div>

		{#if error}
			<div
				role="alert"
				class="rounded-md border border-status-danger/30 bg-status-danger/10 px-3.5 py-2.5 text-xs text-status-danger"
			>
				{error}
			</div>
		{/if}

		<WdgDueDateKpis {items} activeStatus={activeTab} onSelectStatus={(s) => (activeTab = s)} />

		<div
			class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-3 xl:flex-row xl:items-center xl:justify-between"
		>
			<Tabs.Root value={activeTab} onValueChange={(val) => val && (activeTab = val as TabValue)}>
				<Tabs.List class="flex flex-wrap gap-1 rounded-md border border-separator bg-field p-1">
					{#each statusTabs as tab (tab.value)}
						<Tabs.Trigger
							value={tab.value}
							class="flex min-h-[40px] items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium whitespace-nowrap text-secondary transition-colors hover:text-foreground data-[state=active]:bg-surface data-[state=active]:text-foreground data-[state=active]:shadow-xs md:min-h-[32px]"
						>
							<span>{tab.label}</span>
							<span class="font-mono text-[11px] tabular-nums text-secondary">
								{tabCounts[tab.value]}
							</span>
						</Tabs.Trigger>
					{/each}
				</Tabs.List>
			</Tabs.Root>

			<div class="flex w-full min-w-0 flex-col gap-2 sm:flex-row sm:items-center xl:w-auto">
				{#if resolvedAccounts.length > 1}
					<select
						bind:value={selectedAccountId}
						aria-label={t.filterAccount}
						class="h-10 min-w-0 rounded-md border border-separator bg-field px-2.5 text-xs text-foreground focus:border-accent focus:outline-none md:h-9"
					>
						<option value="all">{t.allAccounts}</option>
						{#each resolvedAccounts as account (account.account_id)}
							<option value={account.account_id}>{account.email_display}</option>
						{/each}
					</select>
				{/if}

				<div class="relative flex w-full min-w-0 flex-1 items-center sm:w-72 md:w-80 xl:flex-none">
					<Search class="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-secondary" />
					<input
						type="search"
						bind:value={searchQuery}
						placeholder={t.searchPlaceholder}
						aria-label={t.searchPlaceholder}
						class="h-10 w-full rounded-md border border-separator bg-field pr-8 pl-8 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none md:h-9"
					/>
					{#if searchQuery}
						<button
							type="button"
							onclick={() => (searchQuery = '')}
							aria-label={t.clearSearch}
							class="absolute right-2 rounded p-0.5 text-secondary hover:text-foreground"
						>
							<X class="h-3.5 w-3.5" />
						</button>
					{/if}
				</div>
			</div>
		</div>

		{#if loading}
			<div class="flex h-48 items-center justify-center rounded-lg border border-separator bg-surface text-xs text-secondary">
				{t.loadingBills}
			</div>
		{:else if filteredItems.length === 0}
			<div
				class="flex flex-col items-center justify-center gap-3 rounded-lg border border-separator bg-surface px-6 py-12 text-center"
			>
				<CalendarClock class="h-8 w-8 text-secondary" />
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
						class="min-h-[44px] rounded-md border border-separator bg-field px-4 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft md:min-h-[36px]"
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
