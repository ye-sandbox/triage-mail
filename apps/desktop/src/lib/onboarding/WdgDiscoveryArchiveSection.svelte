<script lang="ts">
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import FileUp from '@lucide/svelte/icons/file-up';
	import { i18n } from '../i18n/index.svelte';
	import type { LocalArchiveFormat } from './types';

	interface Props {
		demoMode: boolean;
		archivePath: string;
		archiveFormat: LocalArchiveFormat;
		importProgress: { active: boolean; importedCount: number };
		onFormatChange: (fmt: LocalArchiveFormat) => void;
		onChooseNative: () => void | Promise<void>;
		onDemoFileChange: (event: Event) => void;
	}

	let {
		demoMode,
		archivePath,
		archiveFormat,
		importProgress,
		onFormatChange,
		onChooseNative,
		onDemoFileChange
	}: Props = $props();

	let demoArchiveInput = $state<HTMLInputElement>();
	const t = $derived(i18n.t.accountForm);
</script>

<div class="flex items-start gap-3 rounded-md border border-separator bg-field px-4 py-3 text-xs text-secondary">
	<FileArchive class="mt-1 h-4 w-4 shrink-0 text-accent" aria-hidden="true" />
	<p class="leading-relaxed">{t.localArchiveNotice}</p>
</div>

<div class="flex flex-col gap-3 rounded-md border border-separator bg-field p-4">
	<div class="flex flex-col gap-2">
		<span class="text-xs font-medium text-secondary">{t.localArchiveFormatLabel}</span>
		<div role="radiogroup" aria-label={t.localArchiveFormatLabel} class="grid grid-cols-1 gap-2 sm:grid-cols-2">
			<button
				type="button"
				role="radio"
				aria-checked={archiveFormat === 'Mbox'}
				onclick={() => onFormatChange('Mbox')}
				class="flex min-h-[38px] items-center justify-center rounded-md border px-3 py-2 text-xs font-medium transition-colors {archiveFormat ===
				'Mbox'
					? 'border-accent bg-selection-soft text-accent'
					: 'border-separator bg-surface text-secondary hover:text-foreground'}"
			>
				{t.localArchiveFormatMbox}
			</button>
			<button
				type="button"
				role="radio"
				aria-checked={archiveFormat === 'Eml'}
				onclick={() => onFormatChange('Eml')}
				class="flex min-h-[38px] items-center justify-center rounded-md border px-3 py-2 text-xs font-medium transition-colors {archiveFormat ===
				'Eml'
					? 'border-accent bg-selection-soft text-accent'
					: 'border-separator bg-surface text-secondary hover:text-foreground'}"
			>
				{t.localArchiveFormatEml}
			</button>
		</div>
	</div>

	<div class="flex flex-wrap items-center justify-between gap-2">
		<span class="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
			{archivePath || t.localArchiveNoFile}
		</span>
		{#if demoMode}
			<input
				bind:this={demoArchiveInput}
				type="file"
				accept=".mbox,.eml,message/rfc822"
				onchange={onDemoFileChange}
				class="sr-only"
				aria-label={t.localArchiveChooseFile}
			/>
		{/if}
		<button
			type="button"
			onclick={() => (demoMode ? demoArchiveInput?.click() : void onChooseNative())}
			class="flex min-h-[36px] shrink-0 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-selection-soft"
		>
			<FileUp class="h-3.5 w-3.5 shrink-0 text-accent" aria-hidden="true" />
			<span>{t.localArchiveChooseFile}</span>
		</button>
	</div>

	{#if importProgress.active}
		<p role="status" class="font-mono text-xs tabular-nums text-status-info">
			{t.localArchiveProgress} {importProgress.importedCount}
		</p>
	{/if}
</div>
