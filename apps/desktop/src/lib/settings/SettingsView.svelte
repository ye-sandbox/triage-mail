<script lang="ts">
	import { untrack } from 'svelte';
	import type { Settings, SettingsTransport, Tokenizer } from './types';
	import { nativeSettingsTransport } from './transport';
	import AiSettingsForm from './AiSettingsForm.svelte';
	import ModelSettings from './ModelSettings.svelte';
	let {
		close,
		transport = nativeSettingsTransport
	}: { close: () => void; transport?: SettingsTransport } = $props();
	let settings = $state<Settings | null>(null);
	let error = $state('');
	let busy = $state(false);
	let active = true;
	let revision = 0;
	async function load() {
		const current = ++revision;
		try {
			const result = await transport.get();
			if (active && current === revision) {
				settings = result;
				error = '';
			}
		} catch {
			if (active && current === revision) error = 'Settings could not be loaded.';
		}
	}
	$effect(() => {
		untrack(() => void load());
		return () => {
			active = false;
			revision++;
		};
	});
	async function setTokenizer(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const data = new FormData(event.currentTarget as HTMLFormElement);
		const mode: Tokenizer =
			data.get('tokenizer') === 'Trigram' ? 'Trigram' : 'Unicode61RemoveDiacritics';
		busy = true;
		try {
			await transport.tokenizer(mode);
			await load();
		} catch {
			if (active) error = 'Search index could not be updated.';
		} finally {
			if (active) busy = false;
		}
	}
</script>

<main class="h-dvh overflow-y-auto bg-[var(--reader)] px-4 py-8 text-[var(--text)]">
	<div class="mx-auto max-w-2xl space-y-8">
		<div class="flex items-center justify-between gap-4">
			<h1 class="text-2xl font-semibold">Settings &amp; privacy</h1>
			<button type="button" onclick={close} class="rounded-lg px-4 py-2 text-[var(--accent)]"
				>Back to inbox</button
			>
		</div>
		{#if error}<p role="alert">{error}</p>
			<button type="button" onclick={() => void load()}>Retry loading settings</button>{/if}
		{#if settings}
			<AiSettingsForm
				initial={settings.ai}
				hasKey={settings.has_api_key}
				{transport}
				changed={() => void load()}
			/>
			<hr class="border-[var(--separator)]" />
			<ModelSettings cached={settings.model_cached} {transport} changed={() => void load()} />
			<hr class="border-[var(--separator)]" />
			<form onsubmit={setTokenizer} class="space-y-4" aria-label="Search settings">
				<h2 class="text-lg font-semibold">Word search</h2>
				<label class="block"
					>Tokenizer<select
						name="tokenizer"
						value={settings.tokenizer}
						disabled={busy}
						class="mt-2 block w-full rounded border bg-[var(--surface)] p-2"
						><option value="Unicode61RemoveDiacritics">Unicode words (ignore accents)</option
						><option value="Trigram">Trigram (substring matching)</option></select
					></label
				>
				<p class="text-sm text-[var(--secondary)]">
					Applying rebuilds the local search index and may take time. Trigram needs at least three
					characters for substring matching and uses more disk space.
				</p>
				<button
					type="submit"
					disabled={busy}
					class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-[var(--accent)]"
					>{busy ? 'Rebuilding…' : 'Apply tokenizer'}</button
				>
			</form>
			<p class="text-sm text-[var(--secondary)]">
				Remote images remain blocked by default. You can allow HTTPS images for a single open
				message in the reader. This may reveal your IP and that you opened the message. Changing
				messages resets consent.
			</p>
		{:else if !error}<p role="status">Loading settings…</p>{/if}
	</div>
</main>
