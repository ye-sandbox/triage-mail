<script lang="ts">
	import type { SettingsTransport, Settings } from './types';
	import { nativeSettingsTransport } from './transport';
	let {
		messageId,
		transport = nativeSettingsTransport
	}: { messageId: string; transport?: SettingsTransport } = $props();
	let settings = $state<Settings | null>(null);
	let checking = $state(false);
	let busy = $state(false);
	let summary = $state('');
	let error = $state('');
	let active = true;
	$effect(() => () => {
		active = false;
	});
	async function prepare() {
		if (checking || busy) return;
		checking = true;
		error = '';
		try {
			const result = await transport.get();
			if (active) {
				if (result.ai.enabled) settings = result;
				else error = 'Enable summaries in Settings first.';
			}
		} catch {
			if (active) error = 'AI settings could not be loaded.';
		} finally {
			if (active) checking = false;
		}
	}
	async function summarize() {
		if (!settings || busy) return;
		busy = true;
		error = '';
		try {
			const result = await transport.summarize(messageId, true, {
				...settings.ai
			});
			if (active) summary = result;
		} catch {
			if (active) error = 'Summary failed. Check Settings before trying again.';
		} finally {
			if (active) {
				busy = false;
				settings = null;
			}
		}
	}
</script>

<section class="space-y-2 px-8 py-4" aria-label="Optional AI summary">
	<button
		type="button"
		onclick={prepare}
		disabled={checking || busy}
		class="text-sm text-[var(--accent)]"
		>{checking ? 'Checking settings…' : 'Summarize with AI…'}</button
	>
	{#if settings}
		<p class="text-sm break-words">
			Send this message’s subject and plain-text body (up to 8,000 characters) to {settings.ai
				.endpoint} using {settings.ai.model}? {settings.ai.provider === 'Byok'
				? 'This leaves your device and may incur a charge.'
				: 'This goes to your local Ollama server.'}
		</p>
		<div class="flex gap-4">
			<button type="button" onclick={summarize} disabled={busy} class="text-sm text-[var(--accent)]"
				>{busy ? 'Summarizing…' : 'Send and summarize'}</button
			><button type="button" onclick={() => (settings = null)} disabled={busy} class="text-sm"
				>Cancel</button
			>
		</div>
	{/if}
	{#if error}<p role="alert" class="text-sm">{error}</p>{/if}
	{#if summary}<p class="text-sm whitespace-pre-wrap break-words">{summary}</p>
		<p class="text-xs text-[var(--secondary)]">
			AI output can be inaccurate. Check the original message.
		</p>{/if}
</section>
