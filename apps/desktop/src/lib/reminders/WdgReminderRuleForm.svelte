<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { Destination } from '$lib/alerts/types';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels } from './labels';
	import { buildValidatedReminderConfig } from './rule-form';
	import type { ScheduledReminderConfig } from './types';
	import WdgReminderTriggerFields from './WdgReminderTriggerFields.svelte';

	interface Props {
		initial: ScheduledReminderConfig;
		destinations: Destination[];
		accounts?: AccountConfig[];
		busy?: boolean;
		error?: string;
		onSave: (next: ScheduledReminderConfig) => void | Promise<void>;
		onCancel: () => void;
	}

	let {
		initial,
		destinations,
		accounts = [],
		busy = false,
		error = '',
		onSave,
		onCancel
	}: Props = $props();

	const t = $derived(labels[i18n.locale]);

	let name = $state(untrack(() => initial.name));
	let enabled = $state(untrack(() => initial.enabled));
	let triggerKind = $state<'DueDate' | 'UnreadInterval'>(
		untrack(() => ('DueDate' in initial.trigger ? 'DueDate' : 'UnreadInterval'))
	);
	let daysBefore = $state(
		untrack(() => ('DueDate' in initial.trigger ? initial.trigger.DueDate.days_before : 2))
	);
	let targetHour = $state(
		untrack(() => ('DueDate' in initial.trigger ? initial.trigger.DueDate.target_hour : 9))
	);
	let intervalHours = $state(
		untrack(() =>
			'UnreadInterval' in initial.trigger ? initial.trigger.UnreadInterval.interval_hours : 24
		)
	);
	let minAmountStr = $state(
		untrack(() => {
			const cents =
				'DueDate' in initial.trigger
					? initial.trigger.DueDate.min_amount_cents
					: initial.trigger.UnreadInterval.min_amount_cents;
			return cents ? (cents / 100).toFixed(2) : '';
		})
	);
	let selectedAccount = $state(untrack(() => initial.account_id ?? 'All'));
	let selectedDestinations = $state<string[]>(untrack(() => [...initial.destination_ids]));
	let localError = $state('');

	function toggleDestination(id: string) {
		selectedDestinations = selectedDestinations.includes(id)
			? selectedDestinations.filter((item) => item !== id)
			: [...selectedDestinations, id];
	}

	function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		localError = '';
		const result = buildValidatedReminderConfig(
			{
				initial: $state.snapshot(initial),
				name,
				enabled,
				triggerKind,
				daysBefore,
				targetHour,
				intervalHours,
				minAmountStr,
				selectedAccount,
				selectedDestinations
			},
			t
		);
		if (!result.ok) {
			localError = result.error;
			return;
		}
		void onSave(result.value);
	}
</script>

<form onsubmit={handleSubmit} class="flex flex-col gap-4 text-xs">
	<div class="flex items-center justify-between border-b border-separator pb-2">
		<h3 class="font-display text-xs font-semibold text-foreground">
			{initial.revision > 0 ? t.editRule : t.addRule}
		</h3>
		<span class="font-mono text-[11px] text-secondary">Rev. {initial.revision}</span>
	</div>

	{#if localError || error}
		<div
			role="alert"
			class="rounded-md border border-status-danger/30 bg-status-danger/10 px-3 py-2 text-xs text-status-danger"
		>
			{localError || error}
		</div>
	{/if}

	<label class="flex flex-col gap-1">
		<span class="font-medium text-foreground">{t.ruleName}</span>
		<input
			id="reminder-name"
			type="text"
			bind:value={name}
			placeholder={t.ruleNamePlaceholder}
			maxlength={120}
			required
			class="h-10 rounded-md border border-separator bg-field px-3 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none md:h-9"
		/>
	</label>

	<label class="flex flex-col gap-1">
		<span class="font-medium text-foreground">{t.triggerType}</span>
		<select
			id="reminder-trigger-type"
			bind:value={triggerKind}
			class="h-10 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none md:h-9"
		>
			<option value="DueDate">{t.triggerDueDate}</option>
			<option value="UnreadInterval">{t.triggerUnread}</option>
		</select>
	</label>

	<WdgReminderTriggerFields
		{triggerKind}
		bind:daysBefore
		bind:targetHour
		bind:intervalHours
	/>

	<label class="flex flex-col gap-1">
		<span class="font-medium text-foreground">{t.minAmount}</span>
		<input
			id="reminder-min-amount"
			type="text"
			bind:value={minAmountStr}
			placeholder={t.minAmountPlaceholder}
			class="h-10 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums text-foreground placeholder:text-secondary focus:border-accent focus:outline-none md:h-9"
		/>
	</label>

	{#if accounts.length > 1}
		<label class="flex flex-col gap-1">
			<span class="font-medium text-foreground">{t.accountScope}</span>
			<select
				id="reminder-account"
				bind:value={selectedAccount}
				class="h-10 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none md:h-9"
			>
				<option value="All">{t.allAccounts}</option>
				{#each accounts as acc (acc.account_id)}
					<option value={acc.account_id}>{acc.email_display}</option>
				{/each}
			</select>
		</label>
	{/if}

	<div class="space-y-2">
		<span class="block font-medium text-foreground">{t.destinations}</span>
		{#if destinations.length === 0}
			<p class="text-xs italic text-secondary">{t.noDestinations}</p>
		{:else}
			<div class="space-y-2 rounded-md border border-separator bg-field p-3">
				{#each destinations as dest (dest.id)}
					<label class="flex cursor-pointer items-center gap-2 text-xs text-foreground">
						<input
							type="checkbox"
							checked={selectedDestinations.includes(dest.id)}
							onchange={() => toggleDestination(dest.id)}
							class="h-3.5 w-3.5 accent-accent"
						/>
						<span class="font-medium">{dest.name}</span>
						<span class="text-secondary">({dest.kind})</span>
					</label>
				{/each}
			</div>
		{/if}
	</div>

	<div class="flex items-center justify-end gap-2 border-t border-separator pt-3">
		<button
			type="button"
			onclick={onCancel}
			disabled={busy}
			class="min-h-[44px] rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground disabled:opacity-50 md:min-h-[36px]"
		>
			{t.cancel}
		</button>
		<button
			type="submit"
			disabled={busy || destinations.length === 0 || selectedDestinations.length === 0}
			class="min-h-[44px] rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50 md:min-h-[36px]"
		>
			{t.save}
		</button>
	</div>
</form>
