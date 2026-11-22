<script lang="ts">
	import Clock from '@lucide/svelte/icons/clock';
	import { minutes, scheduleLabels, time, type SchedulePolicy } from '$lib/alerts/schedule';
	import { i18n } from '$lib/i18n/index.svelte';

	interface Props {
		schedulePolicy: SchedulePolicy;
		scheduleConsent: boolean;
		onPatch: (update: Partial<SchedulePolicy>) => void;
		onConsentChange: (checked: boolean) => void;
	}

	let { schedulePolicy, scheduleConsent, onPatch, onConsentChange }: Props = $props();
	const schedT = $derived(scheduleLabels[i18n.locale]);
</script>

<div class="space-y-2 rounded-md border border-separator bg-field/60 p-3">
	<div class="flex items-center justify-between">
		<span class="flex items-center gap-2 font-medium text-foreground">
			<Clock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{schedT.quiet}</span>
		</span>
		<input
			type="checkbox"
			checked={schedulePolicy.quiet !== null}
			onchange={(e) =>
				onPatch({ quiet: e.currentTarget.checked ? { start: 1320, end: 420 } : null })}
			aria-label={schedT.quiet}
			class="h-4 w-4 accent-accent"
		/>
	</div>
	{#if schedulePolicy.quiet}
		<div class="grid grid-cols-2 gap-2">
			<label class="flex flex-col gap-1">
				<span class="text-[11px] text-secondary">{schedT.start}</span>
				<input
					type="time"
					required
					value={time(schedulePolicy.quiet.start)}
					oninput={(e) =>
						schedulePolicy.quiet &&
						onPatch({
							quiet: { ...schedulePolicy.quiet, start: minutes(e.currentTarget.value) }
						})}
					class="h-8 rounded border border-separator bg-surface px-2 font-mono text-xs tabular-nums"
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-[11px] text-secondary">{schedT.end}</span>
				<input
					type="time"
					required
					value={time(schedulePolicy.quiet.end)}
					oninput={(e) =>
						schedulePolicy.quiet &&
						onPatch({
							quiet: { ...schedulePolicy.quiet, end: minutes(e.currentTarget.value) }
						})}
					class="h-8 rounded border border-separator bg-surface px-2 font-mono text-xs tabular-nums"
				/>
			</label>
		</div>
	{/if}
</div>

<label class="flex cursor-pointer items-center gap-2">
	<input
		type="checkbox"
		checked={schedulePolicy.urgent_bypass}
		onchange={(e) => onPatch({ urgent_bypass: e.currentTarget.checked })}
		class="h-3.5 w-3.5 accent-accent"
	/>
	<span>{schedT.urgent}</span>
</label>

{#if schedulePolicy.urgent_bypass}
	<label class="flex cursor-pointer items-start gap-2 rounded-md border border-separator bg-selection-soft/60 p-3">
		<input
			type="checkbox"
			required
			checked={scheduleConsent}
			onchange={(e) => onConsentChange(e.currentTarget.checked)}
			class="mt-1 h-4 w-4 accent-accent"
		/>
		<span class="text-[11px] leading-relaxed text-foreground">{schedT.consent}</span>
	</label>
{/if}
