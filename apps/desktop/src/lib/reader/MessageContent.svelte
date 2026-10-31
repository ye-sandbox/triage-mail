<script lang="ts">
	import type { MessageRowDto } from '../inbox/types';
	import type { MessageDetailDto, ReaderTransport } from './types';
	import { sanitizeEmailHtml } from '../security/sanitize';
	import AttachmentDownload from './AttachmentDownload.svelte';
	import TriageBadges from '../inbox/TriageBadges.svelte';
	import MessageSummary from '../settings/MessageSummary.svelte';
	let { message, transport }: { message: MessageRowDto; transport: ReaderTransport } = $props();
	let detail = $state.raw<MessageDetailDto | null>(null);
	let loading = $state(true);
	let error = $state('');
	let attempt = $state(0);
	let imagesFor = $state('');
	const identity = $derived(
		JSON.stringify([message.id, message.account_id, message.folder, message.uid])
	);
	const imagesAllowed = $derived(imagesFor === identity);
	const safeHtml = $derived(
		detail?.html_raw ? sanitizeEmailHtml(detail.html_raw, { allowRemoteImages: imagesAllowed }) : ''
	);
	$effect(() => {
		const id = message.id;
		attempt;
		let active = true;
		loading = true;
		error = '';
		detail = null;
		void transport
			.body(id)
			.then((result) => {
				// INVARIANT: Selection changes/unmount invalidate in-flight requests before content appears.
				if (active) {
					if (result.id !== id) throw new Error('Message identity mismatch');
					detail = result;
				}
			})
			.catch(() => {
				if (active) error = 'Message could not be loaded. Please try again.';
			})
			.finally(() => {
				if (active) loading = false;
			});
		return () => {
			active = false;
		};
	});
</script>

<div class="flex min-h-0 flex-1 flex-col" aria-busy={loading}>
	{#if loading}
		<p class="px-8 py-8 text-sm text-[var(--secondary)]" role="status">Loading message…</p>
	{:else if error}
		<div class="px-8 py-8">
			<p role="alert">{error}</p>
			<button
				class="mt-4 rounded-lg px-4 py-2 text-sm text-[var(--accent)]"
				type="button"
				onclick={() => attempt++}>Try again</button
			>
		</div>
	{:else if detail}
		<div class="flex min-h-0 flex-1 flex-col overflow-y-auto">
			<div class="px-8 pt-4">
				<TriageBadges badges={detail.triage_badges} />
			</div>
			<p class="px-8 py-4 text-xs text-[var(--secondary)]">
				External links stay blocked. {imagesAllowed
					? 'HTTPS images are allowed for this message.'
					: 'Remote images are blocked. Loading images may reveal your IP and that you opened this message.'}
			</p>
			{#if detail.html_raw}<button
					type="button"
					class="self-start px-8 py-2 text-sm text-[var(--accent)]"
					onclick={() => (imagesFor = imagesAllowed ? '' : identity)}
					>{imagesAllowed ? 'Block images again' : 'Allow HTTPS images for this message'}</button
				>{/if}
			<MessageSummary messageId={message.id} />
			{#if safeHtml}
				<iframe
					title="Email content"
					sandbox=""
					referrerpolicy="no-referrer"
					srcdoc={safeHtml}
					class="min-h-80 w-full flex-1 border-0"
				></iframe>
			{:else}
				<pre
					class="px-8 py-4 font-sans text-sm leading-relaxed whitespace-pre-wrap break-words">{detail.text_plain ||
						'This message has no displayable content.'}</pre>
			{/if}
			{#if detail.attachments_meta.length}
				<section class="border-t border-[var(--separator)] px-8 py-4" aria-label="Attachments">
					<h3 class="mb-4 text-sm font-semibold">Attachments</h3>
					<ul class="space-y-4">
						{#each detail.attachments_meta as attachment (attachment.part_id)}
							<AttachmentDownload {attachment} {message} {transport} />
						{/each}
					</ul>
				</section>
			{/if}
		</div>
	{/if}
</div>
