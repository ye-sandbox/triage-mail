<script lang="ts">
	import ScheduleTiming from './ScheduleTiming.svelte';
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { SettingsFormGuard } from '../settings/guards';
	import {
		nativeScheduleTransport,
		scheduleLabels,
		type SchedulePolicy,
		type ScheduleState,
		type SchedulePreview,
		type ScheduleTransport
	} from './schedule';
	let {
		id,
		guard,
		cancel,
		transport = nativeScheduleTransport
	}: {
		id: string;
		guard: SettingsFormGuard;
		cancel: () => void;
		transport?: ScheduleTransport;
	} = $props();
	const t = $derived(scheduleLabels[i18n.locale]);
	let info = $state<ScheduleState | null>(null),
		policy = $state<SchedulePolicy | null>(null);
	let preview = $state<SchedulePreview | null>(null),
		busy = $state(false),
		error = $state(''),
		consent = $state(false);
	let active = false;
	let request = 0;
	function change(update: Partial<SchedulePolicy>) {
		if (!policy || busy) return;
		policy = { ...policy, ...update };
		consent = false;
		preview = null;
		request++;
		guard.markDirty();
	}
	async function operation(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		error = '';
		try {
			await work();
		} catch {
			if (active) error = t.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}
	$effect(() => {
		const selected = id;
		active = true;
		untrack(
			() =>
				void operation(async () => {
					const loaded = await transport.read(selected);
					if (active && selected === id) {
						info = loaded;
						policy = loaded.policy;
					}
				})
		);
		return () => {
			active = false;
			request++;
			guard.reset();
		};
	});
	async function showPreview() {
		if (!policy) return;
		const value = $state.snapshot(policy),
			generation = ++request;
		await operation(async () => {
			const result = await transport.preview(value);
			if (active && generation === request) preview = result;
		});
	}
	async function save() {
		if (!policy || (policy.urgent_bypass && !consent)) return;
		const value = $state.snapshot(policy);
		await operation(async () => {
			await transport.save(value, consent);
			if (active) {
				guard.markDirty(false);
				cancel();
			}
		});
	}
</script>

<section class="space-y-4" aria-label={t.title}>
	<h3>{t.title}</h3>
	<p>{t.notice}</p>
	<p>{t.dst}</p>
	<p role="status">{error}</p>
	{#if policy && info}
		<p class="tabular-nums">
			{t.deferred}: {info.deferred} · {t.summarized}: {info.summarized} · {t.expired}:
			{info.expired} · {t.overflow}: {info.overflow}
		</p>
		<form
			class="space-y-4"
			onsubmit={(e) => {
				e.preventDefault();
				void save();
			}}
		>
			<fieldset disabled={busy} class="space-y-4">
				<ScheduleTiming {policy} {change} />
				<label class="flex gap-2"
					><input
						type="checkbox"
						checked={policy.urgent_bypass}
						onchange={(e) => change({ urgent_bypass: e.currentTarget.checked })}
					/>{t.urgent}</label
				>
				{#if policy.urgent_bypass}<label class="flex gap-2"
						><input type="checkbox" required bind:checked={consent} />{t.consent}</label
					>{/if}
				<div class="flex gap-4 flex-wrap">
					<button type="button" onclick={() => void showPreview()}>{t.preview}</button><button
						type="submit"
						disabled={policy.urgent_bypass && !consent}>{t.save}</button
					><button type="button" onclick={cancel}>{t.cancel}</button>
				</div>
			</fieldset>
		</form>
		{#if preview}<p class="tabular-nums">
				{t.next}: {new Date(preview.next_epoch_ms).toLocaleString(i18n.locale)} ·
				{preview.timezone}
			</p>{/if}
	{:else}<button type="button" disabled={busy} onclick={cancel}>{t.cancel}</button>{/if}
</section>
