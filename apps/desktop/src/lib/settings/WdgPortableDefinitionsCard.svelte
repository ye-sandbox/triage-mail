<script lang="ts">
	import Download from '@lucide/svelte/icons/download';
	import FileJson from '@lucide/svelte/icons/file-json';
	import Upload from '@lucide/svelte/icons/upload';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';

	interface Props {
		useNativeFlow: boolean;
		anyBusy: boolean;
		portableStatus: string;
		portableError: boolean;
		onNativeExport: () => void | Promise<void>;
		onNativePreview: () => void | Promise<void>;
		onDemoExport: () => void;
		onDemoFile: (event: Event) => void | Promise<void>;
		onDemoSamplePreview: () => void;
	}

	let {
		useNativeFlow,
		anyBusy,
		portableStatus,
		portableError,
		onNativeExport,
		onNativePreview,
		onDemoExport,
		onDemoFile,
		onDemoSamplePreview
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].backup);
	let portableFileEl = $state<HTMLInputElement>();

	async function handleFileChange(event: Event) {
		await onDemoFile(event);
		if (portableFileEl) portableFileEl.value = '';
	}
</script>

<div class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="max-w-xl space-y-1">
			<div class="flex items-center gap-2 text-sm font-medium text-foreground">
				<FileJson class="h-4 w-4 text-secondary" aria-hidden="true" />
				<span>{t.portableTitle}</span>
			</div>
			<p class="text-xs text-secondary">
				{useNativeFlow ? t.portableDisclosure : t.portableDesc}
			</p>
		</div>

		<div class="flex shrink-0 flex-wrap items-center gap-2">
			{#if useNativeFlow}
				<button
					type="button"
					disabled={anyBusy}
					onclick={() => void onNativeExport()}
					class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
				>
					<Download class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t.exportJsonBtn}</span>
				</button>
				<button
					type="button"
					disabled={anyBusy}
					onclick={() => void onNativePreview()}
					class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
				>
					<Upload class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t.importJsonBtn}</span>
				</button>
			{:else}
				<input
					bind:this={portableFileEl}
					type="file"
					accept=".json"
					onchange={(e) => void handleFileChange(e)}
					class="hidden"
					aria-label="Import portable JSON file"
				/>
				<button
					type="button"
					onclick={onDemoExport}
					class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft"
				>
					<Download class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t.exportJsonBtn}</span>
				</button>
				<button
					type="button"
					onclick={() => portableFileEl?.click()}
					class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft"
				>
					<Upload class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t.importJsonBtn}</span>
				</button>
				<button
					type="button"
					onclick={onDemoSamplePreview}
					class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground hover:opacity-95"
				>
					{t.previewDiffBtn}
				</button>
			{/if}
		</div>
	</div>

	{#if portableStatus}
		<p
			role={portableError ? 'alert' : 'status'}
			class="text-xs {portableError ? 'text-status-danger' : 'text-status-healthy'}"
		>
			{portableStatus}
		</p>
	{/if}
</div>
