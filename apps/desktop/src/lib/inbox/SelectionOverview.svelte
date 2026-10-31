<script lang="ts">
	import Icon from './Icon.svelte';
	import MessageContent from '../reader/MessageContent.svelte';
	import { nativeReaderTransport } from '../reader/transport';
	import type { ReaderTransport } from '../reader/types';
	import type { MessageRowDto } from './types';
	let {
		message,
		transport = nativeReaderTransport
	}: { message: MessageRowDto | undefined; transport?: ReaderTransport } = $props();
</script>

<section class="flex min-h-0 flex-1 flex-col bg-[var(--reader)]" aria-label="Message reader">
	{#key message ? JSON.stringify( [message.id, message.account_id, message.folder, message.uid] ) : ''}
		<div class="overview-content flex min-h-0 flex-1 flex-col" data-message={!!message}>
			{#if message}
				<div class="shrink-0 border-b border-[var(--separator)] px-8 py-4">
					<p class="mb-3 text-xs font-medium text-[var(--secondary)]">
						{message.folder}
					</p>
					<h2 class="text-2xl font-semibold tracking-tight break-words">
						{message.subject || '(No subject)'}
					</h2>
					<div class="mt-6 flex items-center gap-3">
						<span
							class="flex size-10 shrink-0 items-center justify-center rounded-full bg-[var(--selection-soft)] text-[var(--accent)]"
							><Icon name="mail" /></span
						>
						<div class="min-w-0">
							<p class="truncate text-sm font-medium">{message.sender}</p>
							<p class="mt-1 text-xs text-[var(--secondary)]">
								{new Date(message.received_at_epoch_ms).toLocaleString()}
							</p>
						</div>
					</div>
				</div>
				<MessageContent {message} {transport} />
			{:else}
				<div class="flex flex-1 flex-col items-center justify-center px-8 py-12 text-center">
					<span
						class="flex size-16 items-center justify-center rounded-2xl bg-[var(--selection-soft)] text-[var(--accent)]"
						><Icon name="inbox" size={28} /></span
					>
					<h2 class="mt-6 text-xl font-semibold tracking-tight">
						A little clarity for your inbox.
					</h2>
					<p class="mt-3 max-w-72 text-sm leading-relaxed text-[var(--secondary)]">
						Select a message to see its details.<br />Your mail stays on your device.
					</p>
					<div class="mt-8 flex items-center gap-2 text-xs text-[var(--secondary)]">
						<kbd class="keycap">↑</kbd><kbd class="keycap">↓</kbd><span
							>to move through messages</span
						>
					</div>
				</div>
			{/if}
		</div>
	{/key}
</section>
