<script lang="ts">
	import { untrack } from 'svelte';
	import { Switch } from 'bits-ui';
	import { ChevronDown, ChevronUp, Pencil, Plus, Trash2 } from 'lucide-svelte';
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
	import WdgRuleSimulatorPanel, {
		type SimulatedMessageSummary
	} from './WdgRuleSimulatorPanel.svelte';

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
		untrack(
			() =>
				void operation(async () => {
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
				})
		);
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
		if (accounts.length > 0) {
			fresh.all = [{ Account: accounts[0].account_id }];
		}
		draft = fresh;
		simResults = [];
		lastSimulatedAt = null;
		guard.markDirty(true);
	}

	function cancelDraft() {
		isCreatingNew = false;
		simResults = [];
		guard.markDirty(false);
		if (rules.length > 0) {
			selectedId = rules[0].id;
			draft = clonePlain(rules[0]);
		} else {
			selectedId = '';
			draft = null;
		}
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
			toast.success(
				wasNew ? t.ruleCreatedToast(refreshed.name) : t.ruleSavedToast(refreshed.name)
			);
		});
	}

	async function deleteRule(id: string, revision: number) {
		await operation(async () => {
			await transport.remove(id, revision);
			await load();
			if (!active) return;
			isCreatingNew = false;
			guard.markDirty(false);
			if (rules.length > 0) {
				selectedId = rules[0].id;
				draft = clonePlain(rules[0]);
			} else {
				selectedId = '';
				draft = null;
			}
			simResults = [];
			toast.success(t.ruleDeletedToast);
		});
	}

	async function toggleRule(rule: Rule) {
		const snapshot = $state.snapshot(rule);
		const nextEnabled = !snapshot.enabled;
		await operation(async () => {
			const saved = await transport.save({ ...snapshot, enabled: nextEnabled });
			await load();
			if (!active) return;
			if (!isCreatingNew && draft?.id === saved.id) {
				draft = clonePlain(rules.find((r) => r.id === saved.id) ?? saved);
			}
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
			if (!active) return;
			toast.info(t.ruleMovedToast(item.name, targetIdx + 1));
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
			const matchedCount = plans.filter(([, p]) =>
				p.explanations.some((e) => e.outcome === 'Matched')
			).length;
			toast.success(simT.simulatedToast(plans.length, matchedCount));
		});
	}
</script>

<div class="flex flex-col gap-3">
	<p class="text-xs text-secondary">{ruleT.notice}</p>

	{#if errorMessage}
		<p
			role="alert"
			class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger"
		>
			{errorMessage}
		</p>
	{/if}

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-[340px_minmax(0,1fr)] lg:items-start">
		<aside class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-3">
			<div class="flex items-center justify-between gap-2 border-b border-separator pb-2.5">
				<div>
					<h2 class="font-display text-sm font-semibold text-foreground">{t.pipelineTitle}</h2>
					<p class="text-[11px] text-secondary">{t.pipelineSubtitle}</p>
				</div>
				<button
					type="button"
					disabled={busy}
					onclick={startNewRule}
					class="flex min-h-[36px] items-center gap-1 rounded-md bg-accent px-2.5 py-1 text-xs font-medium whitespace-nowrap text-accent-foreground hover:opacity-95 active:scale-[0.98] disabled:opacity-45"
				>
					<Plus class="h-3.5 w-3.5" />
					<span>{t.newRule}</span>
				</button>
			</div>

			{#if rules.length === 0}
				<p class="rounded-md border border-separator bg-field p-3 text-center text-xs text-secondary">
					{t.emptyList}
				</p>
			{:else}
				<div class="flex flex-col divide-y divide-separator">
					{#each rules as rule, idx (rule.id)}
						{@const isSelected = !isCreatingNew && selectedId === rule.id}
						{@const matchCount = ruleMeta[rule.id]?.matches_count}
						<div
							class="flex items-start justify-between gap-2 rounded-md px-2 py-2.5 transition-colors {isSelected
								? 'bg-selection-soft'
								: 'hover:bg-field/60'}"
						>
							<button
								type="button"
								onclick={() => selectRule(rule)}
								class="min-w-0 flex-1 text-left focus-visible:outline-2 focus-visible:outline-accent"
							>
								<div class="flex items-center gap-2">
									<span class="font-mono text-xs font-semibold tabular-nums text-secondary">
										{String(idx + 1).padStart(2, '0')}
									</span>
									<span class="truncate text-xs font-semibold text-foreground">{rule.name}</span>
								</div>
								<div class="mt-1 flex flex-wrap items-center gap-1.5 pl-6 text-[11px] text-secondary">
									<span class="font-mono tabular-nums">
										{t.condsLabel(rule.all.length + rule.any.length + rule.except.length)}
									</span>
									<span aria-hidden="true">·</span>
									<span class="font-mono tabular-nums">{t.actionsLabel(rule.actions.length)}</span>
									{#if typeof matchCount === 'number'}
										<span aria-hidden="true">·</span>
										<span class="font-mono tabular-nums">{t.matchesLabel(matchCount)}</span>
									{/if}
									{#if rule.stop_processing}
										<span
											class="rounded border border-status-info/40 bg-status-info/10 px-1.5 py-0.5 font-mono text-[10px] text-status-info"
										>
											{t.stopProcessing}
										</span>
									{/if}
								</div>
							</button>

							<div class="flex shrink-0 items-center gap-1">
								<button
									type="button"
									disabled={busy}
									onclick={() => selectRule(rule)}
									aria-label={`${t.editRule} ${rule.name}`}
									title={t.editRule}
									class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-foreground disabled:opacity-40"
								>
									<Pencil class="h-3 w-3" />
								</button>
								<button
									type="button"
									disabled={busy || idx === 0}
									onclick={() => void moveRule(idx, -1)}
									aria-label={t.moveUp}
									title={t.moveUp}
									class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-foreground disabled:opacity-40"
								>
									<ChevronUp class="h-3.5 w-3.5" />
								</button>
								<button
									type="button"
									disabled={busy || idx === rules.length - 1}
									onclick={() => void moveRule(idx, 1)}
									aria-label={t.moveDown}
									title={t.moveDown}
									class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-foreground disabled:opacity-40"
								>
									<ChevronDown class="h-3.5 w-3.5" />
								</button>
								<button
									type="button"
									disabled={busy}
									onclick={() => void deleteRule(rule.id, rule.revision)}
									aria-label={`${t.deleteRule} ${rule.name}`}
									title={t.deleteRule}
									class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-status-danger disabled:opacity-40"
								>
									<Trash2 class="h-3 w-3" />
								</button>
								<Switch.Root
									checked={rule.enabled}
									disabled={busy}
									onCheckedChange={() => void toggleRule(rule)}
									aria-label={t.toggleRule(rule.name)}
									class="ml-0.5 inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-field focus-visible:outline-2 focus-visible:outline-accent disabled:opacity-45"
								>
									<Switch.Thumb
										class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
									/>
								</Switch.Root>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</aside>

		<div class="flex min-w-0 flex-col gap-4">
			{#if draft}
				{#key `${draft.id}:${draft.revision}`}
					<WdgRuleEditorForm
						bind:draft
						{destinations}
						{accounts}
						isNew={isCreatingNew}
						{busy}
						onChange={() => guard.markDirty(true)}
						onSave={saveDraft}
						onDelete={deleteRule}
						onCancel={cancelDraft}
					/>
				{/key}
			{:else}
				<div
					class="flex flex-col items-center justify-center gap-2 rounded-lg border border-separator bg-surface p-6 text-center text-xs text-secondary"
				>
					<p>{t.emptyList}</p>
					<button
						type="button"
						disabled={busy}
						onclick={startNewRule}
						class="flex min-h-[36px] items-center gap-1 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95"
					>
						<Plus class="h-3.5 w-3.5" />
						<span>{t.newRule}</span>
					</button>
				</div>
			{/if}

			<WdgRuleSimulatorPanel
				{rules}
				{draft}
				{destinations}
				{selectedIds}
				results={simResults}
				{lastSimulatedAt}
				{busy}
				{resolveMessageSummary}
				onSimulate={runSimulation}
			/>
		</div>
	</div>
</div>
