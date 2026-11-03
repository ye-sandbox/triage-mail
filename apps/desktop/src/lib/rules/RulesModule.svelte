<script lang="ts">
	import type { AccountConfig } from '../onboarding/types';
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { SettingsFormGuard } from '../settings/guards';
	import { labels } from './labels';
	import { newRule, type Rule, type Plan, type RulesTransport, type Field } from './types';
	import { nativeRulesTransport } from './transport';
	import { scheduleLabels } from '../alerts/schedule';
	import { nativeAlertsTransport } from '../alerts/transport';
	import type { AlertsTransport, Destination } from '../alerts/types';
	import { labels as alertLabels } from '../alerts/labels';
	import RuleEditor from './RuleEditor.svelte';
	let {
		transport = nativeRulesTransport,
		guard,
		selectedIds = [],
		seedMessageId,
		accounts = [],
		alertsTransport = nativeAlertsTransport
	}: {
		transport?: RulesTransport;
		guard: SettingsFormGuard;
		selectedIds?: string[];
		seedMessageId?: string;
		accounts?: AccountConfig[];
		alertsTransport?: AlertsTransport;
	} = $props();
	let rules = $state<Rule[]>([]);
	let destinations = $state<Destination[]>([]);
	let draft = $state<Rule | null>(null);
	let busy = $state(false);
	let message = $state('');
	let results = $state<[string, Plan][]>([]);
	let active = false;
	const t = $derived(labels[i18n.locale]);
	function setDraft(rule: Rule | null) {
		draft = rule;
		results = [];
		guard.markDirty(rule !== null);
	}
	async function operation(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		message = '';
		try {
			await work();
		} catch {
			if (active) message = t.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}
	async function load() {
		const loaded = await transport.list();
		const alerts = await alertsTransport.list().catch(() => []);
		if (active) destinations = alerts;
		if (active) rules = loaded;
	}
	$effect(() => {
		active = true;
		const seed = seedMessageId;
		untrack(
			() =>
				void operation(async () => {
					await load();
					if (seed) {
						const result = await transport.draft(seed, crypto.randomUUID());
						if (active) setDraft(result);
					}
				})
		);
		return () => {
			active = false;
			guard.reset();
		};
	});
	async function save() {
		if (!draft) return;
		const value = $state.snapshot(draft);
		await operation(async () => {
			await transport.save(value);
			if (active) {
				setDraft(null);
				await load();
				message = t.saved;
			}
		});
	}
	async function move(index: number, delta: number) {
		const reordered = [...rules];
		[reordered[index], reordered[index + delta]] = [reordered[index + delta], reordered[index]];
		await operation(async () => {
			await transport.reorder(reordered.map((r) => [r.id, r.revision]));
			await load();
		});
	}
	function actionName(action: Plan['actions'][number]) {
		return typeof action === 'object'
			? 'Tag' in action
				? action.Tag
				: `${'UrgentAlert' in action ? scheduleLabels[i18n.locale].urgentRule : alertLabels[i18n.locale].title}: ${destinations.find((d) => d.id === ('Alert' in action ? action.Alert : action.UrgentAlert))?.name ?? ('Alert' in action ? action.Alert : action.UrgentAlert)}`
			: action === 'Star'
				? t.star
				: t.archive;
	}
</script>

<section class="space-y-4" aria-label={t.title}>
	<h2 class="text-lg font-semibold">{t.title}</h2>
	<p class="text-sm text-[var(--secondary)]">{t.notice}</p>
	<p role="status">{message}</p>
	{#if draft}
		{#key draft.id}<RuleEditor
				{accounts}
				{destinations}
				{draft}
				{busy}
				change={setDraft}
				{save}
				cancel={() => setDraft(null)}
			/>{/key}
	{:else}
		<button type="button" disabled={busy} onclick={() => setDraft(newRule(rules.length))}
			>{t.add}</button
		>
		{#if !rules.length}<p>{t.empty}</p>{/if}
		<ul class="space-y-4">
			{#each rules as rule, index (rule.id)}
				<li class="rounded border border-[var(--separator)] p-4 space-y-2">
					<span>{rule.name} · {rule.enabled ? t.enabled : t.outcomes.Disabled}</span>
					<div class="flex flex-wrap gap-4">
						<button
							type="button"
							disabled={busy}
							onclick={() => setDraft(structuredClone($state.snapshot(rule)))}>{t.edit}</button
						>
						<button
							type="button"
							disabled={busy}
							onclick={() =>
								void operation(async () => {
									await transport.save({ ...rule, enabled: !rule.enabled });
									await load();
								})}>{rule.enabled ? t.outcomes.Disabled : t.enabled}</button
						>
						<button
							type="button"
							disabled={busy || index === 0}
							onclick={() => void move(index, -1)}>{t.up}</button
						>
						<button
							type="button"
							disabled={busy || index === rules.length - 1}
							onclick={() => void move(index, 1)}>{t.down}</button
						>
						<button
							type="button"
							disabled={busy}
							onclick={() =>
								void operation(async () => {
									await transport.remove(rule.id, rule.revision);
									await load();
								})}>{t.remove}</button
						>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
	<div class="space-y-2 border-t border-[var(--separator)] pt-4">
		<p class="tabular-nums">{t.selection}: {selectedIds.length}</p>
		{#if !selectedIds.length}<p class="text-sm">{t.noSelection}</p>{/if}
		<button
			type="button"
			disabled={busy || !selectedIds.length}
			onclick={() =>
				void operation(async () => {
					const plans = await transport.simulate(
						selectedIds,
						draft ? $state.snapshot(draft) : undefined
					);
					if (active) results = plans;
				})}>{t.simulate}</button
		>
		{#each results as [id, plan], index (id)}
			<div class="space-y-2 rounded border border-[var(--separator)] p-4">
				<p class="tabular-nums">{t.message} {index + 1}</p>
				<p>{t.actions}: {plan.actions.map(actionName).join(', ') || '—'}</p>
				{#each plan.explanations as explanation}
					<p>
						{rules.find((r) => r.id === explanation.rule_id)?.name ??
							draft?.name ??
							explanation.rule_id}: {t.outcomes[explanation.outcome]}
					</p>
					{#each ['all', 'any', 'except'] as group}
						{@const key = group as 'all' | 'any' | 'except'}
						{#if explanation[key].length}<p class="text-sm">
								{t[key]}: {explanation[key]
									.map((result, i) => {
										const rule =
											draft?.id === explanation.rule_id
												? draft
												: rules.find((r) => r.id === explanation.rule_id);
										const c = rule?.[key][i];
										const field = c ? (Object.keys(c)[0] as Field) : undefined;
										return `${field ? t.fields[field] + ': ' : ''}${t.outcomes[result]}`;
									})
									.join(', ')}
							</p>{/if}
					{/each}
				{/each}
			</div>
		{/each}
	</div>
</section>
