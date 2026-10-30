<script lang="ts">
	import Icon from './Icon.svelte';
	import type { MessageRowDto } from './types';
	let { message }: { message: MessageRowDto | undefined } = $props();
</script>

<section class="flex min-h-0 flex-1 flex-col bg-[var(--reader)]" aria-label="Message overview">
	{#if message}
		<div class="border-b border-[var(--separator)] px-8 py-8">
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
		<div
			class="flex flex-1 flex-col items-center justify-center px-8 py-12 text-center text-[var(--secondary)]"
		>
			<Icon name="shield" size={28} />
			<h3 class="mt-4 text-base font-medium text-[var(--text)]">Message overview</h3>
			<p class="mt-2 max-w-72 text-sm leading-relaxed">
				Content preview will be available with the secure message reader.
			</p>
			{#if message.category_badge}<span
					class="mt-6 rounded-full bg-[var(--selection-soft)] px-3 py-1 text-xs text-[var(--text)]"
					>{message.category_badge}</span
				>{/if}
		</div>
	{:else}
		<div class="flex flex-1 flex-col items-center justify-center px-8 py-12 text-center">
			<span
				class="flex size-16 items-center justify-center rounded-2xl bg-[var(--selection-soft)] text-[var(--accent)]"
				><Icon name="inbox" size={28} /></span
			>
			<h2 class="mt-6 text-xl font-semibold tracking-tight">A little clarity for your inbox.</h2>
			<p class="mt-3 max-w-72 text-sm leading-relaxed text-[var(--secondary)]">
				Select a message to see its details.<br />Your mail stays on your device.
			</p>
			<div class="mt-8 flex items-center gap-2 text-xs text-[var(--secondary)]">
				<kbd class="keycap">↑</kbd><kbd class="keycap">↓</kbd><span>to move through messages</span>
			</div>
		</div>
	{/if}
</section>
