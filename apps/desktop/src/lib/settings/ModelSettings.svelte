<script lang="ts">
	import type { SettingsTransport } from './types';
	let {
		cached,
		transport,
		changed
	}: { cached: boolean; transport: SettingsTransport; changed: () => void } = $props();
	let busy = $state(false);
	let status = $state('');
	async function download(event: SubmitEvent) {
		event.preventDefault();
		if (busy || cached) return;
		const data = new FormData(event.currentTarget as HTMLFormElement);
		if (data.get('consent') !== 'on') return;
		busy = true;
		status = '';
		try {
			await transport.download({
				model_url: String(data.get('modelUrl')),
				tokenizer_url: String(data.get('tokenizerUrl')),
				consent: true
			});
			status = 'Model files cached.';
			changed();
		} catch {
			status = 'Download failed. Check direct HTTPS URLs and try again explicitly.';
		} finally {
			busy = false;
		}
	}
</script>

<section class="space-y-4" aria-label="Local model files">
	<h2 class="text-lg font-semibold">Local model files</h2>
	<p class="text-sm text-[var(--secondary)]">
		all-MiniLM-L6-v2 · {cached ? 'Files cached' : 'Files not cached'}. Downloads stay in
		$XDG_DATA_HOME/triage-mail/models (or ~/.local/share/triage-mail/models).
	</p>
	<p class="text-sm text-[var(--secondary)]">
		The current embedding engine uses deterministic projections, not ONNX inference. Caching these
		files does not enable learned semantic embeddings.
	</p>
	{#if !cached}
		<form onsubmit={download} class="space-y-4">
			<label class="block"
				>Direct model.onnx HTTPS URL<input
					name="modelUrl"
					type="url"
					required
					disabled={busy}
					class="mt-2 block w-full rounded border p-2"
				/></label
			>
			<label class="block"
				>Direct tokenizer.json HTTPS URL<input
					name="tokenizerUrl"
					type="url"
					required
					disabled={busy}
					class="mt-2 block w-full rounded border p-2"
				/></label
			>
			<p class="text-xs text-[var(--secondary)]">
				Use trusted all-MiniLM-L6-v2 artifacts. Limits: 128 MiB model, 8 MiB tokenizer. Redirects
				and URL credentials are rejected. Your IP is visible to these hosts; no email content is
				sent. Downloads have a two-minute timeout per file.
			</p>
			<label class="flex items-start gap-2"
				><input name="consent" type="checkbox" required disabled={busy} /> I consent to download both
				files from these URLs.</label
			>
			<button
				type="submit"
				disabled={busy}
				class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-[var(--accent)]"
				>{busy ? 'Downloading…' : 'Download model files'}</button
			>
		</form>
	{/if}
	{#if status}<p role="status" class="text-sm">{status}</p>{/if}
</section>
