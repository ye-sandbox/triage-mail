<script lang="ts">
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels as ruleLabelsMap } from '$lib/rules/labels';
	import { FIELDS, type Field, type Rule } from '$lib/rules/types';
	import { automationLabels } from './labels';
	import type { GroupKey } from './rule-editor';
	import { conditionPlaceholder, extractConditionEntry } from './simulator';

	interface Props {
		draft: Rule;
		accounts: AccountConfig[];
		totalConditions: number;
		onAddCondition: (group: GroupKey) => void;
		onUpdateField: (group: GroupKey, index: number, nextField: Field) => void;
		onUpdateValue: (group: GroupKey, index: number, field: Field, nextVal: string) => void;
		onRemoveCondition: (group: GroupKey, index: number) => void;
	}

	let {
		draft,
		accounts,
		totalConditions,
		onAddCondition,
		onUpdateField,
		onUpdateValue,
		onRemoveCondition
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].rulesPane);
	const ruleT = $derived(ruleLabelsMap[i18n.locale]);

	const GROUPS = $derived<{ key: GroupKey; title: string; logic: string }[]>([
		{ key: 'all', title: t.groupAll, logic: t.groupAllLogic },
		{ key: 'any', title: t.groupAny, logic: t.groupAnyLogic },
		{ key: 'except', title: t.groupExcept, logic: t.groupExceptLogic }
	]);
</script>

<div class="flex min-w-0 flex-col gap-3">
	{#each GROUPS as grp (grp.key)}
		<section class="min-w-0 rounded-md border border-separator bg-field/60 p-3">
			<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
				<div class="flex min-w-0 flex-wrap items-center gap-2">
					<span class="font-mono text-[11px] font-semibold text-foreground">{grp.title}</span>
					<span class="text-[11px] text-secondary">{grp.logic}</span>
				</div>
				<button
					type="button"
					disabled={totalConditions >= 64}
					onclick={() => onAddCondition(grp.key)}
					class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-[11px] font-medium text-foreground hover:bg-selection-soft disabled:opacity-40"
				>
					<Plus class="h-3 w-3" aria-hidden="true" />
					<span>{t.addCondition}</span>
				</button>
			</div>

			{#if draft[grp.key].length === 0}
				<p class="text-[11px] text-secondary">{t.noGroupConditions(grp.title)}</p>
			{:else}
				<div class="flex min-w-0 flex-col gap-2">
					{#each draft[grp.key] as cond, idx (`${grp.key}-${idx}`)}
						{@const entry = extractConditionEntry(cond)}
						<div class="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center">
							<select
								value={entry.field}
								onchange={(e) => onUpdateField(grp.key, idx, e.currentTarget.value as Field)}
								aria-label={`${grp.title} ${t.conditionFieldAria} ${idx + 1}`}
								class="h-9 w-full min-w-0 rounded-md border border-separator bg-surface px-3 font-mono text-[11px] text-foreground focus:border-accent focus:outline-none sm:w-52"
							>
								{#each FIELDS as field (field)}
									<option value={field}>{ruleT.fields[field]}</option>
								{/each}
							</select>

							{#if entry.field === 'Account'}
								<select
									value={entry.value}
									onchange={(e) => onUpdateValue(grp.key, idx, 'Account', e.currentTarget.value)}
									aria-label={`${grp.title} ${t.conditionValueAria} ${idx + 1}`}
									class="h-9 w-full min-w-0 flex-1 rounded-md border border-separator bg-surface px-3 text-xs text-foreground focus:border-accent focus:outline-none sm:w-auto"
								>
									<option value="">—</option>
									{#each accounts as acct (acct.account_id)}
										<option value={acct.account_id}>{acct.email_display}</option>
									{/each}
									{#if entry.value && !accounts.some((a) => a.account_id === entry.value)}
										<option value={entry.value}>{entry.value}</option>
									{/if}
								</select>
							{:else}
								<input
									type={entry.field.startsWith('Amount')
										? 'number'
										: entry.field.startsWith('Due')
											? 'date'
											: 'text'}
									min={entry.field.startsWith('Amount') ? '0' : undefined}
									max={entry.field.startsWith('Amount') ? '9007199254740991' : undefined}
									step={entry.field.startsWith('Amount') ? '1' : undefined}
									maxlength="512"
									value={entry.value}
									oninput={(e) => onUpdateValue(grp.key, idx, entry.field, e.currentTarget.value)}
									placeholder={conditionPlaceholder(entry.field)}
									aria-label={`${grp.title} ${t.conditionValueAria} ${idx + 1}`}
									class="h-9 w-full min-w-0 flex-1 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none sm:w-auto"
								/>
							{/if}

							<button
								type="button"
								onclick={() => onRemoveCondition(grp.key, idx)}
								aria-label={`${t.removeConditionAria} ${idx + 1}`}
								class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-separator bg-surface text-secondary hover:text-status-danger"
							>
								<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{/each}
</div>
