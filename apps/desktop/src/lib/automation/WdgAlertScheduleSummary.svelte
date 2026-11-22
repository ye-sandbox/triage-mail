<script lang="ts">
	import { scheduleLabels, type ScheduleState } from '$lib/alerts/schedule';
	import { i18n } from '$lib/i18n/index.svelte';

	interface Props {
		scheduleInfo: ScheduleState;
	}

	let { scheduleInfo }: Props = $props();
	const schedT = $derived(scheduleLabels[i18n.locale]);
</script>

<div class="space-y-1 text-[11px] text-secondary">
	<p>{schedT.notice}</p>
	<p>{schedT.dst}</p>
</div>

<div class="grid grid-cols-2 gap-2 rounded-md border border-separator bg-field/60 p-3 font-mono text-[11px] tabular-nums sm:grid-cols-4">
	<div>{schedT.deferred}: {scheduleInfo.deferred}</div>
	<div>{schedT.summarized}: {scheduleInfo.summarized}</div>
	<div>{schedT.expired}: {scheduleInfo.expired}</div>
	<div>{schedT.overflow}: {scheduleInfo.overflow}</div>
</div>
