<script lang="ts">
	import { untrack } from 'svelte';
	import { Tabs } from 'bits-ui';
	import {
		nativeScheduleTransport,
		type ScheduleTransport
	} from '$lib/alerts/schedule';
	import { nativeAlertsTransport } from '$lib/alerts/transport';
	import type { AlertsTransport, Destination } from '$lib/alerts/types';
	import { nativeCategories } from '$lib/categories/transport';
	import type { CategoryTransport } from '$lib/categories/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { parseSender, workspaceRegistry } from '$lib/inbox/workspace.svelte';
	import { onboardingSession } from '$lib/onboarding/session.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { nativeRulesTransport } from '$lib/rules/transport';
	import type { Rule, RulesTransport } from '$lib/rules/types';
	import { nativeReview } from '$lib/sender-review/transport';
	import type { ReviewTransport } from '$lib/sender-review/types';
	import { SettingsFormGuard } from '$lib/settings/guards';
	import { appStore } from '$lib/stores/app.svelte';
	import {
		demoAutomationBackend,
		type HistoryVisualMeta,
		type OverrideVisualMeta,
		type RuleVisualMeta,
		type SenderVisualMeta
	} from './automation.fixtures';
	import { automationLabels } from './labels';
	import WdgAlertsManager from './WdgAlertsManager.svelte';
	import WdgCategoryOverrides from './WdgCategoryOverrides.svelte';
	import WdgRulesSplitPane from './WdgRulesSplitPane.svelte';
	import type { SimulatedMessageSummary } from './WdgRuleSimulatorPanel.svelte';
	import WdgSenderReviewQueue from './WdgSenderReviewQueue.svelte';

	type AutomationTab = 'rules' | 'alerts' | 'governance';

	interface Props {
		rulesTransport?: RulesTransport;
		alertsTransport?: AlertsTransport;
		scheduleTransport?: ScheduleTransport;
		reviewTransport?: ReviewTransport;
		categoriesTransport?: CategoryTransport;
		accounts?: AccountConfig[];
		selectedIds?: string[];
		seedMessageId?: string;
		initialTab?: AutomationTab;
		guard?: SettingsFormGuard;
		ruleMeta?: Record<string, RuleVisualMeta>;
		historyMeta?: Record<string, HistoryVisualMeta>;
		senderMeta?: Record<string, SenderVisualMeta>;
		overrideMeta?: Record<string, OverrideVisualMeta>;
	}

	const isDemo = !nativeAvailable();

	let {
		rulesTransport = isDemo ? demoAutomationBackend.rulesTransport : nativeRulesTransport,
		alertsTransport = isDemo ? demoAutomationBackend.alertsTransport : nativeAlertsTransport,
		scheduleTransport = isDemo ? demoAutomationBackend.scheduleTransport : nativeScheduleTransport,
		reviewTransport = isDemo ? demoAutomationBackend.reviewTransport : nativeReview,
		categoriesTransport = isDemo ? demoAutomationBackend.categoriesTransport : nativeCategories,
		accounts,
		selectedIds,
		seedMessageId,
		initialTab = 'rules',
		guard = new SettingsFormGuard(),
		ruleMeta = isDemo ? demoAutomationBackend.ruleMeta : {},
		historyMeta = isDemo ? demoAutomationBackend.historyMeta : {},
		senderMeta = isDemo ? demoAutomationBackend.senderMeta : {},
		overrideMeta = isDemo ? demoAutomationBackend.overrideMeta : {}
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale]);

	let activeTab = $state<AutomationTab>(untrack(() => initialTab));
	const resolvedSeedMessageId = $state<string | undefined>(
		untrack(() => seedMessageId ?? appStore.consumePendingRuleSeedMessage() ?? undefined)
	);
	let rulesList = $state<Rule[]>([]);
	let destinationsList = $state<Destination[]>([]);
	let queuedSendersCount = $state(0);

	const resolvedAccounts = $derived.by<AccountConfig[]>(() => {
		if (accounts) return accounts;
		if (nativeAvailable()) return onboardingSession.accounts.accounts;
		if (appStore.accounts.length > 0 && isDemo) {
			return demoAutomationBackend.accounts;
		}
		return demoAutomationBackend.accounts;
	});

	const resolvedSelectedIds = $derived.by<string[]>(() => {
		if (selectedIds) return selectedIds;
		const activeWorkspace = workspaceRegistry.active;
		if (activeWorkspace?.inbox.selectedId) {
			return [activeWorkspace.inbox.selectedId];
		}
		if (isDemo) {
			return demoAutomationBackend.sampleMessages.map((m) => m.id);
		}
		return [];
	});

	const activeRulesCount = $derived(rulesList.filter((r) => r.enabled).length);

	function resolveMessageSummary(id: string): SimulatedMessageSummary | undefined {
		const row = workspaceRegistry.active?.inbox.rows.find((r) => r.id === id);
		if (row) {
			const parsed = parseSender(row.sender);
			return {
				subject: row.subject,
				sender: parsed.email || row.sender,
				account_id: row.account_id
			};
		}
		if (isDemo) {
			const sample = demoAutomationBackend.sampleMessages.find((s) => s.id === id);
			if (sample) {
				return {
					subject: sample.subject,
					sender: sample.sender_email,
					account_id: sample.account_id
				};
			}
		}
		return undefined;
	}

	$effect(() => {
		let mounted = true;
		untrack(() => {
			if (nativeAvailable() && !accounts && !onboardingSession.booted) {
				void onboardingSession.boot();
			}
			void Promise.all([
				rulesTransport.list().catch(() => [] as Rule[]),
				alertsTransport.list().catch(() => [] as Destination[]),
				resolvedAccounts[0]?.account_id
					? reviewTransport.list(resolvedAccounts[0].account_id, 0).catch(() => [])
					: Promise.resolve([])
			]).then(([rList, dList, revList]) => {
				if (!mounted) return;
				if (rulesList.length === 0) rulesList = rList;
				if (destinationsList.length === 0) destinationsList = dList;
				if (queuedSendersCount === 0) {
					queuedSendersCount = revList.filter(
						(c) => c.state === 'Pending' || c.state === 'Declined'
					).length;
				}
			});
		});
		return () => {
			mounted = false;
			guard.reset();
		};
	});
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-4 md:p-6">
	<div class="mx-auto flex w-full max-w-6xl flex-col gap-5">
		<header class="flex flex-wrap items-start justify-between gap-4 border-b border-separator pb-4">
			<div class="space-y-1">
				<h1 class="font-display text-lg font-semibold text-foreground">{t.pageTitle}</h1>
				<p class="text-xs text-secondary">{t.pageSubtitle}</p>
			</div>
			<div class="flex flex-wrap items-center gap-2 font-mono text-xs tabular-nums text-secondary">
				<span>{t.rulesActiveMetric(activeRulesCount, rulesList.length)}</span>
				<span aria-hidden="true">·</span>
				<span>{t.destinationsMetric(destinationsList.length)}</span>
				<span aria-hidden="true">·</span>
				<span>{t.queuedSendersMetric(queuedSendersCount)}</span>
			</div>
		</header>

		<Tabs.Root bind:value={activeTab} class="flex flex-col gap-4">
			<Tabs.List
				class="flex w-full flex-wrap gap-1 rounded-lg border border-separator bg-surface p-1 sm:w-fit"
			>
				<Tabs.Trigger
					value="rules"
					class="min-h-[44px] rounded-md px-3 py-2 text-xs font-medium whitespace-nowrap text-secondary transition-colors hover:text-foreground data-[state=active]:bg-selection data-[state=active]:text-foreground md:min-h-[36px]"
				>
					{t.tabs.rules} ({rulesList.length})
				</Tabs.Trigger>
				<Tabs.Trigger
					value="alerts"
					class="min-h-[44px] rounded-md px-3 py-2 text-xs font-medium whitespace-nowrap text-secondary transition-colors hover:text-foreground data-[state=active]:bg-selection data-[state=active]:text-foreground md:min-h-[36px]"
				>
					{t.tabs.alerts} ({destinationsList.length})
				</Tabs.Trigger>
				<Tabs.Trigger
					value="governance"
					class="min-h-[44px] rounded-md px-3 py-2 text-xs font-medium whitespace-nowrap text-secondary transition-colors hover:text-foreground data-[state=active]:bg-selection data-[state=active]:text-foreground md:min-h-[36px]"
				>
					{t.tabs.governance} ({queuedSendersCount})
				</Tabs.Trigger>
			</Tabs.List>

			<Tabs.Content value="rules" class="outline-none">
				<WdgRulesSplitPane
					transport={rulesTransport}
					{alertsTransport}
					{guard}
					selectedIds={resolvedSelectedIds}
					seedMessageId={resolvedSeedMessageId}
					accounts={resolvedAccounts}
					{ruleMeta}
					{resolveMessageSummary}
					onRulesUpdated={(nextRules, nextDests) => {
						rulesList = nextRules;
						destinationsList = nextDests;
					}}
				/>
			</Tabs.Content>

			<Tabs.Content value="alerts" class="outline-none">
				<WdgAlertsManager
					{alertsTransport}
					{scheduleTransport}
					{guard}
					selectedIds={resolvedSelectedIds}
					{historyMeta}
					onDestinationsUpdated={(nextDests) => {
						destinationsList = nextDests;
					}}
				/>
			</Tabs.Content>

			<Tabs.Content value="governance" class="outline-none">
				<div class="flex flex-col gap-5">
					<WdgSenderReviewQueue
						transport={reviewTransport}
						{guard}
						accounts={resolvedAccounts}
						{senderMeta}
						onQueueCountChange={(count) => (queuedSendersCount = count)}
					/>
					<WdgCategoryOverrides
						transport={categoriesTransport}
						{guard}
						accounts={resolvedAccounts}
						{overrideMeta}
					/>
				</div>
			</Tabs.Content>
		</Tabs.Root>
	</div>
</div>
