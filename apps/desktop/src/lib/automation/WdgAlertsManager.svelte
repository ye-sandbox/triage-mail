<script lang="ts">
	import { untrack } from 'svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import { toast } from 'svelte-sonner';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import {
		nativeScheduleTransport,
		type ScheduleState,
		type ScheduleTransport
	} from '$lib/alerts/schedule';
	import { nativeAlertsTransport } from '$lib/alerts/transport';
	import type { AlertsTransport, Destination, History } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { SettingsFormGuard } from '$lib/settings/guards';
	import type { HistoryVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';
	import { clonePlain } from './simulator';
	import WdgAlertConfirmBanner from './WdgAlertConfirmBanner.svelte';
	import WdgAlertDestinationCard from './WdgAlertDestinationCard.svelte';
	import WdgAlertDestinationDialog from './WdgAlertDestinationDialog.svelte';
	import WdgAlertHistoryTable from './WdgAlertHistoryTable.svelte';

	interface Props {
		alertsTransport?: AlertsTransport;
		scheduleTransport?: ScheduleTransport;
		guard?: SettingsFormGuard;
		selectedIds?: string[];
		historyMeta?: Record<string, HistoryVisualMeta>;
		onDestinationsUpdated?: (destinations: Destination[]) => void;
	}

	let {
		alertsTransport = nativeAlertsTransport,
		scheduleTransport = nativeScheduleTransport,
		guard = new SettingsFormGuard(),
		selectedIds = [],
		historyMeta = {},
		onDestinationsUpdated
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);

	let destinations = $state<Destination[]>([]);
	let history = $state<History[]>([]);
	let schedules = $state<Record<string, ScheduleState>>({});
	let dialogOpen = $state(false);
	let dialogInitialTab = $state<'destination' | 'schedule'>('destination');
	let editingDest = $state<Destination | null>(null);
	let busy = $state(false);
	let statusMessage = $state('');
	let errorMessage = $state('');
	let pending = $state<{ kind: 'test' | 'replay'; destination: Destination; ids: string[] } | null>(null);
	let active = false;

	async function operation(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		errorMessage = '';
		try {
			await work();
		} catch {
			if (active) errorMessage = alertT.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}

	async function load() {
		const [loadedDests, loadedHistory] = await Promise.all([alertsTransport.list(), alertsTransport.history()]);
		if (!active) return;
		destinations = loadedDests;
		history = loadedHistory;
		onDestinationsUpdated?.(loadedDests);
		const schedulePairs = await Promise.all(
			loadedDests.map(async (d) => {
				try {
					return [d.id, await scheduleTransport.read(d.id)] as const;
				} catch {
					return null;
				}
			})
		);
		if (!active) return;
		const nextMap: Record<string, ScheduleState> = {};
		for (const pair of schedulePairs) if (pair) nextMap[pair[0]] = pair[1];
		schedules = nextMap;
	}

	$effect(() => {
		active = true;
		untrack(() => void operation(load));
		return () => {
			active = false;
			guard.reset();
		};
	});

	function openCreate() {
		pending = null;
		editingDest = null;
		dialogInitialTab = 'destination';
		dialogOpen = true;
	}

	function openEdit(dest: Destination, tab: 'destination' | 'schedule' = 'destination') {
		pending = null;
		editingDest = clonePlain(dest);
		dialogInitialTab = tab;
		dialogOpen = true;
	}

	async function handleToggle(dest: Destination) {
		if (dest.enabled) {
			const snapshot = $state.snapshot(dest);
			await operation(async () => {
				const saved = await alertsTransport.save({ ...snapshot, enabled: false }, null, false);
				await load();
				if (active) toast.info(t.destinationToggledToast(saved.name, saved.enabled));
			});
			return;
		}
		if (dest.kind === 'Local') {
			const avail = await alertsTransport.available().catch(() => false);
			if (!avail) {
				errorMessage = alertT.unavailable;
				toast.error(alertT.unavailable);
				return;
			}
		}
		openEdit({ ...clonePlain(dest), enabled: true }, 'destination');
	}

	async function handleDelete(dest: Destination) {
		pending = null;
		await operation(async () => {
			await alertsTransport.remove(dest.id, dest.revision);
			await load();
			if (active) toast.success(t.destinationDeletedToast(dest.name));
		});
	}

	async function confirmDelivery() {
		const request = pending;
		if (!request) return;
		pending = null;
		await operation(async () => {
			if (request.kind === 'test') await alertsTransport.test(request.destination.id, request.destination.revision, true);
			else await alertsTransport.replay(request.ids, request.destination.id, request.destination.revision, true);
			await load();
			if (active) {
				statusMessage = alertT.queued;
				toast.success(alertT.queued);
			}
		});
	}

	function requestRowReplay(row: History) {
		const dest = destinations.find((d) => d.id === row.destination_id);
		if (!dest || !dest.enabled) { errorMessage = alertT.error; return; }
		pending = { kind: 'replay', destination: clonePlain(dest), ids: selectedIds.length ? [...selectedIds] : [row.event_id] };
	}
</script>

<div class="flex flex-col gap-5">
	<section class="flex flex-col gap-3" aria-label={alertT.title}>
		<div class="flex flex-wrap items-center justify-between gap-2">
			<div class="space-y-1">
				<h2 class="font-display text-sm font-semibold text-foreground">{t.title}</h2>
				<p class="text-xs text-secondary">{t.subtitle}</p>
				<p class="text-[11px] text-secondary">{alertT.notice}</p>
			</div>
			<button type="button" disabled={busy} onclick={openCreate} class="flex min-h-[44px] items-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium whitespace-nowrap text-accent-foreground hover:opacity-95 active:scale-[0.98] disabled:opacity-45 md:min-h-[36px]">
				<Plus class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{t.newDestination}</span>
			</button>
		</div>

		{#if errorMessage}
			<p role="alert" class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger">
				{errorMessage}
			</p>
		{/if}
		{#if statusMessage}
			<p role="status" class="rounded-md border border-status-info/40 bg-status-info/10 px-3 py-2 text-xs text-foreground">
				{statusMessage}
			</p>
		{/if}

		{#if pending}
			<WdgAlertConfirmBanner
				{pending}
				{busy}
				onConfirm={confirmDelivery}
				onCancel={() => (pending = null)}
			/>
		{/if}

		{#if destinations.length === 0}
			<div class="rounded-lg border border-separator bg-surface p-6 text-center text-xs text-secondary">
				{alertT.empty}
			</div>
		{:else}
			<div class="grid grid-cols-1 gap-3 lg:grid-cols-3">
				{#each destinations as dest (dest.id)}
					<WdgAlertDestinationCard
						{dest}
						sched={schedules[dest.id]}
						{busy}
						selectedIdsCount={selectedIds.length}
						onToggle={handleToggle}
						onEdit={openEdit}
						onDelete={handleDelete}
						onTest={(d) => (pending = { kind: 'test', destination: d, ids: [] })}
						onReplay={(d) => (pending = { kind: 'replay', destination: d, ids: [...selectedIds] })}
					/>
				{/each}
			</div>
		{/if}
	</section>

	<WdgAlertHistoryTable
		{history}
		{destinations}
		{historyMeta}
		{busy}
		onRefresh={() => operation(load)}
		onReplayRow={requestRowReplay}
	/>
</div>

<WdgAlertDestinationDialog
	bind:open={dialogOpen}
	initialTab={dialogInitialTab}
	destination={editingDest}
	{alertsTransport}
	{scheduleTransport}
	{guard}
	onSaved={() => operation(load)}
	onScheduleSaved={() => operation(load)}
/>
