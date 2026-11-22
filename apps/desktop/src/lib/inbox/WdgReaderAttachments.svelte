<script lang="ts">
	import Download from '@lucide/svelte/icons/download';
	import Eye from '@lucide/svelte/icons/eye';
	import Paperclip from '@lucide/svelte/icons/paperclip';
	import { formatBytes, t } from '$lib/i18n';
	import { safeFilename } from '$lib/reader/download';
	import { canPreviewAttachment } from '$lib/reader/preview';
	import type { AttachmentMeta } from '$lib/reader/types';
	import { appStore } from '$lib/stores/app.svelte';

	type Props = {
		attachments: AttachmentMeta[];
		savingPartId: string | null;
		previewingPartId: string | null;
		attachmentStatus: Record<string, string>;
		onPreview: (att: AttachmentMeta) => void | Promise<void>;
		onSaveAs: (att: AttachmentMeta) => void | Promise<void>;
	};

	let {
		attachments,
		savingPartId,
		previewingPartId,
		attachmentStatus,
		onPreview,
		onSaveAs
	}: Props = $props();
</script>

<div class="mx-4 mb-4 rounded-lg border border-separator bg-surface p-3">
	<div class="mb-2 flex items-center gap-2 text-xs font-semibold text-foreground">
		<Paperclip class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
		<span>{t(appStore.locale, 'attachmentsTitle')}</span>
	</div>
	<div class="flex flex-col gap-2">
		{#each attachments as att (att.part_id)}
			{@const filename = safeFilename(att.filename)}
			{@const saving = savingPartId === att.part_id}
			{@const previewing = previewingPartId === att.part_id}
			{@const previewSupported = canPreviewAttachment(att)}
			{@const status = attachmentStatus[att.part_id]}
			<div class="flex flex-col gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<div class="flex min-w-0 items-center gap-2">
						<span class="truncate font-medium text-foreground">
							{filename}
						</span>
						<span class="text-secondary">·</span>
						<span class="font-mono text-[11px] text-secondary">{att.mime_type}</span>
						<span class="text-secondary">·</span>
						<span class="font-mono text-[11px] tabular-nums text-secondary">
							{formatBytes(att.size_bytes)}
						</span>
					</div>
					<div class="flex items-center gap-2">
						<button
							type="button"
							disabled={previewing || !previewSupported}
							aria-label={`Preview ${filename}`}
							title={!previewSupported
								? t(appStore.locale, 'previewAttachmentUnsupported')
								: undefined}
							onclick={() => void onPreview(att)}
							class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] disabled:opacity-50 md:min-h-8"
						>
							<Eye class="h-3.5 w-3.5" aria-hidden="true" />
							<span>
								{previewing
									? t(appStore.locale, 'previewingAttachment')
									: t(appStore.locale, 'previewAttachment')}
							</span>
						</button>
						<button
							type="button"
							disabled={saving}
							aria-label={`Save As ${filename}`}
							onclick={() => void onSaveAs(att)}
							class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] disabled:opacity-50 md:min-h-8"
						>
							<Download class="h-3.5 w-3.5" aria-hidden="true" />
							<span>
								{saving
									? t(appStore.locale, 'savingAttachment')
									: t(appStore.locale, 'saveAttachmentAs')}
							</span>
						</button>
					</div>
				</div>
				{#if status}
					<p role="status" class="text-[11px] text-secondary">{status}</p>
				{/if}
			</div>
		{/each}
	</div>
</div>
