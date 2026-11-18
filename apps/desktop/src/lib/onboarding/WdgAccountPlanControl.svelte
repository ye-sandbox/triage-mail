<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import {
		MAX_CUSTOM_BADGE_CHARS,
		PLAN_BADGES,
		overrideToSelect,
		quotaPercent,
		quotaSummary,
		selectToOverride,
		type AccountSubscription,
		type BadgeOverride
	} from './subscriptions';

	type Props = {
		subscription?: AccountSubscription;
		failed?: boolean;
		onSetBadge: (override: BadgeOverride) => Promise<void>;
	};
	let { subscription, failed = false, onSetBadge }: Props = $props();

	const t = $derived(i18n.t.onboarding);
	let choice = $state<string | null>(null);
	let customLabel = $state('');
	const current = $derived(
		choice ?? (subscription ? overrideToSelect(subscription.badge_override) : 'auto')
	);
	const pending = $derived(selectToOverride(current, customLabel));

	function pick(event: Event) {
		choice = (event.currentTarget as HTMLSelectElement).value;
		if (choice !== 'custom') void apply();
	}

	async function apply() {
		if (!pending) return;
		await onSetBadge(pending);
		choice = null;
	}
</script>

<div class="flex flex-col gap-2 border-t border-separator px-4 py-3 text-xs">
	{#if subscription?.quota}
		{@const quota = subscription.quota}
		<div class="flex items-center gap-2" data-testid="quota-row">
			<span class="text-secondary">{t.storageUsage}</span>
			<div
				role="progressbar"
				aria-label={t.storageUsage}
				aria-valuemin="0"
				aria-valuemax="100"
				aria-valuenow={quotaPercent(quota)}
				class="h-1.5 w-32 overflow-hidden rounded bg-field"
			>
				<div class="h-full bg-accent" style="width: {quotaPercent(quota)}%"></div>
			</div>
			<span class="font-mono tabular-nums text-foreground">{quotaSummary(quota)}</span>
		</div>
	{/if}
	<div class="flex flex-wrap items-center gap-2">
		<label class="text-secondary" for="plan-select-{subscription?.account_id ?? 'x'}"
			>{t.planBadge}</label
		>
		<select
			id="plan-select-{subscription?.account_id ?? 'x'}"
			value={current}
			onchange={pick}
			class="h-8 rounded-md border border-separator bg-field px-2 text-xs text-foreground"
		>
			<option value="auto">{t.planAuto}</option>
			{#each PLAN_BADGES as badge (badge)}
				<option value="preset:{badge}">{badge.toUpperCase()}</option>
			{/each}
			<option value="custom">{t.planCustom}</option>
			<option value="hidden">{t.planHidden}</option>
		</select>
		{#if current === 'custom'}
			<input
				type="text"
				aria-label={t.planCustomLabel}
				maxlength={MAX_CUSTOM_BADGE_CHARS}
				bind:value={customLabel}
				class="h-8 w-32 rounded-md border border-separator bg-field px-2 text-xs text-foreground"
			/>
			<button
				type="button"
				disabled={!pending}
				onclick={() => void apply()}
				class="h-8 rounded-md border border-separator bg-surface px-2.5 text-xs font-medium text-foreground hover:bg-field disabled:opacity-50"
				>{t.planApply}</button
			>
		{/if}
	</div>
	{#if failed}
		<p role="alert" class="text-status-warning">{t.planError}</p>
	{/if}
</div>
