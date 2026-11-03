<script lang="ts">
	import { untrack } from 'svelte';
	import type { InboxController } from '../inbox/controller.svelte';
	import type { MessageRowDto } from '../inbox/types';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import type { SenderChoice, ReviewState } from './types';
	import SenderDecision from './SenderDecision.svelte';
	let {
		inbox,
		message,
		accountLabel,
		disabled = false
	}: {
		inbox: InboxController;
		message: MessageRowDto;
		accountLabel: string;
		disabled?: boolean;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	let value = $state<SenderChoice | null>(null);
	let error = $state(false);
	let loading = $state(true);
	let alive = true;
	let generation = 0;
	async function load() {
		const current = ++generation;
		loading = true;
		try {
			const result = await inbox.reviewTransport?.metadata(message.id);
			if (alive && current === generation) {
				value = result ?? null;
				error = false;
			}
		} catch {
			if (alive && current === generation) error = true;
		} finally {
			if (alive && current === generation) loading = false;
		}
	}
	$effect(() => {
		inbox.badgeRevision;
		untrack(() => void load());
	});
	$effect(() => () => {
		alive = false;
		generation++;
		value = null;
	});
	async function save(state: ReviewState) {
		const current = value;
		const transport = inbox.reviewTransport;
		if (!current || !transport) return;
		await inbox.updateCategory(() => transport.save({ ...current, state }));
		if (alive) await load();
	}
</script>

{#if inbox.reviewTransport}
	<details class="border-b border-[var(--separator)] px-6 py-2 text-sm">
		<summary
			>{t.title}{#if value}: {t[value.state]}{/if}</summary
		>
		<p class="my-2 text-xs">{t.help}</p>
		{#if error}<p role="alert">{t.error}</p>
			<button onclick={() => void load()}>{t.reload}</button>
		{:else if loading}<p role="status">{t.initializing}</p>
		{:else if value}<SenderDecision
				{value}
				{accountLabel}
				disabled={disabled || inbox.mutating || inbox.loading}
				onSave={save}
			/>
		{:else}<p>{t.unresolved}</p>{/if}
	</details>
{/if}
