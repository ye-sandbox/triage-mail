<script lang="ts">
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import { t } from '$lib/i18n';
	import type { Settings } from '$lib/settings/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace: InboxWorkspace;
		messageId: string;
		identity: string;
	};

	let { workspace, messageId, identity }: Props = $props();

	let aiSettings = $state<Settings | null>(null);
	let aiChecking = $state(false);
	let aiBusy = $state(false);
	let aiError = $state('');
	let aiSummaries = $state<Record<string, string>>({});
	let lastIdentity = '';

	$effect(() => {
		if (identity !== lastIdentity) {
			lastIdentity = identity;
			aiSettings = null;
			aiError = '';
		}
	});

	const aiSummary = $derived(aiSummaries[messageId] ?? null);

	async function prepareSummary() {
		if (aiChecking || aiBusy) return;
		const targetIdentity = identity;
		const targetId = messageId;
		aiChecking = true;
		aiError = '';
		try {
			const settings = await workspace.settingsTransport.get();
			if (identity !== targetIdentity) return;
			if (!settings.ai.enabled) {
				aiError = 'Enable summaries in Settings first.';
				return;
			}
			if (workspace.isDemo) {
				await runSummary(targetId, settings);
			} else {
				aiSettings = settings;
			}
		} catch {
			if (identity === targetIdentity) aiError = 'AI settings could not be loaded.';
		} finally {
			if (identity === targetIdentity) aiChecking = false;
		}
	}

	async function runSummary(targetId: string, settings: Settings) {
		if (aiBusy) return;
		const targetIdentity = identity;
		aiBusy = true;
		aiError = '';
		try {
			const summarize =
				workspace.readerTransport.summarize ?? workspace.settingsTransport.summarize;
			const result = await summarize(targetId, true, { ...settings.ai });
			if (identity === targetIdentity) {
				aiSummaries = { ...aiSummaries, [targetId]: result };
			}
		} catch {
			if (identity === targetIdentity) {
				aiError = 'Summary failed. Check Settings before trying again.';
			}
		} finally {
			if (identity === targetIdentity) {
				aiBusy = false;
				aiSettings = null;
			}
		}
	}
</script>

<div class="mx-4 mb-3 rounded-lg border border-separator bg-surface p-3">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<span class="text-xs font-medium text-foreground">On-Device AI Summary</span>
		<button
			type="button"
			disabled={aiChecking || aiBusy}
			onclick={() => void prepareSummary()}
			class="flex min-h-11 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] disabled:opacity-50 md:min-h-8"
		>
			<Sparkles class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
			<span>{aiChecking ? 'Checking settings…' : t(appStore.locale, 'summarizeLocalAi')}</span>
		</button>
	</div>
	{#if aiSettings}
		<div class="mt-2 space-y-2 rounded-md border border-separator bg-field p-3 text-xs">
			<p class="break-words text-secondary">
				Send this message’s subject and plain-text body (up to 8,000 characters) to
				{aiSettings.ai.endpoint} using {aiSettings.ai.model}?
				{aiSettings.ai.provider === 'Byok'
					? 'This leaves your device and may incur a charge.'
					: 'This goes to your local Ollama server.'}
			</p>
			<div class="flex gap-2">
				<button
					type="button"
					disabled={aiBusy}
					onclick={() => aiSettings && void runSummary(messageId, aiSettings)}
					class="rounded-md bg-accent px-3 py-1 text-xs font-medium text-accent-foreground disabled:opacity-50"
				>
					{aiBusy ? 'Summarizing…' : 'Send and summarize'}
				</button>
				<button
					type="button"
					disabled={aiBusy}
					onclick={() => (aiSettings = null)}
					class="rounded-md border border-separator bg-surface px-3 py-1 text-xs text-secondary hover:text-foreground"
				>
					Cancel
				</button>
			</div>
		</div>
	{/if}
	{#if aiError}
		<p role="alert" class="mt-2 text-xs text-status-danger">{aiError}</p>
	{/if}
	{#if aiSummary}
		<p class="mt-2 whitespace-pre-wrap break-words text-xs leading-relaxed text-secondary">
			{aiSummary}
		</p>
		<p class="mt-1 text-[11px] text-secondary">
			AI output can be inaccurate. Check the original message.
		</p>
	{/if}
</div>
