<script lang="ts">
	import { Tabs } from 'bits-ui';
	import AlertsModule from '$lib/alerts/AlertsModule.svelte';
	import CategoriesModule from '$lib/categories/CategoriesModule.svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import RulesModule from '$lib/rules/RulesModule.svelte';
	import ReviewModule from '$lib/sender-review/ReviewModule.svelte';
	import { SettingsFormGuard } from '$lib/settings/guards';

	let activeTab = $state('rules');
	const guard = new SettingsFormGuard();
	const tabLabels = $derived(i18n.t.settingsModal.tabs);
</script>

<svelte:head>
	<title>Automation · triage-mail</title>
</svelte:head>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-y-auto bg-background p-4 sm:p-6">
	<Tabs.Root bind:value={activeTab} class="flex min-h-0 flex-1 flex-col gap-4">
		<Tabs.List class="flex flex-wrap gap-1 border-b border-separator pb-2">
			<Tabs.Trigger
				value="rules"
				class="rounded-md px-3 py-1.5 text-xs font-medium text-secondary data-[state=active]:bg-selection data-[state=active]:text-foreground"
			>
				{tabLabels.rules}
			</Tabs.Trigger>
			<Tabs.Trigger
				value="alerts"
				class="rounded-md px-3 py-1.5 text-xs font-medium text-secondary data-[state=active]:bg-selection data-[state=active]:text-foreground"
			>
				{tabLabels.alerts}
			</Tabs.Trigger>
			<Tabs.Trigger
				value="review"
				class="rounded-md px-3 py-1.5 text-xs font-medium text-secondary data-[state=active]:bg-selection data-[state=active]:text-foreground"
			>
				{tabLabels.senderReview}
			</Tabs.Trigger>
			<Tabs.Trigger
				value="categories"
				class="rounded-md px-3 py-1.5 text-xs font-medium text-secondary data-[state=active]:bg-selection data-[state=active]:text-foreground"
			>
				{tabLabels.categories}
			</Tabs.Trigger>
		</Tabs.List>

		<Tabs.Content value="rules" class="min-h-0 flex-1">
			<RulesModule accounts={[]} {guard} />
		</Tabs.Content>
		<Tabs.Content value="alerts" class="min-h-0 flex-1">
			<AlertsModule {guard} />
		</Tabs.Content>
		<Tabs.Content value="review" class="min-h-0 flex-1">
			<ReviewModule accounts={[]} {guard} />
		</Tabs.Content>
		<Tabs.Content value="categories" class="min-h-0 flex-1">
			<CategoriesModule accounts={[]} {guard} />
		</Tabs.Content>
	</Tabs.Root>
</div>
