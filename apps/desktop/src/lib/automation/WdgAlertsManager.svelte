<script lang="ts">
	import { untrack } from 'svelte';
	import { Switch } from 'bits-ui';
	import {
		Bell,
		Clock,
		KeyRound,
		Pencil,
		Plus,
		RefreshCw,
		RotateCcw,
		Send,
		Trash2
	} from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import {
		nativeScheduleTransport,
		scheduleLabels,
		time,
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
	import WdgAlertDestinationDialog from './WdgAlertDestinationDialog.svelte';

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
	const schedT = $derived(scheduleLabels[i18n.locale]);

	let destinations = $state<Destination[]>([]);
	let history = $state<History[]>([]);
	let schedules = $state<Record<string, ScheduleState>>({});
	let dialogOpen = $state(false);
	let dialogInitialTab = $state<'destination' | 'schedule'>('destination');
	let editingDest = $state<Destination | null>(null);
	let busy = $state(false);
	let statusMessage = $state('');
	let errorMessage = $state('');
	let pending = $state<{
		kind: 'test' | 'replay';
		destination: Destination;
		ids: string[];
	} | null>(null);
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
		const [loadedDests, loadedHistory] = await Promise.all([
			alertsTransport.list(),
			alertsTransport.history()
		]);
		if (!active) return;
		destinations = loadedDests;
		history = loadedHistory;
		onDestinationsUpdated?.(loadedDests);

		const schedulePairs = await Promise.all(
			loadedDests.map(async (d) => {
				try {
					const st = await scheduleTransport.read(d.id);
					return [d.id, st] as const;
				} catch {
					return null;
				}
			})
		);
		if (!active) return;
		const nextMap: Record<string, ScheduleState> = {};
		for (const pair of schedulePairs) {
			if (pair) nextMap[pair[0]] = pair[1];
		}
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
			if (request.kind === 'test') {
				await alertsTransport.test(request.destination.id, request.destination.revision, true);
			} else {
				await alertsTransport.replay(
					request.ids,
					request.destination.id,
					request.destination.revision,
					true
				);
			}
			await load();
			if (active) {
				statusMessage = alertT.queued;
				toast.success(alertT.queued);
			}
		});
	}

	function requestRowReplay(row: History) {
		const dest = destinations.find((d) => d.id === row.destination_id);
		if (!dest || !dest.enabled) {
			errorMessage = alertT.error;
			return;
		}
		const targetIds = selectedIds.length ? [...selectedIds] : [row.event_id];
		pending = {
			kind: 'replay',
			destination: clonePlain(dest),
			ids: targetIds
		};
	}

	function statusBadgeClass(status: History['status']): string {
		switch (status) {
			case 'Accepted':
				return 'border-status-healthy/40 bg-status-healthy/10 text-status-healthy';
			case 'Pending':
				return 'border-status-warning/40 bg-status-warning/10 text-status-warning';
			case 'Failed':
			case 'Expired':
			case 'Unavailable':
				return 'border-status-danger/40 bg-status-danger/10 text-status-danger';
			case 'Canceled':
				return 'border-separator bg-field text-secondary';
		}
	}

	function kindLabel(kind: Destination['kind']): string {
		if (kind === 'Local') return alertT.local;
		return kind;
	}
</script>

<div class="flex flex-col gap-5">
	<section class="flex flex-col gap-3" aria-label={alertT.title}>
		<div class="flex flex-wrap items-center justify-between gap-2">
			<div class="space-y-0.5">
				<h2 class="font-display text-sm font-semibold text-foreground">{t.title}</h2>
				<p class="text-xs text-secondary">{t.subtitle}</p>
				<p class="text-[11px] text-secondary">{alertT.notice}</p>
			</div>
			<button
				type="button"
				disabled={busy}
				onclick={openCreate}
				class="flex min-h-[44px] items-center gap-1.5 rounded-md bg-accent px-3.5 py-1.5 text-xs font-medium whitespace-nowrap text-accent-foreground hover:opacity-95 active:scale-[0.98] disabled:opacity-45 md:min-h-[36px]"
			>
				<Plus class="h-3.5 w-3.5" />
				<span>{t.newDestination}</span>
			</button>
		</div>

		{#if errorMessage}
			<p
				role="alert"
				class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger"
			>
				{errorMessage}
			</p>
		{/if}
		{#if statusMessage}
			<p
				role="status"
				class="rounded-md border border-status-info/40 bg-status-info/10 px-3 py-2 text-xs text-foreground"
			>
				{statusMessage}
			</p>
		{/if}

		{#if pending}
			<div
				role="group"
				aria-label={alertT.send}
				class="flex flex-col gap-2.5 rounded-lg border border-accent bg-selection-soft/70 p-4 text-xs"
			>
				<p class="font-medium text-foreground">
					{pending.kind === 'test' ? alertT.confirmTest : alertT.confirmReplay}
					<span class="font-mono">
						({pending.destination.name} · {pending.destination.public_origin ?? alertT.local})
					</span>
				</p>
				<p class="text-[11px] text-secondary">{schedT.bypass}</p>
				<p class="font-mono text-[11px] text-foreground">
					{alertT.fields}:
					{pending.destination.fields.map((f) => alertT.fieldNames[f]).join(', ') || t.localNoExport}
				</p>
				<div class="flex flex-wrap gap-2">
					<button
						type="button"
						disabled={busy}
						onclick={() => void confirmDelivery()}
						class="min-h-[34px] rounded-md bg-accent px-3.5 py-1 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-45"
					>
						{alertT.send}
					</button>
					<button
						type="button"
						disabled={busy}
						onclick={() => (pending = null)}
						class="min-h-[34px] rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-secondary hover:text-foreground"
					>
						{alertT.cancel}
					</button>
				</div>
			</div>
		{/if}

		{#if destinations.length === 0}
			<div class="rounded-lg border border-separator bg-surface p-6 text-center text-xs text-secondary">
				{alertT.empty}
			</div>
		{:else}
			<div class="grid grid-cols-1 gap-3 lg:grid-cols-3">
				{#each destinations as dest (dest.id)}
					{@const sched = schedules[dest.id]}
					<article
						class="flex flex-col justify-between gap-3 rounded-lg border border-separator bg-surface p-4 text-xs"
					>
						<div class="space-y-2">
							<div class="flex items-start justify-between gap-2">
								<div class="min-w-0">
									<div class="flex items-center gap-2">
										<Bell class="h-3.5 w-3.5 shrink-0 text-accent" />
										<h3 class="truncate font-display text-sm font-semibold text-foreground">
											{dest.name}
										</h3>
									</div>
									<div
										class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 font-mono text-[11px] text-secondary"
									>
										<span>{kindLabel(dest.kind)}</span>
										<span class="inline-flex items-center gap-1.5 whitespace-nowrap tabular-nums">
											<span aria-hidden="true">·</span>
											<span>rev {dest.revision}</span>
										</span>
										{#if dest.public_origin}
											<span class="inline-flex min-w-0 items-center gap-1.5">
												<span aria-hidden="true">·</span>
												<span class="truncate">{dest.public_origin}</span>
											</span>
										{/if}
									</div>
								</div>

								<Switch.Root
									checked={dest.enabled}
									disabled={busy}
									onCheckedChange={() => void handleToggle(dest)}
									aria-label={t.toggleDestinationAria(dest.name)}
									class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-field disabled:opacity-45"
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
									<KeyRound class="h-3 w-3" />
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
								<div class="mt-0.5 tabular-nums">
									{schedT.quiet}:
									{sched?.policy.quiet
										? t.scheduleQuiet(time(sched.policy.quiet.start), time(sched.policy.quiet.end))
										: t.scheduleOff}
								</div>
							</div>
						</div>

						<div class="flex flex-wrap items-center justify-between gap-2 border-t border-separator pt-3">
							<div class="flex flex-wrap items-center gap-1.5">
								<button
									type="button"
									disabled={busy}
									onclick={() => openEdit(dest, 'destination')}
									class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground hover:bg-field disabled:opacity-45"
								>
									<Pencil class="h-3 w-3" />
									<span>{alertT.edit}</span>
								</button>
								<button
									type="button"
									disabled={busy}
									onclick={() => openEdit(dest, 'schedule')}
									class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground hover:bg-field disabled:opacity-45"
								>
									<Clock class="h-3 w-3" />
									<span>{t.scheduleButton}</span>
								</button>
								<button
									type="button"
									disabled={busy}
									onclick={() => void handleDelete(dest)}
									aria-label={`${alertT.remove} ${dest.name}`}
									class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-2 py-1 text-xs text-status-danger hover:bg-field disabled:opacity-45"
								>
									<Trash2 class="h-3 w-3" />
								</button>
							</div>

							<div class="flex flex-wrap items-center gap-1.5">
								<button
									type="button"
									disabled={busy || !dest.enabled}
									onclick={() => (pending = { kind: 'test', destination: dest, ids: [] })}
									class="flex min-h-[32px] items-center gap-1.5 rounded-md border border-separator bg-selection-soft px-2.5 py-1 text-xs font-medium text-foreground hover:bg-selection disabled:opacity-45"
								>
									<Send class="h-3 w-3 text-accent" />
									<span>{alertT.test}</span>
								</button>
								<button
									type="button"
									disabled={busy || !dest.enabled || !selectedIds.length}
									onclick={() =>
										(pending = {
											kind: 'replay',
											destination: dest,
											ids: [...selectedIds]
										})}
									class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
								>
									<RotateCcw class="h-3 w-3" />
									<span>{alertT.replay} ({selectedIds.length})</span>
								</button>
							</div>
						</div>
					</article>
				{/each}
			</div>
		{/if}
	</section>

	<section class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<div>
				<h3 class="font-display text-sm font-semibold text-foreground">{t.historyTitle}</h3>
				<p class="text-xs text-secondary">{t.historySubtitle}</p>
			</div>
			<button
				type="button"
				disabled={busy}
				onclick={() => void operation(load)}
				class="flex min-h-[34px] items-center gap-1.5 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
			>
				<RefreshCw class="h-3 w-3" />
				<span>{t.refreshHistory}</span>
			</button>
		</div>

		{#if history.length === 0}
			<p class="rounded border border-separator bg-field p-4 text-center text-xs text-secondary">
				{t.emptyHistory}
			</p>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full border-collapse text-left text-xs">
					<thead>
						<tr class="border-b border-separator text-[11px] font-medium text-secondary">
							<th class="py-2 pr-3 font-mono">{t.colEventId}</th>
							<th class="py-2 pr-3 font-mono">{t.colDestination}</th>
							<th class="py-2 pr-3">{t.colStatus}</th>
							<th class="py-2 pr-3 text-right">{t.colAttempts}</th>
							<th class="py-2 pr-3">{t.colTimestamp}</th>
							<th class="py-2 pr-3">{t.colDetail}</th>
							<th class="py-2 text-right">{t.colAction}</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-separator">
						{#each history as row (row.event_id)}
							{@const dest = destinations.find((d) => d.id === row.destination_id)}
							{@const meta = historyMeta[row.event_id]}
							<tr class="hover:bg-field/50">
								<td class="py-2.5 pr-3 font-mono text-[11px] tabular-nums text-foreground">
									{row.event_id}
								</td>
								<td class="py-2.5 pr-3 font-mono text-[11px] text-secondary">
									{dest?.name ?? row.destination_id}
								</td>
								<td class="py-2.5 pr-3">
									<span
										class="inline-block rounded-md border px-2 py-0.5 font-mono text-[11px] font-medium {statusBadgeClass(
											row.status
										)}"
									>
										{alertT.status[row.status]}
									</span>
								</td>
								<td class="py-2.5 pr-3 text-right font-mono tabular-nums text-foreground">
									{row.attempts}
								</td>
								<td class="py-2.5 pr-3 font-mono text-[11px] tabular-nums text-secondary">
									{new Date(row.created).toLocaleString(i18n.locale)}
								</td>
								<td class="py-2.5 pr-3 text-secondary">{meta?.detail ?? '—'}</td>
								<td class="py-2.5 text-right">
									<button
										type="button"
										disabled={busy || !dest?.enabled}
										onclick={() => requestRowReplay(row)}
										class="inline-flex min-h-[30px] items-center gap-1 rounded border border-separator bg-surface px-2.5 py-1 text-[11px] font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
									>
										<RotateCcw class="h-3 w-3" />
										<span>{t.replayButton}</span>
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>
</div>

<WdgAlertDestinationDialog
	bind:open={dialogOpen}
	initialTab={dialogInitialTab}
	destination={editingDest}
	{alertsTransport}
	{scheduleTransport}
	{guard}
	onSaved={async () => {
		await operation(load);
	}}
	onScheduleSaved={async () => {
		await operation(load);
	}}
/>
