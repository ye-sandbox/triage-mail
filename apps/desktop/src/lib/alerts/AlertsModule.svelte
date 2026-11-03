<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { SettingsFormGuard } from '../settings/guards';
	import { labels } from './labels';
	import {
		newDestination,
		type Destination,
		type History,
		type AlertsTransport,
		type AlertSecret
	} from './types';
	import { nativeAlertsTransport } from './transport';
	import DestinationEditor from './DestinationEditor.svelte';
	import ScheduleEditor from './ScheduleEditor.svelte';
	import { scheduleLabels } from './schedule';
	let scheduleId = $state<string | null>(null);
	let {
		guard,
		selectedIds = [],
		transport = nativeAlertsTransport
	}: {
		guard: SettingsFormGuard;
		selectedIds?: string[];
		transport?: AlertsTransport;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	let destinations = $state<Destination[]>([]),
		history = $state<History[]>([]),
		draft = $state<Destination | null>(null);
	let busy = $state(false),
		message = $state('');
	let pending = $state<{
		kind: 'test' | 'replay';
		destination: Destination;
		ids: string[];
	} | null>(null);
	let active = false;
	function edit(d: Destination | null) {
		draft = d;
		pending = null;
		guard.markDirty(d !== null);
	}
	async function load() {
		const [d, h] = await Promise.all([transport.list(), transport.history()]);
		if (active) {
			destinations = d;
			history = h;
		}
	}
	async function operation(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		message = '';
		try {
			await work();
		} catch {
			if (active) message = t.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}
	$effect(() => {
		active = true;
		untrack(() => void operation(load));
		return () => {
			active = false;
			guard.reset();
		};
	});
	async function save(secret: AlertSecret | null, consent: boolean) {
		if (!draft) return;
		const value = $state.snapshot(draft);
		await operation(async () => {
			if (value.kind === 'Local' && value.enabled && !(await transport.available())) {
				if (active) message = t.unavailable;
				return;
			}
			await transport.save(value, secret, consent);
			if (active) {
				edit(null);
				await load();
			}
		});
	}
	async function deliver() {
		const request = pending;
		if (!request) return;
		pending = null;
		await operation(async () => {
			if (request.kind === 'test')
				await transport.test(request.destination.id, request.destination.revision, true);
			else
				await transport.replay(
					request.ids,
					request.destination.id,
					request.destination.revision,
					true
				);
			await load();
			if (active) message = t.queued;
		});
	}
</script>

<section class="space-y-4" aria-label={t.title}>
	<h2 class="text-lg font-semibold">{t.title}</h2>
	<p>{t.notice}</p>
	<p role="status">{message}</p>
	{#if draft}
		{#key draft.id}<DestinationEditor
				{draft}
				{busy}
				change={edit}
				{save}
				cancel={() => edit(null)}
			/>{/key}
	{:else if scheduleId}
		{#key scheduleId}<ScheduleEditor
				id={scheduleId}
				{guard}
				cancel={() => {
					scheduleId = null;
					guard.markDirty(false);
				}}
			/>{/key}
	{:else}
		<button type="button" disabled={busy} onclick={() => edit(newDestination())}>{t.add}</button>
		{#if !destinations.length}<p>{t.empty}</p>{/if}
		<ul class="space-y-4">
			{#each destinations as d (d.id)}
				<li class="border border-[var(--separator)] rounded p-4 space-y-2">
					<p>
						{d.name} · {d.kind === 'Local' ? t.local : d.public_origin} · {d.enabled
							? t.enabled
							: t.status.Canceled}
					</p>
					<p>
						{t.preview}: {d.fields.map((f) => t.fieldNames[f]).join(', ') || '—'}
					</p>
					<div class="flex gap-4 flex-wrap">
						<button
							type="button"
							disabled={busy}
							onclick={() => {
								pending = null;
								scheduleId = d.id;
							}}>{scheduleLabels[i18n.locale].title}</button
						>
						<button
							type="button"
							disabled={busy}
							onclick={() => edit(structuredClone($state.snapshot(d)))}>{t.edit}</button
						>
						<button
							type="button"
							disabled={busy}
							onclick={() =>
								void operation(async () => {
									await transport.remove(d.id, d.revision);
									await load();
								})}>{t.remove}</button
						>
						<button
							type="button"
							disabled={busy || !d.enabled}
							onclick={() => (pending = { kind: 'test', destination: d, ids: [] })}>{t.test}</button
						>
						<button
							type="button"
							disabled={busy || !d.enabled || !selectedIds.length}
							onclick={() =>
								(pending = {
									kind: 'replay',
									destination: d,
									ids: [...selectedIds]
								})}>{t.replay} ({selectedIds.length})</button
						>
					</div>
				</li>{/each}
		</ul>
	{/if}
	{#if pending}<div class="border rounded p-4 space-y-4" role="group" aria-label={t.send}>
			<p>
				{pending.kind === 'test' ? t.confirmTest : t.confirmReplay}
				{pending.destination.name} · {pending.destination.public_origin ?? t.local}
			</p>
			<p>{scheduleLabels[i18n.locale].bypass}</p>
			<p>{pending.destination.fields.map((f) => t.fieldNames[f]).join(', ')}</p>
			<button type="button" disabled={busy} onclick={() => void deliver()}>{t.send}</button>
			<button type="button" disabled={busy} onclick={() => (pending = null)}>{t.cancel}</button>
		</div>{/if}
	<h3>{t.history}</h3>
	<button type="button" disabled={busy} onclick={() => void operation(load)}>{t.history}</button>
	<ul class="space-y-2 tabular-nums">
		{#each history as h (h.event_id)}<li>
				{destinations.find((d) => d.id === h.destination_id)?.name ?? '—'} · {t.status[h.status]} · {h.attempts}
				· {new Date(h.created).toLocaleString(i18n.locale)}
			</li>{/each}
	</ul>
</section>
