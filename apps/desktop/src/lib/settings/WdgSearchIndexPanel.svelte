<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { Switch } from 'bits-ui';
	import {
		AlertTriangle,
		CheckCircle2,
		Cpu,
		Database,
		Download,
		HardDrive,
		RefreshCw,
		ShieldAlert
	} from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';
	import { ONNX_MODEL_META } from './settings.fixtures';
	import { getEmbeddingProgressPct, getEmbeddingStatusTone } from './settings.helpers';
	import type { EmbeddingState, Settings, SettingsTransport, Tokenizer } from './types';

	interface Props {
		settings: Settings;
		transport: SettingsTransport;
		busy?: boolean;
		onReload?: () => void | Promise<void>;
	}

	let { settings, transport, busy = false, onReload }: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].search);

	let selectedTokenizer = $state<Tokenizer>(untrack(() => settings.tokenizer));
	let rebuildingFts = $state(false);
	let ftsFeedback = $state<'ok' | 'error' | ''>('');

	let embeddingState = $state<EmbeddingState>({
		status: 'Unavailable',
		indexed: 0,
		pending: 0,
		failed: 0
	});
	let semanticWorking = $state(false);
	let semanticError = $state(false);
	let vectorConsentChecked = $state(false);

	let downloadingOnnx = $state(false);
	let onnxFeedback = $state<'ok' | 'error' | ''>('');

	let disposed = false;
	onDestroy(() => {
		disposed = true;
	});

	$effect(() => {
		const nextTokenizer = settings.tokenizer;
		if (!rebuildingFts) {
			selectedTokenizer = nextTokenizer;
		}
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
	const isBusy = $derived(busy || rebuildingFts || semanticWorking || downloadingOnnx);

	async function handleApplyTokenizer(event: SubmitEvent) {
		event.preventDefault();
		if (isBusy) return;
		rebuildingFts = true;
		ftsFeedback = '';
		try {
			await transport.tokenizer(selectedTokenizer);
			if (!disposed) {
				ftsFeedback = 'ok';
				toast.success(t.tokenizerUpdated);
				await onReload?.();
			}
		} catch {
			if (!disposed) {
				ftsFeedback = 'error';
				toast.error(t.tokenizerError);
			}
		} finally {
			if (!disposed) rebuildingFts = false;
		}
	}

	async function handleSemanticToggle(enabled: boolean) {
		if (busy || semanticWorking) return;
		semanticWorking = true;
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
			if (!disposed) semanticWorking = false;
		}
	}

	async function handleVectorRebuild() {
		if (busy || semanticWorking || !vectorConsentChecked || !transport.rebuildEmbeddings) return;
		semanticWorking = true;
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
			if (!disposed) semanticWorking = false;
		}
	}

	async function handleOnnxDownload(event: SubmitEvent) {
		event.preventDefault();
		if (busy || downloadingOnnx || settings.model_cached) return;
		const form = event.currentTarget as HTMLFormElement;
		const consentInput = form.elements.namedItem('consent') as HTMLInputElement | null;
		const data = new FormData(form);
		if (!consentInput?.checked && data.get('consent') !== 'on') return;
		const modelUrl = String(data.get('modelUrl') ?? '').trim();
		const tokenizerUrl = String(data.get('tokenizerUrl') ?? '').trim();
		if (!modelUrl || !tokenizerUrl) return;

		downloadingOnnx = true;
		onnxFeedback = '';
		try {
			await transport.download({
				model_url: modelUrl,
				tokenizer_url: tokenizerUrl,
				consent: true
			});
			if (!disposed) {
				onnxFeedback = 'ok';
				form.reset();
				toast.success(t.downloadOk);
				await onReload?.();
			}
		} catch {
			if (!disposed) {
				onnxFeedback = 'error';
				toast.error(t.downloadError);
			}
		} finally {
			if (!disposed) downloadingOnnx = false;
		}
	}
</script>

<section class="flex flex-col gap-5" aria-label={t.heading} aria-labelledby="search-index-heading">
	<div class="border-b border-separator pb-3">
		<h2 id="search-index-heading" class="font-display text-base font-semibold text-foreground">
			{t.heading}
		</h2>
		<p class="mt-0.5 text-xs text-secondary">{t.subtitle}</p>
	</div>

	<form
		onsubmit={handleApplyTokenizer}
		aria-label={t.tokenizerTitle}
		class="flex flex-col gap-4 rounded-lg border border-separator bg-surface p-4"
	>
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div class="space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<Database class="h-4 w-4 text-secondary" />
					<span>{t.tokenizerTitle}</span>
				</div>
				<p class="text-xs text-secondary">{t.tokenizerDesc}</p>
			</div>
			<button
				type="submit"
				disabled={isBusy}
				class="flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md bg-accent px-3.5 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
			>
				<RefreshCw class="h-3.5 w-3.5 {rebuildingFts ? 'animate-spin' : ''}" />
				<span>{rebuildingFts ? t.rebuildingTokenizer : t.applyTokenizer}</span>
			</button>
		</div>

		<div
			role="radiogroup"
			aria-label={t.tokenizerSelectLabel}
			class="grid grid-cols-1 gap-3 sm:grid-cols-2"
		>
			<button
				type="button"
				role="radio"
				aria-checked={selectedTokenizer === 'Unicode61RemoveDiacritics'}
				disabled={isBusy}
				onclick={() => (selectedTokenizer = 'Unicode61RemoveDiacritics')}
				class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:opacity-50 {selectedTokenizer ===
				'Unicode61RemoveDiacritics'
					? 'border-accent bg-selection-soft'
					: 'border-separator bg-field hover:bg-selection-soft/40'}"
			>
				<span class="font-mono text-xs font-semibold text-foreground">Unicode61RemoveDiacritics</span>
				<span class="text-[11px] text-foreground">{t.unicodeLabel}</span>
				<span class="text-[11px] text-secondary">{t.unicodeDesc}</span>
			</button>

			<button
				type="button"
				role="radio"
				aria-checked={selectedTokenizer === 'Trigram'}
				disabled={isBusy}
				onclick={() => (selectedTokenizer = 'Trigram')}
				class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:opacity-50 {selectedTokenizer ===
				'Trigram'
					? 'border-accent bg-selection-soft'
					: 'border-separator bg-field hover:bg-selection-soft/40'}"
			>
				<span class="font-mono text-xs font-semibold text-foreground">Trigram</span>
				<span class="text-[11px] text-foreground">{t.trigramLabel}</span>
				<span class="text-[11px] text-secondary">{t.trigramDesc}</span>
			</button>
		</div>

		{#if selectedTokenizer === 'Trigram'}
			<div class="flex items-start gap-2 rounded-md border border-status-warning/40 bg-status-warning/10 px-3 py-2 text-xs text-status-warning">
				<AlertTriangle class="mt-0.5 h-3.5 w-3.5 shrink-0" />
				<span>{t.trigramWarning}</span>
			</div>
		{/if}

		{#if ftsFeedback === 'ok'}
			<p role="status" class="text-xs text-status-healthy">{t.tokenizerUpdated}</p>
		{:else if ftsFeedback === 'error'}
			<p role="alert" class="text-xs text-status-danger">{t.tokenizerError}</p>
		{/if}
	</form>

	<section
		aria-label={t.semanticTitle}
		class="flex flex-col gap-4 rounded-lg border border-separator bg-surface p-4"
	>
		<div class="flex flex-wrap items-center justify-between gap-4 border-b border-separator pb-3">
			<div class="space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<Cpu class="h-4 w-4 text-secondary" />
					<span>{t.semanticTitle}</span>
				</div>
				<p class="max-w-xl text-xs text-secondary">{t.semanticDesc}</p>
			</div>
			<Switch.Root
				checked={settings.experimental_semantic_search}
				disabled={busy || semanticWorking}
				onCheckedChange={(checked) => void handleSemanticToggle(checked)}
				aria-label={t.semanticToggleAria}
				class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-field"
			>
				<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
			</Switch.Root>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="flex flex-wrap items-center gap-2.5">
				<span class="text-xs font-medium text-foreground">{t.telemetryTitle}</span>
				<span class="inline-flex items-center gap-1.5 rounded border px-2 py-0.5 font-mono text-[11px] font-medium {statusTone}">
					{#if embeddingState.status === 'Ready'}
						<CheckCircle2 class="h-3 w-3" />
					{:else if embeddingState.status === 'Rebuilding'}
						<RefreshCw class="h-3 w-3 animate-spin" />
					{:else if embeddingState.status === 'Failed'}
						<ShieldAlert class="h-3 w-3" />
					{:else}
						<AlertTriangle class="h-3 w-3" />
					{/if}
					<span>{embeddingState.status}</span>
				</span>
			</div>
			<p role="status" class="text-xs text-secondary">{t.states[embeddingState.status]}</p>
		</div>

		<div class="grid grid-cols-3 gap-3 rounded-md border border-separator bg-field p-3">
			<div>
				<div class="text-[11px] text-secondary">{t.indexed}</div>
				<div class="mt-0.5 font-mono text-base font-semibold tabular-nums text-foreground">
					{embeddingState.indexed.toLocaleString()}
				</div>
			</div>
			<div>
				<div class="text-[11px] text-secondary">{t.pending}</div>
				<div class="mt-0.5 font-mono text-base font-semibold tabular-nums text-status-warning">
					{embeddingState.pending.toLocaleString()}
				</div>
			</div>
			<div>
				<div class="text-[11px] text-secondary">{t.failed}</div>
				<div class="mt-0.5 font-mono text-base font-semibold tabular-nums text-status-danger">
					{embeddingState.failed.toLocaleString()}
				</div>
			</div>
		</div>

		<div class="space-y-1.5">
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
					class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
				>
					{semanticWorking ? t.rebuildingVector : t.rebuildBtn}
				</button>
			</div>
		{/if}

		{#if semanticError}
			<p role="alert" class="text-xs text-status-danger">{t.semanticError}</p>
		{/if}
	</section>

	<section
		aria-label={t.modelTitle}
		class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4"
	>
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div class="space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<HardDrive class="h-4 w-4 text-secondary" />
					<span>{t.modelTitle}</span>
				</div>
				<p class="font-mono text-[11px] text-secondary">
					all-MiniLM-L6-v2 · {settings.model_cached ? t.modelCached : t.modelMissing} · {ONNX_MODEL_META.cacheDir}
				</p>
				<p class="text-xs text-secondary">{t.modelInfo}</p>
			</div>
			<span
				class="shrink-0 whitespace-nowrap font-mono text-xs font-medium {settings.model_cached
					? 'text-status-healthy'
					: 'text-status-warning'}"
			>
				model_cached: {settings.model_cached ? 'true' : 'false'}
			</span>
		</div>

		{#if !settings.model_cached}
			<form onsubmit={handleOnnxDownload} class="flex flex-col gap-3 border-t border-separator pt-3">
				<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
					<label class="flex flex-col gap-1.5">
						<span class="text-xs font-medium text-foreground">{t.modelUrlLabel}</span>
						<input
							name="modelUrl"
							type="url"
							required
							disabled={busy || downloadingOnnx}
							placeholder="https://models.example.org/model.onnx"
							class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground placeholder:font-sans placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-50"
						/>
					</label>
					<label class="flex flex-col gap-1.5">
						<span class="text-xs font-medium text-foreground">{t.tokenizerUrlLabel}</span>
						<input
							name="tokenizerUrl"
							type="url"
							required
							disabled={busy || downloadingOnnx}
							placeholder="https://models.example.org/tokenizer.json"
							class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground placeholder:font-sans placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-50"
						/>
					</label>
				</div>

				<p class="text-[11px] text-secondary">{t.modelPolicy}</p>

				<div class="flex flex-wrap items-center justify-between gap-3 border-t border-separator pt-3">
					<label class="flex cursor-pointer items-start gap-2 text-xs text-secondary">
						<input
							name="consent"
							type="checkbox"
							required
							disabled={busy || downloadingOnnx}
							class="mt-0.5 h-3.5 w-3.5 accent-accent"
						/>
						<span>{t.modelConsent}</span>
					</label>
					<button
						type="submit"
						disabled={busy || downloadingOnnx}
						class="flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
					>
						<Download class="h-3.5 w-3.5" />
						<span>{downloadingOnnx ? t.downloadingBtn : t.downloadBtn}</span>
					</button>
				</div>
			</form>
		{/if}

		{#if onnxFeedback === 'ok'}
			<p role="status" class="text-xs text-status-healthy">{t.downloadOk}</p>
		{:else if onnxFeedback === 'error'}
			<p role="alert" class="text-xs text-status-danger">{t.downloadError}</p>
		{/if}
	</section>
</section>
