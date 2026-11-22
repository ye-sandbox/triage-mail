<script lang="ts">
	import { Switch } from 'bits-ui';
	import Bell from '@lucide/svelte/icons/bell';
	import Clock from '@lucide/svelte/icons/clock';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Pencil from '@lucide/svelte/icons/pencil';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Send from '@lucide/svelte/icons/send';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import { scheduleLabels, time, type ScheduleState } from '$lib/alerts/schedule';
	import type { Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { automationLabels } from './labels';

	interface Props {
		dest: Destination;
		sched?: ScheduleState;
		busy: boolean;
		selectedIdsCount: number;
		onToggle: (dest: Destination) => void | Promise<void>;
		onEdit: (dest: Destination, tab: 'destination' | 'schedule') => void;
		onDelete: (dest: Destination) => void | Promise<void>;
		onTest: (dest: Destination) => void;
		onReplay: (dest: Destination) => void;
	}

	let {
		dest,
		sched,
		busy,
		selectedIdsCount,
		onToggle,
		onEdit,
		onDelete,
		onTest,
		onReplay
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);
	const schedT = $derived(scheduleLabels[i18n.locale]);

	function kindLabel(kind: Destination['kind']): string {
		return kind === 'Local' ? alertT.local : kind;
	}
</script>

<article class="flex flex-col justify-between gap-3 rounded-lg border border-separator bg-surface p-4 text-xs">
	<div class="space-y-2">
		<div class="flex items-start justify-between gap-2">
			<div class="min-w-0">
				<div class="flex items-center gap-2">
					<Bell class="h-3.5 w-3.5 shrink-0 text-accent" aria-hidden="true" />
					<h3 class="truncate font-display text-sm font-semibold text-foreground">{dest.name}</h3>
				</div>
				<div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 font-mono text-[11px] text-secondary">
					<span>{kindLabel(dest.kind)}</span>
					<span class="inline-flex items-center gap-2 whitespace-nowrap tabular-nums">
						<span aria-hidden="true">·</span>
						<span>rev {dest.revision}</span>
					</span>
					{#if dest.public_origin}
						<span class="inline-flex min-w-0 items-center gap-2">
							<span aria-hidden="true">·</span>
							<span class="truncate">{dest.public_origin}</span>
						</span>
					{/if}
				</div>
			</div>

			<Switch.Root
				checked={dest.enabled}
				disabled={busy}
				onCheckedChange={() => void onToggle(dest)}
				aria-label={t.toggleDestinationAria(dest.name)}
				class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-field disabled:opacity-45"
			>
				<Switch.Thumb
					class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
				/>
			</Switch.Root>
		</div>

		<div class="flex flex-wrap items-center gap-2 text-[11px]">
			<span
				class="inline-flex items-center gap-1 font-mono {dest.has_credentials
					? 'text-status-healthy'
					: 'text-secondary'}"
			>
				<KeyRound class="h-3 w-3" aria-hidden="true" />
				{dest.has_credentials ? t.credentialsSealed : t.credentialsNone}
			</span>
		</div>

		<div class="rounded border border-separator bg-field/60 p-2 font-mono text-[11px] text-secondary">
			{#if dest.kind === 'Local'}
				<div>{t.localNoExport}</div>
			{:else}
				<div>
					{t.fieldsCount(dest.fields.length)}:
					{dest.fields.map((f) => alertT.fieldNames[f]).join(', ') || '—'}
				</div>
			{/if}
			<div class="mt-1 tabular-nums">
				{schedT.quiet}:
				{sched?.policy.quiet
					? t.scheduleQuiet(time(sched.policy.quiet.start), time(sched.policy.quiet.end))
					: t.scheduleOff}
			</div>
		</div>
	</div>

	<div class="flex flex-wrap items-center justify-between gap-2 border-t border-separator pt-3">
		<div class="flex flex-wrap items-center gap-2">
			<button
				type="button"
				disabled={busy}
				onclick={() => onEdit(dest, 'destination')}
				class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-field disabled:opacity-45"
			>
				<Pencil class="h-3 w-3" aria-hidden="true" />
				<span>{alertT.edit}</span>
			</button>
			<button
				type="button"
				disabled={busy}
				onclick={() => onEdit(dest, 'schedule')}
				class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-field disabled:opacity-45"
			>
				<Clock class="h-3 w-3" aria-hidden="true" />
				<span>{t.scheduleButton}</span>
			</button>
			<button
				type="button"
				disabled={busy}
				onclick={() => void onDelete(dest)}
				aria-label={`${alertT.remove} ${dest.name}`}
				class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-2 py-1 text-xs text-status-danger hover:bg-field disabled:opacity-45"
			>
				<Trash2 class="h-3 w-3" aria-hidden="true" />
			</button>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<button
				type="button"
				disabled={busy || !dest.enabled}
				onclick={() => onTest(dest)}
				class="flex min-h-[32px] items-center gap-2 rounded-md border border-separator bg-selection-soft px-3 py-1 text-xs font-medium text-foreground hover:bg-selection disabled:opacity-45"
			>
				<Send class="h-3 w-3 text-accent" aria-hidden="true" />
				<span>{alertT.test}</span>
			</button>
			<button
				type="button"
				disabled={busy || !dest.enabled || !selectedIdsCount}
				onclick={() => onReplay(dest)}
				class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
			>
				<RotateCcw class="h-3 w-3" aria-hidden="true" />
				<span>{alertT.replay} ({selectedIdsCount})</span>
			</button>
		</div>
	</div>
</article>
