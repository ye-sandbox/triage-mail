<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import { scheduleLabels, minutes, time, type SchedulePolicy } from './schedule';
	let {
		policy,
		change
	}: { policy: SchedulePolicy; change: (update: Partial<SchedulePolicy>) => void } = $props();
	const t = $derived(scheduleLabels[i18n.locale]);
</script>

<label class="block"
	>{t.cadence}
	<select
		value={typeof policy.cadence === 'string'
			? 'Immediate'
			: 'Hourly' in policy.cadence
				? 'Hourly'
				: 'Daily'}
		onchange={(e) =>
			change({
				cadence:
					e.currentTarget.value === 'Immediate'
						? 'Immediate'
						: e.currentTarget.value === 'Hourly'
							? { Hourly: { minute: 0 } }
							: { Daily: { minute: 540 } }
			})}
	>
		<option value="Immediate">{t.immediate}</option><option value="Hourly">{t.hourly}</option
		><option value="Daily">{t.daily}</option>
	</select>
</label>
{#if typeof policy.cadence !== 'string' && 'Hourly' in policy.cadence}
	<label class="block"
		>{t.minute}<input
			type="number"
			min="0"
			max="59"
			required
			value={policy.cadence.Hourly.minute}
			oninput={(e) =>
				change({
					cadence: {
						Hourly: { minute: Number(e.currentTarget.value) }
					}
				})}
		/></label
	>
{:else if typeof policy.cadence !== 'string' && 'Daily' in policy.cadence}
	<label class="block"
		>{t.time}<input
			type="time"
			required
			value={time(policy.cadence.Daily.minute)}
			oninput={(e) =>
				change({
					cadence: {
						Daily: { minute: minutes(e.currentTarget.value) }
					}
				})}
		/></label
	>
{/if}
<label class="block"
	>{t.zone}<select
		value={policy.zone}
		onchange={(e) =>
			change({
				zone: e.currentTarget.value === 'Utc' ? 'Utc' : 'SystemLocal'
			})}><option value="SystemLocal">{t.local}</option><option value="Utc">{t.utc}</option></select
	></label
>
<label class="flex gap-2"
	><input
		type="checkbox"
		checked={policy.quiet !== null}
		onchange={(e) =>
			change({
				quiet: e.currentTarget.checked ? { start: 1320, end: 420 } : null
			})}
	/>{t.quiet}</label
>
{#if policy.quiet}
	<label class="block"
		>{t.start}<input
			type="time"
			required
			value={time(policy.quiet.start)}
			oninput={(e) =>
				policy?.quiet &&
				change({
					quiet: {
						...policy.quiet,
						start: minutes(e.currentTarget.value)
					}
				})}
		/></label
	>
	<label class="block"
		>{t.end}<input
			type="time"
			required
			value={time(policy.quiet.end)}
			oninput={(e) =>
				policy?.quiet &&
				change({
					quiet: {
						...policy.quiet,
						end: minutes(e.currentTarget.value)
					}
				})}
		/></label
	>
{/if}
