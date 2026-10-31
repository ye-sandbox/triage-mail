<script lang="ts">
	import type { AiSettings, SettingsTransport } from './types';
	let {
		initial,
		hasKey,
		transport,
		changed
	}: {
		initial: AiSettings;
		hasKey: boolean;
		transport: SettingsTransport;
		changed: () => void;
	} = $props();
	let busy = $state(false);
	let status = $state('');
	let form: HTMLFormElement;
	$effect(() => () => form?.reset());
	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const data = new FormData(form);
		const keyInput = form.elements.namedItem('apiKey') as HTMLInputElement;
		// INVARIANT: Keys live only in the unbound input and invoke arguments; clear on submit.
		const key = keyInput.value;
		data.delete('apiKey');
		keyInput.value = '';
		busy = true;
		status = '';
		try {
			await transport.save(
				{
					enabled: data.get('enabled') === 'on',
					provider: data.get('provider') === 'Byok' ? 'Byok' : 'Ollama',
					endpoint: String(data.get('endpoint') ?? '').trim(),
					model: String(data.get('model') ?? '').trim()
				},
				key
			);
			status = 'AI settings saved. No request was sent.';
			changed();
		} catch {
			status = 'AI settings could not be saved. Check the endpoint, model and API key.';
		} finally {
			busy = false;
		}
	}
</script>

<form bind:this={form} onsubmit={save} class="space-y-4" aria-label="AI settings">
	<h2 class="text-lg font-semibold">Optional summaries</h2>
	<p class="text-sm text-[var(--secondary)]">
		Disabled by default. Each summary needs your confirmation before sending the subject and
		plain-text body (up to 8,000 characters) to your chosen endpoint. Cloud providers may charge
		you.
	</p>
	<label class="flex items-center gap-2"
		><input type="checkbox" name="enabled" checked={initial.enabled} disabled={busy} /> Enable summary
		requests</label
	>
	<label class="block"
		>Provider<select
			name="provider"
			value={initial.provider}
			disabled={busy}
			class="mt-2 block w-full rounded border bg-[var(--surface)] p-2"
			><option value="Ollama">Local Ollama</option><option value="Byok"
				>BYOK (OpenAI compatible)</option
			></select
		></label
	>
	<label class="block"
		>Endpoint<input
			name="endpoint"
			type="url"
			required
			value={initial.endpoint}
			disabled={busy}
			class="mt-2 block w-full rounded border p-2"
		/></label
	>
	<p class="text-xs text-[var(--secondary)]">
		Ollama requires loopback HTTP/HTTPS. BYOK requires a public HTTPS domain. No embedded
		credentials, query strings or fragments.
	</p>
	<label class="block"
		>Model<input
			name="model"
			required
			maxlength="200"
			value={initial.model}
			disabled={busy}
			class="mt-2 block w-full rounded border p-2"
		/></label
	>
	<label class="block"
		>API key<input
			name="apiKey"
			type="password"
			autocomplete="off"
			disabled={busy}
			class="mt-2 block w-full rounded border p-2"
		/></label
	>
	<p class="text-xs text-[var(--secondary)]">
		{hasKey ? 'A key is stored in your vault.' : 'No API key is stored.'} Leave blank to retain a key
		for the same endpoint. Switching endpoints requires a new key. Saving Ollama clears the key.
	</p>
	<button
		type="submit"
		disabled={busy}
		class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-[var(--accent)]"
		>{busy ? 'Saving…' : 'Save AI settings'}</button
	>
	{#if status}<p role="status" class="text-sm">{status}</p>{/if}
</form>
