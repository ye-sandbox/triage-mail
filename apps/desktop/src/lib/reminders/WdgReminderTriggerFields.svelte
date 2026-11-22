<script lang="ts">
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels } from './labels';

	interface Props {
		triggerKind: 'DueDate' | 'UnreadInterval';
		daysBefore: number;
		targetHour: number;
		intervalHours: number;
	}

	let {
		triggerKind,
		daysBefore = $bindable(),
		targetHour = $bindable(),
		intervalHours = $bindable()
	}: Props = $props();

	const t = $derived(labels[i18n.locale]);
	const PRESET_DAYS = [14, 7, 3, 1, 0];
</script>

{#if triggerKind === 'DueDate'}
	<div class="space-y-2">
		<span class="block font-medium text-secondary">{t.quickPresets}</span>
		<div class="flex flex-wrap gap-2">
			{#each PRESET_DAYS as days (days)}
				<button
					type="button"
					aria-pressed={daysBefore === days}
					onclick={() => (daysBefore = days)}
					class="min-h-[34px] rounded-md border px-3 py-1 font-mono text-xs font-medium tabular-nums transition-colors {daysBefore ===
					days
						? 'border-accent bg-selection text-foreground'
						: 'border-separator bg-field text-secondary hover:text-foreground'}"
				>
					{t.presetDaysLabel(days)}
				</button>
			{/each}
		</div>
	</div>

	<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
		<label class="flex flex-col gap-1">
			<span class="font-medium text-foreground">{t.daysBefore}</span>
			<input
				id="reminder-days-before"
				type="number"
				min="0"
				max="365"
				bind:value={daysBefore}
				class="h-10 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums text-foreground focus:border-accent focus:outline-none md:h-9"
			/>
		</label>
		<label class="flex flex-col gap-1">
			<span class="font-medium text-foreground">{t.targetHour}</span>
			<input
				id="reminder-target-hour"
				type="number"
				min="0"
				max="23"
				bind:value={targetHour}
				class="h-10 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums text-foreground focus:border-accent focus:outline-none md:h-9"
			/>
		</label>
	</div>
{:else}
	<label class="flex flex-col gap-1">
		<span class="font-medium text-foreground">{t.unreadHours}</span>
		<input
			id="reminder-unread-hours"
			type="number"
			min="1"
			max="8760"
			bind:value={intervalHours}
			class="h-10 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums text-foreground focus:border-accent focus:outline-none md:h-9"
		/>
	</label>
{/if}
