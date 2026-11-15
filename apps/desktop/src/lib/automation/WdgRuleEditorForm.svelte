<script lang="ts">
	import { Plus, Save, Trash2 } from 'lucide-svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import type { Destination } from '$lib/alerts/types';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import { scheduleLabels } from '$lib/alerts/schedule';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as ruleLabelsMap } from '$lib/rules/labels';
	import { FIELDS, type Condition, type Field, type Rule, type RuleAction } from '$lib/rules/types';
	import { automationLabels } from './labels';
	import { buildCondition, conditionPlaceholder, extractConditionEntry } from './simulator';

	type GroupKey = 'all' | 'any' | 'except';
	type ActionKind = 'Tag' | 'Star' | 'Archive' | 'Alert' | 'UrgentAlert';

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
	const alertT = $derived(alertLabelsMap[i18n.locale]);
	const schedT = $derived(scheduleLabels[i18n.locale]);

	const GROUPS = $derived<{ key: GroupKey; title: string; logic: string }[]>([
		{ key: 'all', title: t.groupAll, logic: t.groupAllLogic },
		{ key: 'any', title: t.groupAny, logic: t.groupAnyLogic },
		{ key: 'except', title: t.groupExcept, logic: t.groupExceptLogic }
	]);

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
		patchDraft({
			...draft,
			[group]: draft[group].filter((_, i) => i !== index)
		});
	}

	function createAction(kind: ActionKind): RuleAction {
		if (kind === 'Tag') return { Tag: '' };
		if (kind === 'Star') return 'Star';
		if (kind === 'Archive') return 'Archive';
		const firstDestId = destinations[0]?.id ?? '';
		return kind === 'Alert' ? { Alert: firstDestId } : { UrgentAlert: firstDestId };
	}

	function actionKindOf(action: RuleAction): ActionKind {
		if (action === 'Star') return 'Star';
		if (action === 'Archive') return 'Archive';
		if ('Tag' in action) return 'Tag';
		if ('Alert' in action) return 'Alert';
		return 'UrgentAlert';
	}

	function addAction(kind: ActionKind) {
		if (draft.actions.length >= 16) return;
		patchDraft({ ...draft, actions: [...draft.actions, createAction(kind)] });
	}

	function updateActionKind(index: number, kind: ActionKind) {
		patchDraft({
			...draft,
			actions: draft.actions.map((a, i) => (i === index ? createAction(kind) : a))
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
		patchDraft({
			...draft,
			actions: draft.actions.filter((_, i) => i !== index)
		});
	}

	function validateDraft(value: Rule): string | null {
		if (!value.name.trim() || value.name.trim().length > 120) {
			return ruleT.error;
		}
		if (value.all.length + value.any.length === 0) {
			return t.validationMissingCondition;
		}
		if (value.all.length + value.any.length + value.except.length > 64) {
			return t.validationTooManyConditions;
		}
		if (value.actions.length === 0 || value.actions.length > 16) {
			return t.validationMissingAction;
		}
		for (const cond of [...value.all, ...value.any, ...value.except]) {
			const { field, value: raw } = extractConditionEntry(cond);
			if (field === 'AmountAtLeast' || field === 'AmountAtMost') {
				const num = Number(raw);
				if (!Number.isFinite(num) || !Number.isInteger(num) || num < 0) {
					return t.validationInvalidCondition;
				}
			} else if (field === 'DueOnOrAfter' || field === 'DueOnOrBefore') {
				if (!/^\d{4}-\d{2}-\d{2}$/.test(raw.trim())) {
					return t.validationInvalidCondition;
				}
			} else if (field === 'Sender') {
				if (!raw.trim().includes('@')) {
					return t.validationInvalidCondition;
				}
			} else if (!raw.trim()) {
				return t.validationInvalidCondition;
			}
		}
		for (const act of value.actions) {
			if (typeof act === 'object' && 'Tag' in act) {
				const trimmed = act.Tag.trim();
				if (!trimmed || trimmed.length > 80) {
					return t.validationInvalidTag;
				}
			} else if (typeof act === 'object' && ('Alert' in act || 'UrgentAlert' in act)) {
				const destId = 'Alert' in act ? act.Alert : act.UrgentAlert;
				if (!destId.trim()) {
					return t.validationMissingAction;
				}
			}
		}
		return null;
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
		const err = validateDraft(sanitized);
		if (err) {
			validationError = err;
			return;
		}
		await onSave(sanitized);
	}
</script>

<form onsubmit={handleSubmit} class="rounded-lg border border-separator bg-surface p-4 text-xs">
	<fieldset disabled={busy} class="flex flex-col gap-4 border-0 p-0">
		<div class="flex flex-wrap items-end justify-between gap-3 border-b border-separator pb-3">
			<label class="flex min-w-[200px] flex-1 flex-col gap-1">
				<span class="text-[11px] font-medium text-secondary">{t.ruleName}</span>
				<input
					type="text"
					required
					maxlength="120"
					value={draft.name}
					oninput={(e) => patchDraft({ ...draft, name: e.currentTarget.value })}
					placeholder={t.ruleNamePlaceholder}
					class="h-9 w-full rounded-md border border-separator bg-field px-3 text-xs font-medium text-foreground focus:border-accent focus:outline-none"
				/>
			</label>

			<div class="flex flex-wrap items-center gap-2">
				<label
					class="flex cursor-pointer items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground"
				>
					<input
						type="checkbox"
						checked={draft.enabled}
						onchange={(e) => patchDraft({ ...draft, enabled: e.currentTarget.checked })}
						class="h-3.5 w-3.5 accent-accent"
					/>
					<span>{t.enabledLabel}</span>
				</label>

				<label
					class="flex cursor-pointer items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground"
				>
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
						class="flex h-9 items-center rounded-md border border-separator bg-surface px-2.5 text-status-danger hover:bg-field"
					>
						<Trash2 class="h-3.5 w-3.5" />
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
					class="flex h-9 items-center gap-1.5 rounded-md bg-accent px-3.5 text-xs font-medium whitespace-nowrap text-accent-foreground hover:opacity-95"
				>
					<Save class="h-3.5 w-3.5" />
					<span>{isNew ? t.createRule : t.saveRule}</span>
				</button>
			</div>
		</div>

		<p class="text-[11px] text-secondary">{t.centsHelper}</p>

		{#if validationError}
			<p
				role="alert"
				class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-[11px] text-status-danger"
			>
				{validationError}
			</p>
		{/if}

		<div class="flex flex-col gap-3">
			{#each GROUPS as grp (grp.key)}
				<section class="rounded-md border border-separator bg-field/60 p-3">
					<div class="mb-2 flex items-center justify-between gap-2">
						<div class="flex flex-wrap items-center gap-2">
							<span class="font-mono text-[11px] font-semibold text-foreground">{grp.title}</span>
							<span class="text-[11px] text-secondary">{grp.logic}</span>
						</div>
						<button
							type="button"
							disabled={totalConditions >= 64}
							onclick={() => addCondition(grp.key)}
							class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-[11px] font-medium text-foreground hover:bg-selection-soft disabled:opacity-40"
						>
							<Plus class="h-3 w-3" />
							<span>{t.addCondition}</span>
						</button>
					</div>

					{#if draft[grp.key].length === 0}
						<p class="text-[11px] text-secondary">{t.noGroupConditions(grp.title)}</p>
					{:else}
						<div class="flex flex-col gap-2">
							{#each draft[grp.key] as cond, idx (`${grp.key}-${idx}`)}
								{@const entry = extractConditionEntry(cond)}
								<div class="flex flex-col gap-2 sm:flex-row sm:items-center">
									<select
										value={entry.field}
										onchange={(e) =>
											updateConditionField(grp.key, idx, e.currentTarget.value as Field)}
										aria-label={`${grp.title} ${t.conditionFieldAria} ${idx + 1}`}
										class="h-9 rounded-md border border-separator bg-surface px-2.5 font-mono text-[11px] text-foreground focus:border-accent focus:outline-none sm:w-52"
									>
										{#each FIELDS as field (field)}
											<option value={field}>{ruleT.fields[field]}</option>
										{/each}
									</select>

									{#if entry.field === 'Account'}
										<select
											value={entry.value}
											onchange={(e) =>
												updateConditionValue(grp.key, idx, 'Account', e.currentTarget.value)}
											aria-label={`${grp.title} ${t.conditionValueAria} ${idx + 1}`}
											class="h-9 min-w-0 flex-1 rounded-md border border-separator bg-surface px-2.5 text-xs text-foreground focus:border-accent focus:outline-none"
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
											oninput={(e) =>
												updateConditionValue(grp.key, idx, entry.field, e.currentTarget.value)}
											placeholder={conditionPlaceholder(entry.field)}
											aria-label={`${grp.title} ${t.conditionValueAria} ${idx + 1}`}
											class="h-9 min-w-0 flex-1 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
										/>
									{/if}

									<button
										type="button"
										onclick={() => removeCondition(grp.key, idx)}
										aria-label={`${t.removeConditionAria} ${idx + 1}`}
										class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-separator bg-surface text-secondary hover:text-status-danger"
									>
										<Trash2 class="h-3.5 w-3.5" />
									</button>
								</div>
							{/each}
						</div>
					{/if}
				</section>
			{/each}
		</div>

		<section class="rounded-md border border-separator bg-field/60 p-3">
			<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
				<span class="font-mono text-[11px] font-semibold text-foreground">
					{t.actionStack(draft.actions.length)}
				</span>
				<div class="flex flex-wrap gap-1.5">
					{#each ['Tag', 'Star', 'Archive', 'Alert', 'UrgentAlert'] as kind (kind)}
						{@const needsDest = kind === 'Alert' || kind === 'UrgentAlert'}
						<button
							type="button"
							disabled={draft.actions.length >= 16 || (needsDest && destinations.length === 0)}
							onclick={() => addAction(kind as ActionKind)}
							title={needsDest && destinations.length === 0 ? t.noDestinationsAvailable : undefined}
							class="flex min-h-[30px] items-center gap-1 rounded-md border border-separator bg-surface px-2 py-1 font-mono text-[11px] text-foreground hover:bg-selection-soft disabled:opacity-40"
						>
							<Plus class="h-3 w-3" />
							<span>{kind}</span>
						</button>
					{/each}
				</div>
			</div>

			{#if draft.actions.length === 0}
				<p class="text-[11px] text-secondary">{t.addActionHint}</p>
			{:else}
				<div class="flex flex-col gap-2">
					{#each draft.actions as act, idx (`act-${idx}`)}
						{@const kind = actionKindOf(act)}
						<div class="flex flex-wrap items-center gap-2">
							<select
								value={kind}
								onchange={(e) => updateActionKind(idx, e.currentTarget.value as ActionKind)}
								aria-label={`${t.actionTypeAria} ${idx + 1}`}
								class="h-9 rounded-md border border-separator bg-surface px-2.5 font-mono text-[11px] text-foreground focus:border-accent focus:outline-none"
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
									oninput={(e) => updateActionValue(idx, 'Tag', e.currentTarget.value)}
									placeholder="Payable"
									class="h-9 min-w-0 flex-1 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
								/>
							{:else if typeof act === 'object' && ('Alert' in act || 'UrgentAlert' in act)}
								{@const destId = 'Alert' in act ? act.Alert : act.UrgentAlert}
								<select
									value={destId}
									onchange={(e) => updateActionValue(idx, kind, e.currentTarget.value)}
									aria-label={`${t.targetDestinationAria} ${idx + 1}`}
									class="h-9 min-w-0 flex-1 rounded-md border border-separator bg-surface px-2.5 text-xs text-foreground focus:border-accent focus:outline-none"
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
								<span class="flex-1 font-mono text-[11px] text-secondary">{t.noParamRequired}</span>
							{/if}

							<button
								type="button"
								onclick={() => removeAction(idx)}
								aria-label={`${t.removeActionAria} ${idx + 1}`}
								class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-separator bg-surface text-secondary hover:text-status-danger"
							>
								<Trash2 class="h-3.5 w-3.5" />
							</button>
						</div>
					{/each}
				</div>
			{/if}

			<p class="mt-2 text-[11px] text-secondary">{schedT.urgentHelp}</p>
		</section>
	</fieldset>
</form>
