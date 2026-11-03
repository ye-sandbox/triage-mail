<script lang="ts">
	import { untrack } from 'svelte';
	import type { AccountConfig } from '../onboarding/types';
	import type { SettingsFormGuard } from '../settings/guards';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import { nativeReview } from './transport';
	import type { ReviewTransport, ReviewPreferences, SenderChoice, ReviewState } from './types';
	import SenderDecision from './SenderDecision.svelte';
	let {
		accounts = [],
		guard,
		transport = nativeReview
	}: {
		accounts?: AccountConfig[];
		guard: SettingsFormGuard;
		transport?: ReviewTransport;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	let accountId = $state('');
	let value = $state<ReviewPreferences | null>(null);
	let choices = $state.raw<SenderChoice[]>([]);
	let more = $state(false);
	let enabled = $state(false);
	let hide = $state(false);
	let dirty = $state(false);
	let busy = $state(false);
	let loading = $state(false);
	let error = $state(false);
	let alive = true;
	let generation = 0;
	const accountLabel = $derived(
		accounts.find((a) => a.account_id === accountId)?.email_display ?? accountId
	);
	async function load(append = false) {
		const current = ++generation;
		const account = accountId;
		if (!account) return;
		loading = true;
		try {
			const [pref, rows] = await Promise.all([
				transport.preferences(account),
				transport.list(account, append ? choices.length : 0)
			]);
			if (alive && current === generation) {
				value = pref;
				choices = append ? [...choices, ...rows] : rows;
				more = rows.length === 200;
				enabled = pref.enabled;
				hide = pref.hide_in_inbox;
				dirty = false;
				guard.markDirty(false);
				error = false;
			}
		} catch {
			if (alive && current === generation) error = true;
		} finally {
			if (alive && current === generation) loading = false;
		}
	}
	function select(id: string) {
		if (dirty || busy) return;
		generation++;
		accountId = id;
		value = null;
		choices = [];
		void load();
	}
	$effect(() => {
		const ids = accounts.map((a) => a.account_id);
		untrack(() => {
			if (!accountId && ids[0]) select(ids[0]);
			else if (accountId && !ids.includes(accountId)) {
				generation++;
				accountId = '';
				value = null;
				choices = [];
				dirty = false;
				guard.markDirty(false);
			}
		});
	});
	$effect(() => {
		const timer = setInterval(() => {
			if (value?.initializing && !busy && !dirty && !loading) void load();
		}, 1500);
		return () => {
			clearInterval(timer);
			alive = false;
			generation++;
			choices = [];
			value = null;
			guard.reset();
		};
	});
	function changed() {
		generation++;
		dirty = true;
		guard.markDirty(true);
	}
	async function write(operation: () => Promise<unknown>) {
		if (busy || loading) return;
		busy = true;
		guard.setBusy(true);
		try {
			await operation();
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
	async function save() {
		if (!value) return;
		const draft = { ...value, enabled, hide_in_inbox: hide };
		await write(() => transport.savePreferences(draft));
	}
	async function decide(choice: SenderChoice, state: ReviewState) {
		await write(() => transport.save({ ...choice, state }));
	}
</script>

<section class="space-y-4" aria-label={t.title}>
	<h2 class="text-lg font-semibold">{t.title}</h2>
	<p class="text-sm">{t.help}</p>
	<p class="text-sm">{t.history}</p>
	<label class="block"
		>{t.account}<select
			class="ml-2 rounded border bg-[var(--surface)] p-2"
			value={accountId}
			disabled={busy || dirty || loading}
			onchange={(e) => select(e.currentTarget.value)}
			><option value="">{t.choose}</option>{#each accounts as account}<option
					value={account.account_id}>{account.email_display}</option
				>{/each}</select
		></label
	>
	{#if error}<p role="alert">{t.error}</p>{/if}
	<button disabled={busy || dirty || loading || !accountId} onclick={() => void load()}
		>{t.reload}</button
	>
	{#if value}
		<fieldset disabled={busy || loading} class="space-y-4">
			<label class="flex gap-2"
				><input type="checkbox" bind:checked={enabled} onchange={changed} />{t.enable}</label
			>
			<label class="flex gap-2"
				><input type="checkbox" bind:checked={hide} onchange={changed} />{t.hide}</label
			>
			<p class="text-sm">{t.hideHelp}</p>
			<button disabled={!dirty} onclick={() => void save()}>{busy ? t.saving : t.save}</button>
		</fieldset>
		{#if value.enabled}<p role="status">
				{value.initializing ? t.initializing : t.ready}
			</p>{/if}
		<h3 class="font-semibold">{t.choices}</h3>
		{#each choices as choice (choice.sender)}<SenderDecision
				value={choice}
				{accountLabel}
				disabled={busy || dirty || loading}
				onSave={(state) => decide(choice, state)}
			/>{:else}<p>{t.empty}</p>{/each}
		{#if more}<button disabled={busy || dirty || loading} onclick={() => void load(true)}
				>{t.more}</button
			>{/if}
	{/if}
</section>
