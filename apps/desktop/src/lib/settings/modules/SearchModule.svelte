<script lang="ts">
	import type { SettingsModuleProps, Tokenizer } from '../types';

	let {
		settings,
		transport,
		busy = false,
		onReload
	}: SettingsModuleProps = $props();

	let internalBusy = $state(false);
	let error = $state('');
	let status = $state('');

	const isBusy = $derived(busy || internalBusy);

	async function setTokenizer(event: SubmitEvent) {
		event.preventDefault();
		if (isBusy) return;
		const form = (event.currentTarget ?? event.target) as HTMLFormElement | null;
		if (!form) return;
		const data = new FormData(form);
		const mode: Tokenizer =
			data.get('tokenizer') === 'Trigram' ? 'Trigram' : 'Unicode61RemoveDiacritics';
		internalBusy = true;
		error = '';
		status = '';
		try {
			await transport.tokenizer(mode);
			status = 'Search index updated.';
			await onReload?.();
		} catch {
			error = 'Search index could not be updated.';
		} finally {
			internalBusy = false;
		}
	}

	async function toggleSemanticSearch(event: Event) {
		if (isBusy) return;
		const target = (event.currentTarget ?? event.target) as HTMLInputElement | null;
		if (!target) return;
		const enabled = target.checked;
		internalBusy = true;
		error = '';
		status = '';
		try {
			await transport.setExperimentalSemanticSearch(enabled);
			status = enabled ? 'Semantic search enabled.' : 'Semantic search disabled.';
			await onReload?.();
		} catch {
			error = 'Could not update semantic search setting.';
		} finally {
			internalBusy = false;
		}
	}
</script>

<div class="space-y-6" aria-label="Search settings">
	<div>
		<h2 class="text-lg font-semibold text-[var(--text)]">Search</h2>
		<p class="text-sm text-[var(--secondary)]">Configure lexical indexing and semantic search capabilities.</p>
	</div>

	<section class="space-y-4" aria-label="Experimental semantic search">
		<h3 class="text-base font-medium text-[var(--text)]">Semantic search (Experimental)</h3>
		<p class="text-sm text-[var(--secondary)]">
			Uses deterministic token and character n-gram feature-hash projections (not learned neural
			ONNX models) to generate 384-dimensional vectors for sqlite-vec. When enabled, Semantic and
			Hybrid search options are available in the inbox. Learned ONNX neural embeddings are
			planned for v0.2.
		</p>
		<label class="flex items-center gap-3 text-sm">
			<input
				type="checkbox"
				checked={settings.experimental_semantic_search}
				disabled={isBusy}
				onchange={toggleSemanticSearch}
				class="h-4 w-4 rounded border-[var(--separator)]"
			/>
			Enable experimental semantic search
		</label>
	</section>

	<hr class="border-[var(--separator)]" />

	<form onsubmit={setTokenizer} class="space-y-4" aria-label="Word search settings">
		<h3 class="text-base font-medium text-[var(--text)]">Word search</h3>
		<label class="block text-sm"
			>Tokenizer
			<select
				name="tokenizer"
				value={settings.tokenizer}
				disabled={isBusy}
				class="mt-2 block w-full rounded border border-[var(--separator)] bg-[var(--surface)] p-2 text-[var(--text)]"
			>
				<option value="Unicode61RemoveDiacritics">Unicode words (ignore accents)</option>
				<option value="Trigram">Trigram (substring matching)</option>
			</select>
		</label>
		<p class="text-sm text-[var(--secondary)]">
			Applying rebuilds the local search index and may take time. Trigram needs at least three
			characters for substring matching and uses more disk space.
		</p>
		<button
			type="submit"
			disabled={isBusy}
			class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-[var(--accent)] disabled:opacity-50"
		>
			{isBusy ? 'Rebuilding…' : 'Apply tokenizer'}
		</button>
	</form>

	{#if error}<p role="alert" class="text-sm text-red-500">{error}</p>{/if}
	{#if status}<p role="status" class="text-sm text-[var(--accent)]">{status}</p>{/if}
</div>
