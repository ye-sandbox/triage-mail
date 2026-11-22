<script lang="ts">
	import Save from '@lucide/svelte/icons/save';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import type { Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels as ruleLabelsMap } from '$lib/rules/labels';
	import type { Condition, Field, Rule } from '$lib/rules/types';
	import { automationLabels } from './labels';
	import { createRuleAction, validateRuleDraft, type ActionKind, type GroupKey } from './rule-editor';
	import { buildCondition } from './simulator';
	import WdgRuleActionsSection from './WdgRuleActionsSection.svelte';
	import WdgRuleConditionsSection from './WdgRuleConditionsSection.svelte';

	interface Props {
		draft: Rule;
		destinations: Destination[];
		accounts?: AccountConfig[];
		isNew?: boolean;
		busy?: boolean;
		onChange?: (next: Rule) => void;
		onSave: (updated: Rule) => void | Promise<void>;
		onDelete?: (id: string, revision: number) => void | Promise<void>;
		onCancel?: () => void;
	}

	let {
		draft = $bindable(),
		destinations,
		accounts = [],
		isNew = false,
		busy = false,
		onChange,
		onSave,
		onDelete,
		onCancel
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].rulesPane);
	const ruleT = $derived(ruleLabelsMap[i18n.locale]);

	let validationError = $state('');
	const totalConditions = $derived(draft.all.length + draft.any.length + draft.except.length);

	function patchDraft(next: Rule) {
		validationError = '';
		draft = next;
		onChange?.(next);
	}

	function addCondition(group: GroupKey) {
		if (totalConditions >= 64) {
			validationError = t.validationTooManyConditions;
			return;
		}
		const fallback: Condition =
			group === 'all' && accounts.length > 0 && draft.all.length === 0
				? { Account: accounts[0].account_id }
				: { Subject: '' };
		patchDraft({ ...draft, [group]: [...draft[group], fallback] });
	}

	function updateConditionField(group: GroupKey, index: number, nextField: Field) {
		const initialVal = nextField === 'AmountAtLeast' || nextField === 'AmountAtMost' ? '0' : '';
		const nextGroup = draft[group].map((c, i) =>
			i === index ? buildCondition(nextField, initialVal) : c
		);
		patchDraft({ ...draft, [group]: nextGroup });
	}

	function updateConditionValue(group: GroupKey, index: number, field: Field, nextVal: string) {
		const nextGroup = draft[group].map((c, i) =>
			i === index ? buildCondition(field, nextVal) : c
		);
		patchDraft({ ...draft, [group]: nextGroup });
	}

	function removeCondition(group: GroupKey, index: number) {
		patchDraft({ ...draft, [group]: draft[group].filter((_, i) => i !== index) });
	}

	function addAction(kind: ActionKind) {
		if (draft.actions.length >= 16) return;
		patchDraft({ ...draft, actions: [...draft.actions, createRuleAction(kind, destinations)] });
	}

	function updateActionKind(index: number, kind: ActionKind) {
		patchDraft({
			...draft,
			actions: draft.actions.map((a, i) => (i === index ? createRuleAction(kind, destinations) : a))
		});
	}

	function updateActionValue(index: number, kind: ActionKind, val: string) {
		patchDraft({
			...draft,
			actions: draft.actions.map((a, i) => {
				if (i !== index) return a;
				if (kind === 'Tag') return { Tag: val };
				if (kind === 'Alert') return { Alert: val };
				if (kind === 'UrgentAlert') return { UrgentAlert: val };
				return a;
			})
		});
	}

	function removeAction(index: number) {
		patchDraft({ ...draft, actions: draft.actions.filter((_, i) => i !== index) });
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		const snapshot = $state.snapshot(draft);
		const sanitized: Rule = {
			...snapshot,
			name: snapshot.name.trim(),
			actions: snapshot.actions.map((a) =>
				typeof a === 'object' && 'Tag' in a ? { Tag: a.Tag.trim() } : a
			)
		};
		const err = validateRuleDraft(sanitized, {
			error: ruleT.error,
			validationMissingCondition: t.validationMissingCondition,
			validationTooManyConditions: t.validationTooManyConditions,
			validationMissingAction: t.validationMissingAction,
			validationInvalidCondition: t.validationInvalidCondition,
			validationInvalidTag: t.validationInvalidTag
		});
		if (err) {
			validationError = err;
			return;
		}
		await onSave(sanitized);
	}
</script>

<form onsubmit={handleSubmit} class="min-w-0 max-w-full rounded-lg border border-separator bg-surface p-4 text-xs">
	<fieldset disabled={busy} class="flex min-w-0 max-w-full flex-col gap-4 border-0 p-0">
		<div class="flex min-w-0 flex-wrap items-end justify-between gap-3 border-b border-separator pb-3">
			<label class="flex w-full min-w-0 flex-col gap-1 sm:w-auto sm:min-w-[200px] sm:flex-1">
				<span class="text-[11px] font-medium text-secondary">{t.ruleName}</span>
				<input
					type="text"
					required
					maxlength="120"
					value={draft.name}
					oninput={(e) => patchDraft({ ...draft, name: e.currentTarget.value })}
					placeholder={t.ruleNamePlaceholder}
					class="h-9 w-full min-w-0 rounded-md border border-separator bg-field px-3 text-xs font-medium text-foreground focus:border-accent focus:outline-none"
				/>
			</label>

			<div class="flex min-w-0 max-w-full flex-wrap items-center gap-2">
				<label class="flex cursor-pointer items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground">
					<input
						type="checkbox"
						checked={draft.enabled}
						onchange={(e) => patchDraft({ ...draft, enabled: e.currentTarget.checked })}
						class="h-3.5 w-3.5 accent-accent"
					/>
					<span>{t.enabledLabel}</span>
				</label>

				<label class="flex cursor-pointer items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground">
					<input
						type="checkbox"
						checked={draft.stop_processing}
						onchange={(e) => patchDraft({ ...draft, stop_processing: e.currentTarget.checked })}
						class="h-3.5 w-3.5 accent-accent"
					/>
					<span class="font-mono text-[11px]">{t.stopProcessing}</span>
				</label>

				{#if !isNew && onDelete}
					<button
						type="button"
						onclick={() => void onDelete(draft.id, draft.revision)}
						aria-label={t.deleteRule}
						title={t.deleteRule}
						class="flex h-9 items-center rounded-md border border-separator bg-surface px-3 text-status-danger hover:bg-field"
					>
						<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
				{/if}

				{#if onCancel}
					<button
						type="button"
						onclick={onCancel}
						class="flex h-9 items-center rounded-md border border-separator bg-surface px-3 text-xs font-medium text-secondary hover:text-foreground"
					>
						{t.cancelDraft}
					</button>
				{/if}

				<button
					type="submit"
					class="flex h-9 items-center gap-2 rounded-md bg-accent px-4 text-xs font-medium whitespace-nowrap text-accent-foreground hover:opacity-95"
				>
					<Save class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{isNew ? t.createRule : t.saveRule}</span>
				</button>
			</div>
		</div>

		<p class="text-[11px] text-secondary">{t.centsHelper}</p>

		{#if validationError}
			<p role="alert" class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-[11px] text-status-danger">
				{validationError}
			</p>
		{/if}

		<WdgRuleConditionsSection
			{draft}
			{accounts}
			{totalConditions}
			onAddCondition={addCondition}
			onUpdateField={updateConditionField}
			onUpdateValue={updateConditionValue}
			onRemoveCondition={removeCondition}
		/>

		<WdgRuleActionsSection
			actions={draft.actions}
			{destinations}
			onAddAction={addAction}
			onUpdateKind={updateActionKind}
			onUpdateValue={updateActionValue}
			onRemoveAction={removeAction}
		/>
	</fieldset>
</form>
