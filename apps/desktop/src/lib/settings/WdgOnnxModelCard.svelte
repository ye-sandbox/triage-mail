<script lang="ts">
	import { onDestroy } from 'svelte';
	import Download from '@lucide/svelte/icons/download';
	import HardDrive from '@lucide/svelte/icons/hard-drive';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';
	import { ONNX_MODEL_META } from './settings.fixtures';
	import type { Settings, SettingsTransport } from './types';

	interface Props {
		settings: Settings;
		transport: SettingsTransport;
		busy: boolean;
		downloadingOnnx: boolean;
		onDownloadingChange: (downloading: boolean) => void;
		onReload?: () => void | Promise<void>;
	}

	let {
		settings,
		transport,
		busy,
		downloadingOnnx,
		onDownloadingChange,
		onReload
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].search);
	let onnxFeedback = $state<'ok' | 'error' | ''>('');
	let disposed = false;

	onDestroy(() => {
		disposed = true;
	});

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

		onDownloadingChange(true);
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
			if (!disposed) onDownloadingChange(false);
		}
	}
</script>

<section aria-label={t.modelTitle} class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div class="space-y-1">
			<div class="flex items-center gap-2 text-sm font-medium text-foreground">
				<HardDrive class="h-4 w-4 text-secondary" aria-hidden="true" />
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
				<label class="flex flex-col gap-2">
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
				<label class="flex flex-col gap-2">
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
						class="mt-1 h-3.5 w-3.5 accent-accent"
					/>
					<span>{t.modelConsent}</span>
				</label>
				<button
					type="submit"
					disabled={busy || downloadingOnnx}
					class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
				>
					<Download class="h-3.5 w-3.5" aria-hidden="true" />
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
