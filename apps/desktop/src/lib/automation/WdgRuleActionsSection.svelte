<script lang="ts">
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import { scheduleLabels } from '$lib/alerts/schedule';
	import type { Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as ruleLabelsMap } from '$lib/rules/labels';
	import type { RuleAction } from '$lib/rules/types';
	import { automationLabels } from './labels';
	import { actionKindOf, type ActionKind } from './rule-editor';

	interface Props {
		actions: RuleAction[];
		destinations: Destination[];
		onAddAction: (kind: ActionKind) => void;
		onUpdateKind: (index: number, kind: ActionKind) => void;
		onUpdateValue: (index: number, kind: ActionKind, val: string) => void;
		onRemoveAction: (index: number) => void;
	}

	let {
		actions,
		destinations,
		onAddAction,
		onUpdateKind,
		onUpdateValue,
		onRemoveAction
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].rulesPane);
	const ruleT = $derived(ruleLabelsMap[i18n.locale]);
	const alertT = $derived(alertLabelsMap[i18n.locale]);
	const schedT = $derived(scheduleLabels[i18n.locale]);
</script>

<section class="min-w-0 rounded-md border border-separator bg-field/60 p-3">
	<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
		<span class="font-mono text-[11px] font-semibold text-foreground">
			{t.actionStack(actions.length)}
		</span>
		<div class="flex flex-wrap gap-2">
			{#each ['Tag', 'Star', 'Archive', 'Alert', 'UrgentAlert'] as kind (kind)}
				{@const needsDest = kind === 'Alert' || kind === 'UrgentAlert'}
				<button
					type="button"
					disabled={actions.length >= 16 || (needsDest && destinations.length === 0)}
					onclick={() => onAddAction(kind as ActionKind)}
					title={needsDest && destinations.length === 0 ? t.noDestinationsAvailable : undefined}
					class="flex min-h-[30px] items-center gap-1 rounded-md border border-separator bg-surface px-2 py-1 font-mono text-[11px] text-foreground hover:bg-selection-soft disabled:opacity-40"
				>
					<Plus class="h-3 w-3" aria-hidden="true" />
					<span>{kind}</span>
				</button>
			{/each}
		</div>
	</div>

	{#if actions.length === 0}
		<p class="text-[11px] text-secondary">{t.addActionHint}</p>
	{:else}
		<div class="flex min-w-0 flex-col gap-2">
			{#each actions as act, idx (`act-${idx}`)}
				{@const kind = actionKindOf(act)}
				<div class="flex min-w-0 flex-wrap items-center gap-2">
					<select
						value={kind}
						onchange={(e) => onUpdateKind(idx, e.currentTarget.value as ActionKind)}
						aria-label={`${t.actionTypeAria} ${idx + 1}`}
						class="h-9 min-w-0 max-w-full rounded-md border border-separator bg-surface px-3 font-mono text-[11px] text-foreground focus:border-accent focus:outline-none"
					>
						<option value="Tag">Tag(string)</option>
						<option value="Star">{ruleT.star}</option>
						<option value="Archive">{ruleT.archive}</option>
						<option value="Alert" disabled={destinations.length === 0}>Alert(destination)</option>
						<option value="UrgentAlert" disabled={destinations.length === 0}>
							UrgentAlert(destination)
						</option>
					</select>

					{#if typeof act === 'object' && 'Tag' in act}
						<input
							type="text"
							maxlength="80"
							value={act.Tag}
							oninput={(e) => onUpdateValue(idx, 'Tag', e.currentTarget.value)}
							placeholder="Payable"
							class="h-9 min-w-0 flex-1 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
						/>
					{:else if typeof act === 'object' && ('Alert' in act || 'UrgentAlert' in act)}
						{@const destId = 'Alert' in act ? act.Alert : act.UrgentAlert}
						<select
							value={destId}
							onchange={(e) => onUpdateValue(idx, kind, e.currentTarget.value)}
							aria-label={`${t.targetDestinationAria} ${idx + 1}`}
							class="h-9 min-w-0 flex-1 rounded-md border border-separator bg-surface px-3 text-xs text-foreground focus:border-accent focus:outline-none"
						>
							{#if destId && !destinations.some((d) => d.id === destId)}
								<option value={destId}>
									{destId} · {alertT.status.Unavailable} ({t.unavailableDestination})
								</option>
							{/if}
							{#each destinations as dest (dest.id)}
								<option value={dest.id}>
									{dest.name} ({dest.kind === 'Local' ? alertT.local : dest.public_origin ?? dest.kind})
								</option>
							{/each}
						</select>
					{:else}
						<span class="min-w-0 flex-1 font-mono text-[11px] text-secondary">{t.noParamRequired}</span>
					{/if}

					<button
						type="button"
						onclick={() => onRemoveAction(idx)}
						aria-label={`${t.removeActionAria} ${idx + 1}`}
						class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-separator bg-surface text-secondary hover:text-status-danger"
					>
						<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
				</div>
			{/each}
		</div>
	{/if}

	<p class="mt-2 text-[11px] text-secondary">{schedT.urgentHelp}</p>
</section>
