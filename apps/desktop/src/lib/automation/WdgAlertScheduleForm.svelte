<script lang="ts">
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import {
		minutes,
		scheduleLabels,
		time,
		type SchedulePolicy,
		type SchedulePreview,
		type ScheduleState,
		type ScheduleTransport
	} from '$lib/alerts/schedule';
	import type { Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SettingsFormGuard } from '$lib/settings/guards';
	import { automationLabels } from './labels';
	import { clonePlain } from './simulator';
	import WdgAlertQuietHoursFields from './WdgAlertQuietHoursFields.svelte';
	import WdgAlertScheduleSummary from './WdgAlertScheduleSummary.svelte';

	interface Props {
		open: boolean;
		destination: Destination | null;
		scheduleTransport: ScheduleTransport;
		guard?: SettingsFormGuard;
		busy: boolean;
		onSwitchToDestination: () => void;
		onScheduleSaved?: (saved: SchedulePolicy) => void | Promise<void>;
		onClose: () => void;
	}

	let {
		open,
		destination,
		scheduleTransport,
		guard,
		busy = $bindable(),
		onSwitchToDestination,
		onScheduleSaved,
		onClose
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const schedT = $derived(scheduleLabels[i18n.locale]);

	let scheduleInfo = $state<ScheduleState | null>(null);
	let schedulePolicy = $state<SchedulePolicy | null>(null);
	let schedulePreview = $state<SchedulePreview | null>(null);
	let scheduleConsent = $state(false);
	let scheduleError = $state('');
	let scheduleGen = 0;

	async function loadScheduleFor(destId: string) {
		const gen = ++scheduleGen;
		scheduleError = '';
		schedulePreview = null;
		scheduleConsent = false;
		try {
			const loaded = await scheduleTransport.read(destId);
			if (gen === scheduleGen && open) {
				scheduleInfo = loaded;
				schedulePolicy = clonePlain(loaded.policy);
			}
		} catch {
			if (gen === scheduleGen && open) scheduleError = schedT.error;
		}
	}

	$effect(() => {
		if (open) {
			untrack(() => {
				scheduleError = '';
				schedulePreview = null;
				scheduleConsent = false;
				if (destination && destination.revision > 0) void loadScheduleFor(destination.id);
				else { scheduleInfo = null; schedulePolicy = null; }
			});
		} else {
			untrack(() => scheduleGen++);
		}
	});

	function patchSchedule(update: Partial<SchedulePolicy>) {
		if (!schedulePolicy || busy) return;
		schedulePolicy = { ...schedulePolicy, ...update };
		scheduleConsent = false;
		schedulePreview = null;
		scheduleError = '';
		scheduleGen++;
		guard?.markDirty(true);
	}

	async function handlePreviewSchedule() {
		if (!schedulePolicy || busy) return;
		if (schedulePolicy.quiet && schedulePolicy.quiet.start === schedulePolicy.quiet.end) {
			scheduleError = t.validationStartEndEqual;
			return;
		}
		const gen = ++scheduleGen;
		const snapshot = $state.snapshot(schedulePolicy);
		busy = true;
		guard?.setBusy(true);
		scheduleError = '';
		try {
			const res = await scheduleTransport.preview(snapshot);
			if (gen === scheduleGen && open) schedulePreview = res;
		} catch {
			if (gen === scheduleGen && open) scheduleError = schedT.error;
		} finally {
			busy = false;
			guard?.setBusy(false);
		}
	}

	async function handleSaveSchedule(event: SubmitEvent) {
		event.preventDefault();
		if (!schedulePolicy || busy) return;
		if (schedulePolicy.quiet && schedulePolicy.quiet.start === schedulePolicy.quiet.end) {
			scheduleError = t.validationStartEndEqual;
			return;
		}
		if (schedulePolicy.urgent_bypass && !scheduleConsent) {
			scheduleError = schedT.consent;
			return;
		}
		const snapshot = $state.snapshot(schedulePolicy);
		busy = true;
		guard?.setBusy(true);
		scheduleError = '';
		try {
			const saved = await scheduleTransport.save(snapshot, scheduleConsent);
			guard?.markDirty(false);
			await onScheduleSaved?.(saved);
			onClose();
			toast.success(t.scheduleSavedToast);
		} catch {
			scheduleError = schedT.error;
		} finally {
			busy = false;
			guard?.setBusy(false);
		}
	}
</script>

{#if !destination || destination.revision === 0}
	<div class="space-y-3 rounded-md border border-separator bg-field p-4 text-xs text-secondary">
		<p>{t.saveScheduleRequired}</p>
		<button
			type="button"
			onclick={onSwitchToDestination}
			class="min-h-[34px] rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
		>
			{t.tabDestination}
		</button>
	</div>
{:else if schedulePolicy && scheduleInfo}
	<form onsubmit={handleSaveSchedule} class="text-xs">
		<fieldset disabled={busy} class="flex flex-col gap-3 border-0 p-0">
			<WdgAlertScheduleSummary {scheduleInfo} />

			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
				<label class="flex flex-col gap-1">
					<span class="font-medium text-secondary">{schedT.cadence}</span>
					<select
						value={typeof schedulePolicy.cadence === 'string' ? 'Immediate' : 'Hourly' in schedulePolicy.cadence ? 'Hourly' : 'Daily'}
						onchange={(e) =>
							patchSchedule({
								cadence: e.currentTarget.value === 'Immediate' ? 'Immediate' : e.currentTarget.value === 'Hourly' ? { Hourly: { minute: 0 } } : { Daily: { minute: 540 } }
							})}
						class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
					>
						<option value="Immediate">{schedT.immediate}</option>
						<option value="Hourly">{schedT.hourly}</option>
						<option value="Daily">{schedT.daily}</option>
					</select>
				</label>

				<label class="flex flex-col gap-1">
					<span class="font-medium text-secondary">{schedT.zone}</span>
					<select
						value={schedulePolicy.zone}
						onchange={(e) => patchSchedule({ zone: e.currentTarget.value === 'Utc' ? 'Utc' : 'SystemLocal' })}
						class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
					>
						<option value="SystemLocal">{schedT.local}</option>
						<option value="Utc">{schedT.utc}</option>
					</select>
				</label>
			</div>

			{#if typeof schedulePolicy.cadence !== 'string' && 'Hourly' in schedulePolicy.cadence}
				<label class="flex flex-col gap-1">
					<span class="font-medium text-secondary">{schedT.minute}</span>
					<input
						type="number" min="0" max="59" required value={schedulePolicy.cadence.Hourly.minute}
						oninput={(e) => patchSchedule({ cadence: { Hourly: { minute: Number(e.currentTarget.value) } } })}
						class="h-9 w-36 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums"
					/>
				</label>
			{:else if typeof schedulePolicy.cadence !== 'string' && 'Daily' in schedulePolicy.cadence}
				<label class="flex flex-col gap-1">
					<span class="font-medium text-secondary">{schedT.time}</span>
					<input
						type="time" required value={time(schedulePolicy.cadence.Daily.minute)}
						oninput={(e) => patchSchedule({ cadence: { Daily: { minute: minutes(e.currentTarget.value) } } })}
						class="h-9 w-36 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums"
					/>
				</label>
			{/if}

			<WdgAlertQuietHoursFields
				{schedulePolicy}
				{scheduleConsent}
				onPatch={patchSchedule}
				onConsentChange={(checked) => (scheduleConsent = checked)}
			/>

			{#if schedulePreview}
				<p class="rounded border border-status-info/40 bg-status-info/10 px-3 py-2 font-mono text-[11px] tabular-nums text-foreground">
					{schedT.next}: {new Date(schedulePreview.next_epoch_ms).toLocaleString(i18n.locale)} ·
					{schedulePreview.timezone}
				</p>
			{/if}

			{#if scheduleError}
				<p role="alert" class="rounded border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-[11px] text-status-danger">
					{scheduleError}
				</p>
			{/if}

			<div class="flex flex-wrap justify-between gap-2 border-t border-separator pt-3">
				<button type="button" onclick={() => void handlePreviewSchedule()} class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft">
					{schedT.preview}
				</button>
				<div class="flex gap-2">
					<button type="button" onclick={onClose} class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground">
						{schedT.cancel}
					</button>
					<button type="submit" disabled={schedulePolicy.urgent_bypass && !scheduleConsent} class="min-h-[36px] rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-45">
						{schedT.save}
					</button>
				</div>
			</div>
		</fieldset>
	</form>
{:else if scheduleError}
	<p role="alert" class="text-xs text-status-danger">{scheduleError}</p>
{/if}
