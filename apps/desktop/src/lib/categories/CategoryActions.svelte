<script lang="ts">
	import { untrack } from 'svelte';
	import type { InboxController } from '../inbox/controller.svelte';
	import type { MessageRowDto } from '../inbox/types';
	import { i18n } from '../i18n/index.svelte';
	import { categories, type Category, type CategoryMetadata } from './types';
	import { labels } from './labels';
	let {
		inbox,
		message,
		disabled = false
	}: { inbox: InboxController; message: MessageRowDto; disabled?: boolean } = $props();
	const t = $derived(labels[i18n.locale]);
	let metadata = $state<CategoryMetadata | null>(null);
	let choice = $state<Category>('Personal');
	let consent = $state(false);
	let error = $state(false);
	let alive = true;
	let generation = 0;
	async function load() {
		const revision = ++generation;
		try {
			const value = await inbox.categoryTransport?.metadata(message.id);
			if (alive && revision === generation && value) {
				metadata = value;
				choice = value.effective;
				error = false;
				consent = false;
			}
		} catch {
			if (alive && revision === generation) error = true;
		}
	}
	$effect(() => {
		inbox.badgeRevision;
		untrack(() => void load());
	});
	$effect(() => () => {
		alive = false;
		generation++;
		metadata = null;
	});
	async function save(sender: boolean, reset: boolean) {
		const transport = inbox.categoryTransport;
		const current = metadata;
		if (!transport || !current || (sender && (!current.sender || (!reset && !consent)))) return;
		await inbox.updateCategory(() =>
			sender
				? transport.save({
						account_id: message.account_id,
						sender: current.sender!,
						revision: current.sender_revision,
						category: reset ? null : choice
					})
				: transport.correct(message.id, current.correction_revision, reset ? null : choice)
		);
		if (alive) await load();
	}
</script>

{#if inbox.categoryTransport}
	<details class="border-b border-[var(--separator)] px-6 py-2 text-sm">
		<summary
			>{t.title}{#if metadata}: {t.category[metadata.effective]} · {t.source[
					metadata.source
				]}{/if}</summary
		>
		<p class="my-2 text-xs text-[var(--secondary)]">{t.help} {t.local}</p>
		{#if error}<p role="alert">{t.error}</p>
			<button onclick={() => void load()}>{t.reload}</button>{/if}
		{#if metadata}
			<p class="text-xs">
				{t.source.Automatic}: {t.category[metadata.automatic]} · {t.reason[metadata.reason]} ·
				<span class="tabular-nums">v{metadata.version}</span>
			</p>
			{#if !metadata.durable}<p class="my-2 text-xs">{t.durability}</p>{/if}
			<fieldset disabled={disabled || inbox.mutating || inbox.loading} class="my-2 space-y-2">
				<label class="block"
					>{t.title}<select
						class="ml-2 rounded border bg-[var(--surface)] p-2"
						bind:value={choice}
						onchange={() => (consent = false)}
						>{#each categories as category}<option value={category}>{t.category[category]}</option
							>{/each}</select
					></label
				>
				<div class="flex flex-wrap gap-4">
					<button onclick={() => void save(false, false)}>{t.correct}</button><button
						disabled={metadata.correction_revision === 0}
						onclick={() => void save(false, true)}>{t.reset}</button
					>
				</div>
				{#if metadata.sender}
					<p>{t.sender}: {metadata.sender}</p>
					<label class="flex gap-2"
						><input type="checkbox" bind:checked={consent} />{t.consent}</label
					>
					<div class="flex flex-wrap gap-4">
						<button disabled={!consent} onclick={() => void save(true, false)}
							>{t.saveSender}</button
						><button disabled={metadata.sender_revision === 0} onclick={() => void save(true, true)}
							>{t.resetSender}</button
						>
					</div>
				{/if}
			</fieldset>
		{:else if !error}<p role="status">{t.loading}</p>{/if}
	</details>
{/if}
