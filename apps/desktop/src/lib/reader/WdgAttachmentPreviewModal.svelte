<script lang="ts">
	import { Dialog } from 'bits-ui';
	import Download from '@lucide/svelte/icons/download';
	import X from '@lucide/svelte/icons/x';
	import { formatBytes, t } from '$lib/i18n';
	import { safeFilename } from '$lib/reader/download';
	import type { EphemeralAttachmentPreview } from '$lib/reader/preview';
	import { appStore } from '$lib/stores/app.svelte';

	type Props = {
		preview: EphemeralAttachmentPreview | null;
		onClose: () => void;
		onSaveAs?: () => void;
		saving?: boolean;
	};

	let { preview, onClose, onSaveAs, saving = false }: Props = $props();

	const open = $derived(preview !== null);
</script>

<Dialog.Root
	{open}
	onOpenChange={(next) => {
		if (!next) onClose();
	}}
>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			data-testid="attachment-preview-dialog"
			class="fixed left-1/2 top-1/2 z-50 flex max-h-[88vh] w-[min(94vw,860px)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
		>
			{#if preview}
				{@const filename = safeFilename(preview.filename)}
				<div class="flex items-start justify-between gap-3 border-b border-separator pb-3">
					<div class="min-w-0">
						<Dialog.Title class="truncate font-display text-sm font-semibold text-foreground">
							{filename}
						</Dialog.Title>
						<Dialog.Description class="mt-1 text-[11px] text-secondary">
							<span class="font-mono">{preview.mimeType}</span>
							<span> · </span>
							<span class="font-mono tabular-nums">{formatBytes(preview.sizeBytes)}</span>
							<span> — {t(appStore.locale, 'previewEphemeralNotice')}</span>
						</Dialog.Description>
					</div>
					<div class="flex shrink-0 items-center gap-2">
						{#if onSaveAs}
							<button
								type="button"
								disabled={saving}
								onclick={onSaveAs}
								class="flex min-h-8 items-center gap-2 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
							>
								<Download class="h-3.5 w-3.5" aria-hidden="true" />
								<span>
									{saving ? t(appStore.locale, 'savingAttachment') : t(appStore.locale, 'saveAttachmentAs')}
								</span>
							</button>
						{/if}
						<Dialog.Close
							aria-label={t(appStore.locale, 'closePreview')}
							onclick={onClose}
							class="rounded p-1 text-secondary hover:bg-field hover:text-foreground"
						>
							<X class="h-4 w-4" aria-hidden="true" />
						</Dialog.Close>
					</div>
				</div>

				{#if preview.kind === 'image' && preview.blobUrl}
					<div class="flex max-h-[68vh] items-center justify-center overflow-auto rounded-md border border-separator bg-field p-3">
						<img
							src={preview.blobUrl}
							alt={filename}
							class="max-h-[64vh] max-w-full object-contain"
						/>
					</div>
				{:else if preview.kind === 'text' && preview.textContent !== null}
					<pre class="max-h-[68vh] overflow-auto rounded-md border border-separator bg-field p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-words text-foreground">{preview.textContent}</pre>
				{:else if preview.kind === 'pdf' && preview.blobUrl}
					<iframe
						title={filename}
						src={preview.blobUrl}
						sandbox="allow-scripts"
						referrerpolicy="no-referrer"
						class="h-[68vh] w-full rounded-md border border-separator bg-field"
					></iframe>
				{/if}
			{/if}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
