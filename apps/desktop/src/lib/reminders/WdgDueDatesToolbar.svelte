<script lang="ts">
	import { Tabs } from 'bits-ui';
	import Search from '@lucide/svelte/icons/search';
	import X from '@lucide/svelte/icons/x';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels } from './labels';
	import type { DueDateStatus } from './types';

	export type DueDateTabValue = 'All' | DueDateStatus;

	interface Props {
		activeTab: DueDateTabValue;
		selectedAccountId: string;
		searchQuery: string;
		tabCounts: Record<DueDateTabValue, number>;
		accounts: AccountConfig[];
	}

	let {
		activeTab = $bindable<DueDateTabValue>('All'),
		selectedAccountId = $bindable('all'),
		searchQuery = $bindable(''),
		tabCounts,
		accounts
	}: Props = $props();

	const t = $derived(labels[i18n.locale]);

	const statusTabs = $derived<{ value: DueDateTabValue; label: string }[]>([
		{ value: 'All', label: t.all },
		{ value: 'Overdue', label: t.status.Overdue },
		{ value: 'DueToday', label: t.status.DueToday },
		{ value: 'Upcoming', label: t.status.Upcoming },
		{ value: 'Dismissed', label: t.status.Dismissed }
	]);
</script>

<div
	class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-3 xl:flex-row xl:items-center xl:justify-between"
>
	<Tabs.Root value={activeTab} onValueChange={(val) => val && (activeTab = val as DueDateTabValue)}>
		<Tabs.List class="flex flex-wrap gap-1 rounded-md border border-separator bg-field p-1">
			{#each statusTabs as tab (tab.value)}
				<Tabs.Trigger
					value={tab.value}
					class="flex min-h-[40px] items-center gap-2 rounded px-3 py-1 text-xs font-medium whitespace-nowrap text-secondary transition-colors hover:text-foreground data-[state=active]:bg-surface data-[state=active]:text-foreground data-[state=active]:shadow-xs md:min-h-[32px]"
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
		{#if accounts.length > 1}
			<select
				bind:value={selectedAccountId}
				aria-label={t.filterAccount}
				class="h-10 min-w-0 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none md:h-9"
			>
				<option value="all">{t.allAccounts}</option>
				{#each accounts as account (account.account_id)}
					<option value={account.account_id}>{account.email_display}</option>
				{/each}
			</select>
		{/if}

		<div class="relative flex w-full min-w-0 flex-1 items-center sm:w-72 md:w-80 xl:flex-none">
			<Search class="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-secondary" aria-hidden="true" />
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
					class="absolute right-2 rounded p-1 text-secondary hover:text-foreground"
				>
					<X class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
			{/if}
		</div>
	</div>
</div>
