<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { MessageRowDto } from '../inbox/types';
	import type { AttachmentMeta, ReaderTransport } from './types';
	import { safeFilename, saveAttachment } from './download';
	let {
		attachment,
		message,
		transport
	}: {
		attachment: AttachmentMeta;
		message: MessageRowDto;
		transport: ReaderTransport;
	} = $props();
	let status = $state('');
	let busy = $state(false);
	let active = true;
	let release: (() => void) | undefined;
	onDestroy(() => {
		active = false;
		release?.();
	});
	async function download() {
		if (busy) return;
		busy = true;
		status = 'Downloading…';
		try {
			const { bytes } = await transport.attachment({
				account_id: message.account_id,
				folder: message.folder,
				uid: message.uid,
				part_id: attachment.part_id
			});
			// INVARIANT: A response from a dismissed message must never trigger a file download.
			if (!active) return;
			release?.();
			release = saveAttachment(bytes, attachment.filename);
			status = 'Download requested';
		} catch {
			if (active) status = 'Download failed. Please try again.';
		} finally {
			if (active) busy = false;
		}
	}
</script>

<li class="rounded-lg border border-[var(--separator)] p-4">
	<div class="flex items-center justify-between gap-4">
		<div class="min-w-0">
			<p class="truncate text-sm font-medium">{safeFilename(attachment.filename)}</p>
			<p class="text-xs tabular-nums text-[var(--secondary)]">
				{attachment.size_bytes.toLocaleString()} bytes{attachment.is_inline ? ' · Inline' : ''}
			</p>
		</div>
		<button
			type="button"
			class="shrink-0 rounded-lg px-4 py-2 text-sm text-[var(--accent)] disabled:opacity-50"
			onclick={download}
			disabled={busy}
			aria-label={`Download ${safeFilename(attachment.filename)}`}>Download</button
		>
	</div>
	<p class="mt-2 text-xs text-[var(--secondary)]" role="status">{status}</p>
</li>
