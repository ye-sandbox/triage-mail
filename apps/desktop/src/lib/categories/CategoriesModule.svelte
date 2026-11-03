<script lang="ts">
	import { untrack } from 'svelte';
	import type { AccountConfig } from '../onboarding/types';
	import type { SettingsFormGuard } from '../settings/guards';
	import { i18n } from '../i18n/index.svelte';
	import { nativeCategories } from './transport';
	import type { SenderOverride, CategoryTransport } from './types';
	import { labels } from './labels';
	let {
		accounts = [],
		guard,
		transport = nativeCategories
	}: {
		accounts?: AccountConfig[];
		guard: SettingsFormGuard;
		transport?: CategoryTransport;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	let values = $state.raw<SenderOverride[]>([]);
	let more = $state(false);
	let busy = $state(false);
	let error = $state(false);
	let alive = true;
	let generation = 0;
	async function load(append = false) {
		const current = ++generation;
		try {
			const result = await transport.list(append ? values.length : 0);
			if (alive && current === generation) {
				values = append ? [...values, ...result] : result;
				more = result.length === 200;
				error = false;
			}
		} catch {
			if (alive && current === generation) error = true;
		}
	}
	$effect(() => {
		untrack(() => void load());
		return () => {
			alive = false;
			generation++;
			values = [];
			guard.reset();
		};
	});
	async function reset(value: SenderOverride) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		try {
			await transport.save({ ...value, category: null });
			if (alive) await load();
		} catch {
			if (alive) error = true;
		} finally {
			if (alive) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}
</script>

<section class="space-y-4" aria-label={t.title}>
	<h2 class="text-lg font-semibold">{t.title}</h2>
	<p class="text-sm">{t.help} {t.local}</p>
	<p class="text-sm">{t.consent}</p>
	<button disabled={busy} onclick={() => void load()}>{t.reload}</button>
	{#if error}<p role="alert">{t.error}</p>{/if}
	<h3 class="font-semibold">{t.overrides}</h3>
	{#each values.filter((v) => v.category) as value (`${value.account_id}:${value.sender}`)}
		<div class="space-y-2 rounded border border-[var(--separator)] p-4 text-sm">
			<p>
				{accounts.find((a) => a.account_id === value.account_id)?.email_display ?? t.unavailable}
			</p>
			<p class="break-all">
				{value.sender} · {value.category ? t.category[value.category] : t.all}
			</p>
			<button disabled={busy} onclick={() => void reset(value)}>{t.resetSender}</button>
		</div>
	{:else}<p class="text-sm">{t.empty}</p>{/each}
	{#if more}<button disabled={busy} onclick={() => void load(true)}>{t.more}</button>{/if}
</section>
