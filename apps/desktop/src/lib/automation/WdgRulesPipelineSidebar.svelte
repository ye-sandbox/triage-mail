<script lang="ts">
	import { Switch } from 'bits-ui';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { Rule } from '$lib/rules/types';
	import type { RuleVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';

	interface Props {
		rules: Rule[];
		selectedId: string;
		isCreatingNew: boolean;
		busy: boolean;
		ruleMeta: Record<string, RuleVisualMeta>;
		onNewRule: () => void;
		onSelectRule: (rule: Rule) => void;
		onMoveRule: (index: number, delta: -1 | 1) => void | Promise<void>;
		onDeleteRule: (id: string, revision: number) => void | Promise<void>;
		onToggleRule: (rule: Rule) => void | Promise<void>;
	}

	let {
		rules,
		selectedId,
		isCreatingNew,
		busy,
		ruleMeta,
		onNewRule,
		onSelectRule,
		onMoveRule,
		onDeleteRule,
		onToggleRule
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].rulesPane);
</script>

<aside class="flex min-w-0 flex-col gap-3 rounded-lg border border-separator bg-surface p-3">
	<div class="flex items-center justify-between gap-2 border-b border-separator pb-3">
		<div class="min-w-0">
			<h2 class="font-display text-sm font-semibold text-foreground">{t.pipelineTitle}</h2>
			<p class="text-[11px] text-secondary">{t.pipelineSubtitle}</p>
		</div>
		<button
			type="button"
			disabled={busy}
			onclick={onNewRule}
			class="flex min-h-[36px] shrink-0 items-center gap-1 rounded-md bg-accent px-3 py-1 text-xs font-medium whitespace-nowrap text-accent-foreground hover:opacity-95 active:scale-[0.98] disabled:opacity-45"
		>
			<Plus class="h-3.5 w-3.5" aria-hidden="true" />
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
					class="flex flex-col gap-2 rounded-md px-3 py-3 transition-colors {isSelected
						? 'bg-selection-soft'
						: 'hover:bg-field/60'}"
				>
					<button
						type="button"
						onclick={() => onSelectRule(rule)}
						class="w-full min-w-0 text-left focus-visible:outline-2 focus-visible:outline-accent"
					>
						<div class="flex items-center gap-2">
							<span class="shrink-0 font-mono text-xs font-semibold tabular-nums text-secondary">
								{String(idx + 1).padStart(2, '0')}
							</span>
							<span class="truncate text-xs font-semibold text-foreground">{rule.name}</span>
						</div>
						<div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 pl-6 text-[11px] text-secondary">
							<span class="font-mono whitespace-nowrap tabular-nums">
								{t.condsLabel(rule.all.length + rule.any.length + rule.except.length)}
							</span>
							<span class="font-mono whitespace-nowrap tabular-nums">
								{t.actionsLabel(rule.actions.length)}
							</span>
							{#if typeof matchCount === 'number'}
								<span class="font-mono whitespace-nowrap tabular-nums">
									{t.matchesLabel(matchCount)}
								</span>
							{/if}
							{#if rule.stop_processing}
								<span
									class="rounded border border-status-info/40 bg-status-info/10 px-2 py-1 font-mono text-[10px] whitespace-nowrap text-status-info"
								>
									{t.stopProcessing}
								</span>
							{/if}
						</div>
					</button>

					<div class="flex items-center justify-between gap-2 pl-6">
						<div class="flex items-center gap-1">
							<button
								type="button"
								disabled={busy}
								onclick={() => onSelectRule(rule)}
								aria-label={`${t.editRule} ${rule.name}`}
								title={t.editRule}
								class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-foreground disabled:opacity-40"
							>
								<Pencil class="h-3 w-3" aria-hidden="true" />
							</button>
							<button
								type="button"
								disabled={busy || idx === 0}
								onclick={() => void onMoveRule(idx, -1)}
								aria-label={t.moveUp}
								title={t.moveUp}
								class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-foreground disabled:opacity-40"
							>
								<ChevronUp class="h-3.5 w-3.5" aria-hidden="true" />
							</button>
							<button
								type="button"
								disabled={busy || idx === rules.length - 1}
								onclick={() => void onMoveRule(idx, 1)}
								aria-label={t.moveDown}
								title={t.moveDown}
								class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-foreground disabled:opacity-40"
							>
								<ChevronDown class="h-3.5 w-3.5" aria-hidden="true" />
							</button>
							<button
								type="button"
								disabled={busy}
								onclick={() => void onDeleteRule(rule.id, rule.revision)}
								aria-label={`${t.deleteRule} ${rule.name}`}
								title={t.deleteRule}
								class="flex h-7 w-7 items-center justify-center rounded border border-separator bg-surface text-secondary hover:text-status-danger disabled:opacity-40"
							>
								<Trash2 class="h-3 w-3" aria-hidden="true" />
							</button>
						</div>
						<Switch.Root
							checked={rule.enabled}
							disabled={busy}
							onCheckedChange={() => void onToggleRule(rule)}
							aria-label={t.toggleRule(rule.name)}
							class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-field focus-visible:outline-2 focus-visible:outline-accent disabled:opacity-45"
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
