<script lang="ts">
	import { untrack } from 'svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import { toast } from 'svelte-sonner';
	import type { AlertsTransport, Destination } from '$lib/alerts/types';
	import { nativeAlertsTransport } from '$lib/alerts/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels as ruleLabelsMap } from '$lib/rules/labels';
	import { nativeRulesTransport } from '$lib/rules/transport';
	import { newRule, type Plan, type Rule, type RulesTransport } from '$lib/rules/types';
	import { SettingsFormGuard } from '$lib/settings/guards';
	import type { RuleVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';
	import { clonePlain } from './simulator';
	import WdgRuleEditorForm from './WdgRuleEditorForm.svelte';
	import WdgRuleSimulatorPanel, { type SimulatedMessageSummary } from './WdgRuleSimulatorPanel.svelte';
	import WdgRulesPipelineSidebar from './WdgRulesPipelineSidebar.svelte';

	interface Props {
		transport?: RulesTransport;
		alertsTransport?: AlertsTransport;
		guard?: SettingsFormGuard;
		selectedIds?: string[];
		seedMessageId?: string;
		accounts?: AccountConfig[];
		ruleMeta?: Record<string, RuleVisualMeta>;
		resolveMessageSummary?: (id: string) => SimulatedMessageSummary | undefined;
		onRulesUpdated?: (rules: Rule[], destinations: Destination[]) => void;
	}

	let {
		transport = nativeRulesTransport,
		alertsTransport = nativeAlertsTransport,
		guard = new SettingsFormGuard(),
		selectedIds = [],
		seedMessageId,
		accounts = [],
		ruleMeta = {},
		resolveMessageSummary,
		onRulesUpdated
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].rulesPane);
	const simT = $derived(automationLabels[i18n.locale].simulator);
	const ruleT = $derived(ruleLabelsMap[i18n.locale]);
	let rules = $state<Rule[]>([]);
	let destinations = $state<Destination[]>([]);
	let draft = $state<Rule | null>(null);
	let isCreatingNew = $state(false);
	let selectedId = $state<string>('');
	let busy = $state(false);
	let errorMessage = $state('');
	let simResults = $state<[string, Plan][]>([]);
	let lastSimulatedAt = $state<string | null>(null);
	let active = false;

	async function operation(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		errorMessage = '';
		try {
			await work();
		} catch {
			if (active) errorMessage = ruleT.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}

	async function load() {
		const [loadedRules, loadedDests] = await Promise.all([
			transport.list(),
			alertsTransport.list().catch(() => [])
		]);
		if (!active) return;
		rules = [...loadedRules].sort((a, b) => a.position - b.position);
		destinations = loadedDests;
		onRulesUpdated?.(rules, destinations);
	}

	$effect(() => {
		active = true;
		const seed = seedMessageId;
		untrack(() => void operation(async () => {
			await load();
			if (!active) return;
			if (seed) {
				const seeded = await transport.draft(seed, crypto.randomUUID());
				if (active) {
					isCreatingNew = true;
					selectedId = '';
					draft = seeded;
					simResults = [];
					guard.markDirty(true);
				}
			} else if (rules.length > 0 && !draft) {
				selectedId = rules[0].id;
				draft = clonePlain(rules[0]);
				isCreatingNew = false;
				guard.markDirty(false);
			}
		}));
		return () => {
			active = false;
			guard.reset();
		};
	});

	function selectRule(rule: Rule) {
		isCreatingNew = false;
		selectedId = rule.id;
		draft = clonePlain(rule);
		simResults = [];
		lastSimulatedAt = null;
		guard.markDirty(false);
	}

	function startNewRule() {
		isCreatingNew = true;
		selectedId = '';
		const fresh = newRule(rules.length);
		if (accounts.length > 0) fresh.all = [{ Account: accounts[0].account_id }];
		draft = fresh;
		simResults = [];
		lastSimulatedAt = null;
		guard.markDirty(true);
	}

	function cancelDraft() {
		isCreatingNew = false;
		simResults = [];
		guard.markDirty(false);
		selectedId = rules[0]?.id ?? '';
		draft = rules[0] ? clonePlain(rules[0]) : null;
	}

	async function saveDraft(updated: Rule) {
		await operation(async () => {
			const saved = await transport.save(updated);
			await load();
			if (!active) return;
			const wasNew = isCreatingNew;
			isCreatingNew = false;
			selectedId = saved.id;
			const refreshed = rules.find((r) => r.id === saved.id) ?? saved;
			draft = clonePlain(refreshed);
			guard.markDirty(false);
			toast.success(wasNew ? t.ruleCreatedToast(refreshed.name) : t.ruleSavedToast(refreshed.name));
		});
	}

	async function deleteRule(id: string, revision: number) {
		await operation(async () => {
			await transport.remove(id, revision);
			await load();
			if (!active) return;
			isCreatingNew = false;
			guard.markDirty(false);
			selectedId = rules[0]?.id ?? '';
			draft = rules[0] ? clonePlain(rules[0]) : null;
			simResults = [];
			toast.success(t.ruleDeletedToast);
		});
	}

	async function toggleRule(rule: Rule) {
		const snapshot = $state.snapshot(rule);
		await operation(async () => {
			const saved = await transport.save({ ...snapshot, enabled: !snapshot.enabled });
			await load();
			if (!active) return;
			if (!isCreatingNew && draft?.id === saved.id) draft = clonePlain(rules.find((r) => r.id === saved.id) ?? saved);
			toast.success(t.ruleToggledToast(saved.name, saved.enabled));
		});
	}

	async function moveRule(index: number, delta: -1 | 1) {
		const targetIdx = index + delta;
		if (targetIdx < 0 || targetIdx >= rules.length) return;
		const reordered = [...rules];
		const [item] = reordered.splice(index, 1);
		reordered.splice(targetIdx, 0, item);
		await operation(async () => {
			await transport.reorder(reordered.map((r) => [r.id, r.revision]));
			await load();
			if (active) toast.info(t.ruleMovedToast(item.name, targetIdx + 1));
		});
	}

	async function runSimulation() {
		if (!selectedIds.length) return;
		const draftSnapshot = draft && (isCreatingNew || guard.dirty) ? $state.snapshot(draft) : undefined;
		await operation(async () => {
			const plans = await transport.simulate(selectedIds, draftSnapshot);
			if (!active) return;
			simResults = plans;
			lastSimulatedAt = new Date().toTimeString().slice(0, 8);
			const matchedCount = plans.filter(([, p]) => p.explanations.some((e) => e.outcome === 'Matched')).length;
			toast.success(simT.simulatedToast(plans.length, matchedCount));
		});
	}
</script>

<div class="flex flex-col gap-3">
	<p class="text-xs text-secondary">{ruleT.notice}</p>

	{#if errorMessage}
		<p role="alert" class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger">
			{errorMessage}
		</p>
	{/if}

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-[380px_minmax(0,1fr)] lg:items-start">
		<WdgRulesPipelineSidebar
			{rules} {selectedId} {isCreatingNew} {busy} {ruleMeta}
			onNewRule={startNewRule} onSelectRule={selectRule} onMoveRule={moveRule} onDeleteRule={deleteRule} onToggleRule={toggleRule}
		/>

		<div class="flex min-w-0 flex-col gap-4">
			{#if draft}
				{#key `${draft.id}:${draft.revision}`}
					<WdgRuleEditorForm
						bind:draft {destinations} {accounts} isNew={isCreatingNew} {busy}
						onChange={() => guard.markDirty(true)} onSave={saveDraft} onDelete={deleteRule} onCancel={cancelDraft}
					/>
				{/key}
			{:else}
				<div class="flex flex-col items-center justify-center gap-2 rounded-lg border border-separator bg-surface p-6 text-center text-xs text-secondary">
					<p>{t.emptyList}</p>
					<button type="button" disabled={busy} onclick={startNewRule} class="flex min-h-[36px] items-center gap-1 rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground hover:opacity-95">
						<Plus class="h-3.5 w-3.5" aria-hidden="true" />
						<span>{t.newRule}</span>
					</button>
				</div>
			{/if}

			<WdgRuleSimulatorPanel
				{rules} {draft} {destinations} {selectedIds} results={simResults} {lastSimulatedAt} {busy} {resolveMessageSummary} onSimulate={runSimulation}
			/>
		</div>
	</div>
</div>
