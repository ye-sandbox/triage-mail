<script lang="ts">
	import Play from '@lucide/svelte/icons/play';
	import type { Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as ruleLabelsMap } from '$lib/rules/labels';
	import type { ConditionResult, Field, Outcome, Plan, Rule } from '$lib/rules/types';
	import { automationLabels } from './labels';
	import { extractConditionEntry, formatRuleAction } from './simulator';

	export interface SimulatedMessageSummary {
		subject: string;
		sender: string;
		account_id: string;
	}

	interface Props {
		rules: Rule[];
		draft?: Rule | null;
		destinations: Destination[];
		selectedIds: string[];
		results: [string, Plan][];
		lastSimulatedAt: string | null;
		busy?: boolean;
		resolveMessageSummary?: (id: string) => SimulatedMessageSummary | undefined;
		onSimulate: () => void | Promise<void>;
	}

	let {
		rules,
		draft = null,
		destinations,
		selectedIds,
		results,
		lastSimulatedAt,
		busy = false,
		resolveMessageSummary,
		onSimulate
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].simulator);
	const ruleT = $derived(ruleLabelsMap[i18n.locale]);

	function outcomeStyle(outcome: Outcome): string {
		switch (outcome) {
			case 'Matched':
				return 'border-status-healthy/40 bg-status-healthy/10 text-status-healthy';
			case 'NotMatched':
				return 'border-separator bg-field text-secondary';
			case 'Excluded':
				return 'border-status-warning/40 bg-status-warning/10 text-status-warning';
			case 'Disabled':
				return 'border-separator bg-field text-secondary';
			case 'Stopped':
				return 'border-status-info/40 bg-status-info/10 text-status-info';
		}
	}

	function condStyle(state: ConditionResult): string {
		switch (state) {
			case 'Matched':
				return 'border-status-healthy/40 bg-status-healthy/10 text-status-healthy';
			case 'NotMatched':
				return 'border-separator bg-surface text-secondary';
			case 'MissingFact':
				return 'border-status-warning/40 bg-status-warning/10 text-status-warning';
		}
	}

	function lookupRule(ruleId: string): Rule | undefined {
		if (draft && draft.id === ruleId) return draft;
		return rules.find((r) => r.id === ruleId);
	}

	function focusedExplanation(plan: Plan) {
		if (draft) {
			const match = plan.explanations.find((e) => e.rule_id === draft.id);
			if (match) return match;
		}
		return (
			plan.explanations.find((e) => e.outcome === 'Matched') ??
			plan.explanations[0] ??
			null
		);
	}

	function stopperRuleName(plan: Plan): string | null {
		for (const exp of plan.explanations) {
			if (exp.outcome === 'Matched') {
				const r = lookupRule(exp.rule_id);
				if (r?.stop_processing) return r.name || r.id;
			}
		}
		return null;
	}

	function conditionChips(exp: Plan['explanations'][number]) {
		const rule = lookupRule(exp.rule_id);
		const groups: { key: 'all' | 'any' | 'except'; label: string }[] = [
			{ key: 'all', label: t.allLabel },
			{ key: 'any', label: t.anyLabel },
			{ key: 'except', label: t.exceptLabel }
		];
		const chips: {
			id: string;
			group: string;
			field: string;
			value: string;
			state: ConditionResult;
		}[] = [];

		for (const grp of groups) {
			exp[grp.key].forEach((state, idx) => {
				const cond = rule?.[grp.key][idx];
				const entry = cond ? extractConditionEntry(cond) : null;
				const fieldName = entry ? ruleT.fields[entry.field as Field] ?? entry.field : grp.label;
				chips.push({
					id: `${exp.rule_id}-${grp.key}-${idx}`,
					group: grp.label,
					field: fieldName,
					value: entry?.value ?? '',
					state
				});
			});
		}
		return chips;
	}
</script>

<section class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4 text-xs">
	<div class="flex flex-wrap items-center justify-between gap-3 border-b border-separator pb-3">
		<div class="space-y-1">
			<h3 class="font-display text-sm font-semibold text-foreground">{t.title}</h3>
			<p class="text-[11px] text-secondary">{t.subtitle}</p>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			<span class="font-mono text-[11px] tabular-nums text-secondary">
				{t.selectedCount(selectedIds.length)}
			</span>
			{#if lastSimulatedAt}
				<span class="font-mono text-[11px] tabular-nums text-secondary">
					{t.ranAt(lastSimulatedAt)}
				</span>
			{/if}
			<button
				type="button"
				disabled={busy || !selectedIds.length}
				onclick={() => void onSimulate()}
				class="flex min-h-[44px] items-center gap-2 rounded-md border border-accent bg-selection-soft px-4 py-2 text-xs font-medium whitespace-nowrap text-foreground hover:bg-selection active:scale-[0.98] disabled:opacity-45 md:min-h-[36px]"
			>
				<Play class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
				<span>{busy ? t.running : t.runButton}</span>
			</button>
		</div>
	</div>

	{#if !selectedIds.length}
		<div class="rounded-md border border-separator bg-field p-4 text-center text-xs text-secondary">
			{t.noMessages}
		</div>
	{:else if results.length === 0}
		<div class="rounded-md border border-separator bg-field p-4 text-center text-xs text-secondary">
			{t.emptyPrompt}
		</div>
	{:else}
		<div class="flex flex-col divide-y divide-separator rounded-md border border-separator bg-field/40">
			{#each results as [msgId, plan], idx (msgId)}
				{@const summary = resolveMessageSummary?.(msgId)}
				{@const focused = focusedExplanation(plan)}
				{@const stopper = focused?.outcome === 'Stopped' ? stopperRuleName(plan) : null}
				<div class="flex flex-col gap-2 p-3">
					<div class="flex flex-wrap items-start justify-between gap-2">
						<div class="min-w-0 flex-1 space-y-1">
							<div class="truncate font-medium text-foreground">
								{summary?.subject ?? `${ruleT.message} ${idx + 1}`}
							</div>
							<div class="flex flex-wrap items-center gap-2 font-mono text-[11px] text-secondary">
								<span>{summary?.sender ?? msgId}</span>
								{#if summary?.account_id}
									<span aria-hidden="true">·</span>
									<span>{summary.account_id}</span>
								{/if}
							</div>
						</div>
						{#if focused}
							<span
								class="rounded-md border px-2 py-1 font-mono text-[11px] font-medium whitespace-nowrap {outcomeStyle(
									focused.outcome
								)}"
							>
								{t.planPrefix} {ruleT.outcomes[focused.outcome]}
							</span>
						{/if}
					</div>

					{#if stopper}
						<div class="text-[11px] text-status-info">
							{t.shortCircuitedBy} <span class="font-medium">{stopper}</span> (<code
								class="font-mono"
							>stop_processing</code
							>)
						</div>
					{/if}

					{#if focused}
						{@const chips = conditionChips(focused)}
						{#if chips.length > 0}
							<div class="flex flex-wrap gap-2">
								{#each chips as cond (cond.id)}
									<span
										class="inline-flex items-center gap-2 rounded-md border px-2 py-1 font-mono text-[11px] {condStyle(
											cond.state
										)}"
									>
										<span class="font-semibold">{cond.group}</span>
										<span>{cond.field}={cond.value || '""'}</span>
										<span aria-hidden="true">·</span>
										<span>{ruleT.outcomes[cond.state]}</span>
									</span>
								{/each}
							</div>
						{/if}
					{/if}

					{#if plan.explanations.length > 1}
						<div class="flex flex-wrap gap-2 text-[11px]">
							{#each plan.explanations as exp (exp.rule_id)}
								{@const rName = lookupRule(exp.rule_id)?.name ?? exp.rule_id}
								<span
									class="rounded border px-2 py-1 font-mono {outcomeStyle(exp.outcome)}"
								>
									{rName}: {ruleT.outcomes[exp.outcome]}
								</span>
							{/each}
						</div>
					{/if}

					<div class="flex flex-wrap items-center gap-2 text-[11px] text-foreground">
						<span class="text-secondary">{t.plannedActions}</span>
						<span class="font-mono">
							{plan.actions.map((a) => formatRuleAction(a, destinations)).join(', ') || '—'}
						</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</section>
