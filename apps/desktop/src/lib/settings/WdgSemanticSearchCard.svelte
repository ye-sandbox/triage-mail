<script lang="ts">
	import { onDestroy } from 'svelte';
	import { Switch } from 'bits-ui';
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import Cpu from '@lucide/svelte/icons/cpu';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import ShieldAlert from '@lucide/svelte/icons/shield-alert';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';
	import { getEmbeddingProgressPct, getEmbeddingStatusTone } from './settings.helpers';
	import type { EmbeddingState, Settings, SettingsTransport } from './types';

	interface Props {
		settings: Settings;
		transport: SettingsTransport;
		busy: boolean;
		semanticWorking: boolean;
		onWorkingChange: (working: boolean) => void;
		onReload?: () => void | Promise<void>;
	}

	let {
		settings,
		transport,
		busy,
		semanticWorking,
		onWorkingChange,
		onReload
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].search);

	let embeddingState = $state<EmbeddingState>({
		status: 'Unavailable',
		indexed: 0,
		pending: 0,
		failed: 0
	});
	let semanticError = $state(false);
	let vectorConsentChecked = $state(false);
	let disposed = false;

	onDestroy(() => {
		disposed = true;
	});

	$effect(() => {
		settings.experimental_semantic_search;
		settings.model_cached;
		let active = true;
		let loading = false;

		async function refreshEmbeddingState() {
			if (loading || !transport.embeddingState) return;
			loading = true;
			try {
				const next = await transport.embeddingState();
				if (active) embeddingState = next;
			} catch {
				if (active) {
					embeddingState = { status: 'Unavailable', indexed: 0, pending: 0, failed: 0 };
				}
			} finally {
				loading = false;
			}
		}

		void refreshEmbeddingState();
		const timer = setInterval(() => void refreshEmbeddingState(), 2000);
		return () => {
			active = false;
			clearInterval(timer);
		};
	});

	const progressPct = $derived(getEmbeddingProgressPct(embeddingState));
	const statusTone = $derived(getEmbeddingStatusTone(embeddingState.status));

	async function handleSemanticToggle(enabled: boolean) {
		if (busy || semanticWorking) return;
		onWorkingChange(true);
		semanticError = false;
		try {
			await transport.setExperimentalSemanticSearch(enabled);
			if (!disposed) {
				vectorConsentChecked = false;
				if (typeof window !== 'undefined') {
					window.dispatchEvent(new Event('triage-settings-updated'));
				}
				await onReload?.();
			}
		} catch {
			if (!disposed) {
				semanticError = true;
				toast.error(t.semanticError);
			}
		} finally {
			if (!disposed) onWorkingChange(false);
		}
	}

	async function handleVectorRebuild() {
		if (busy || semanticWorking || !vectorConsentChecked || !transport.rebuildEmbeddings) return;
		onWorkingChange(true);
		semanticError = false;
		try {
			await transport.rebuildEmbeddings(true);
			if (!disposed) {
				vectorConsentChecked = false;
				await onReload?.();
			}
		} catch {
			if (!disposed) {
				semanticError = true;
				toast.error(t.semanticError);
			}
		} finally {
			if (!disposed) onWorkingChange(false);
		}
	}
</script>

<section aria-label={t.semanticTitle} class="flex flex-col gap-4 rounded-lg border border-separator bg-surface p-4">
	<div class="flex flex-wrap items-center justify-between gap-4 border-b border-separator pb-3">
		<div class="space-y-1">
			<div class="flex items-center gap-2 text-sm font-medium text-foreground">
				<Cpu class="h-4 w-4 text-secondary" aria-hidden="true" />
				<span>{t.semanticTitle}</span>
			</div>
			<p class="max-w-xl text-xs text-secondary">{t.semanticDesc}</p>
		</div>
		<Switch.Root
			checked={settings.experimental_semantic_search}
			disabled={busy || semanticWorking}
			onCheckedChange={(checked) => void handleSemanticToggle(checked)}
			aria-label={t.semanticToggleAria}
			class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-field"
		>
			<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
		</Switch.Root>
	</div>

	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex flex-wrap items-center gap-3">
			<span class="text-xs font-medium text-foreground">{t.telemetryTitle}</span>
			<span class="inline-flex items-center gap-2 rounded border px-2 py-1 font-mono text-[11px] font-medium {statusTone}">
				{#if embeddingState.status === 'Ready'}
					<CheckCircle2 class="h-3 w-3" aria-hidden="true" />
				{:else if embeddingState.status === 'Rebuilding'}
					<RefreshCw class="h-3 w-3 animate-spin" aria-hidden="true" />
				{:else if embeddingState.status === 'Failed'}
					<ShieldAlert class="h-3 w-3" aria-hidden="true" />
				{:else}
					<AlertTriangle class="h-3 w-3" aria-hidden="true" />
				{/if}
				<span>{embeddingState.status}</span>
			</span>
		</div>
		<p role="status" class="text-xs text-secondary">{t.states[embeddingState.status]}</p>
	</div>

	<div class="grid grid-cols-3 gap-3 rounded-md border border-separator bg-field p-3">
		<div>
			<div class="text-[11px] text-secondary">{t.indexed}</div>
			<div class="mt-1 font-mono text-base font-semibold tabular-nums text-foreground">
				{embeddingState.indexed.toLocaleString()}
			</div>
		</div>
		<div>
			<div class="text-[11px] text-secondary">{t.pending}</div>
			<div class="mt-1 font-mono text-base font-semibold tabular-nums text-status-warning">
				{embeddingState.pending.toLocaleString()}
			</div>
		</div>
		<div>
			<div class="text-[11px] text-secondary">{t.failed}</div>
			<div class="mt-1 font-mono text-base font-semibold tabular-nums text-status-danger">
				{embeddingState.failed.toLocaleString()}
			</div>
		</div>
	</div>

	<div class="space-y-2">
		<div class="flex items-center justify-between font-mono text-[11px] tabular-nums text-secondary">
			<span>{t.vectorProgress}</span>
			<span>{progressPct}%</span>
		</div>
		<div class="h-2 w-full overflow-hidden rounded-full bg-field">
			<div class="h-full bg-accent transition-all duration-200" style="width: {progressPct}%"></div>
		</div>
	</div>

	{#if settings.experimental_semantic_search}
		<div class="flex flex-wrap items-center justify-between gap-3 border-t border-separator pt-3">
			<label class="flex cursor-pointer items-center gap-2 text-xs text-secondary">
				<input
					type="checkbox"
					bind:checked={vectorConsentChecked}
					disabled={busy || semanticWorking}
					class="h-3.5 w-3.5 accent-accent"
				/>
				<span>{t.rebuildConfirm}</span>
			</label>
			<button
				type="button"
				disabled={!vectorConsentChecked || busy || semanticWorking || !transport.rebuildEmbeddings}
				onclick={() => void handleVectorRebuild()}
				class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
			>
				{semanticWorking ? t.rebuildingVector : t.rebuildBtn}
			</button>
		</div>
	{/if}

	{#if semanticError}
		<p role="alert" class="text-xs text-status-danger">{t.semanticError}</p>
	{/if}
</section>
