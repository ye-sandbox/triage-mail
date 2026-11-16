<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { Download, Paperclip, ShieldAlert, Sparkles } from 'lucide-svelte';
	import { formatBytes, t } from '$lib/i18n';
	import { hasBlockedRemoteImages, sanitizeEmailHtml } from '$lib/security/sanitize';
	import { safeFilename, saveAttachment } from '$lib/reader/download';
	import type { AttachmentMeta, MessageDetailDto } from '$lib/reader/types';
	import type { Settings } from '$lib/settings/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageRowDto } from './types';
	import WdgConversationThread from './WdgConversationThread.svelte';
	import WdgExtractionSummary from './WdgExtractionSummary.svelte';
	import WdgReaderHeader from './WdgReaderHeader.svelte';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace?: InboxWorkspace;
		onBackToList?: () => void;
	};

	let { workspace = workspaceRegistry.active!, onBackToList }: Props = $props();

	let threadMember = $state<MessageRowDto | null>(null);
	const rootMessage = $derived(workspace.selected);
	const conversationRow = $derived(
		workspace.inbox.conversations && rootMessage
			? workspace.inbox.conversationRows[rootMessage.id]
			: undefined
	);
	const message = $derived(
		conversationRow && threadMember ? threadMember : rootMessage
	);

	$effect(() => {
		rootMessage?.id;
		workspace.inbox.conversations;
		untrack(() => {
			threadMember = null;
		});
	});

	const identity = $derived(
		message
			? JSON.stringify([message.id, message.account_id, message.folder, message.uid])
			: ''
	);

	let detail = $state.raw<MessageDetailDto | null>(null);
	let loading = $state(false);
	let error = $state('');
	let attempt = $state(0);
	let imagesFor = $state('');

	let aiSettings = $state<Settings | null>(null);
	let aiChecking = $state(false);
	let aiBusy = $state(false);
	let aiError = $state('');
	let aiSummaries = $state<Record<string, string>>({});

	let fetchingPartId = $state<string | null>(null);
	let attachmentStatus = $state<Record<string, string>>({});
	let active = true;
	let releaseDownload: (() => void) | undefined;

	onDestroy(() => {
		active = false;
		releaseDownload?.();
	});

	$effect(() => {
		const id = message?.id;
		attempt;
		if (!id) {
			detail = null;
			loading = false;
			error = '';
			fetchingPartId = null;
			return;
		}
		let currentActive = true;
		loading = true;
		error = '';
		detail = null;
		aiSettings = null;
		aiError = '';
		fetchingPartId = null;

		void workspace.readerTransport
			.body(id)
			.then((result) => {
				// INVARIANT: Selection changes/unmount invalidate in-flight requests before content appears.
				if (!currentActive) return;
				if (result.id !== id) throw new Error('Message identity mismatch');
				detail = result;
				workspace.recordRowBadges(id, result.triage_badges);
			})
			.catch(() => {
				if (currentActive) error = t(appStore.locale, 'loadMessageError');
			})
			.finally(() => {
				if (currentActive) loading = false;
			});

		return () => {
			currentActive = false;
		};
	});

	const imagesAllowed = $derived(Boolean(identity && imagesFor === identity));
	const hasRemoteImages = $derived(
		Boolean(detail?.html_raw && hasBlockedRemoteImages(detail.html_raw))
	);
	const safeSrcDoc = $derived(
		detail?.html_raw
			? sanitizeEmailHtml(detail.html_raw, {
					allowRemoteImages: imagesAllowed,
					theme: appStore.theme
				})
			: ''
	);
	const aiSummary = $derived(message ? (aiSummaries[message.id] ?? null) : null);

	async function prepareSummary() {
		if (!message || aiChecking || aiBusy) return;
		const targetIdentity = identity;
		const targetId = message.id;
		aiChecking = true;
		aiError = '';
		try {
			const settings = await workspace.settingsTransport.get();
			if (!active || identity !== targetIdentity) return;
			if (!settings.ai.enabled) {
				aiError = 'Enable summaries in Settings first.';
				return;
			}
			if (workspace.isDemo) {
				await runSummary(targetId, settings);
			} else {
				aiSettings = settings;
			}
		} catch {
			if (active && identity === targetIdentity) aiError = 'AI settings could not be loaded.';
		} finally {
			if (active && identity === targetIdentity) aiChecking = false;
		}
	}

	async function runSummary(messageId: string, settings: Settings) {
		if (aiBusy) return;
		const targetIdentity = identity;
		aiBusy = true;
		aiError = '';
		try {
			const summarize =
				workspace.readerTransport.summarize ?? workspace.settingsTransport.summarize;
			const result = await summarize(messageId, true, { ...settings.ai });
			if (active && identity === targetIdentity) {
				aiSummaries = { ...aiSummaries, [messageId]: result };
			}
		} catch {
			if (active && identity === targetIdentity) {
				aiError = 'Summary failed. Check Settings before trying again.';
			}
		} finally {
			if (active && identity === targetIdentity) {
				aiBusy = false;
				aiSettings = null;
			}
		}
	}

	async function fetchAttachment(att: AttachmentMeta) {
		if (!message || fetchingPartId === att.part_id) return;
		const targetIdentity = identity;
		const currentMessage = message;
		fetchingPartId = att.part_id;
		attachmentStatus = {
			...attachmentStatus,
			[att.part_id]: t(appStore.locale, 'fetchingAttachment')
		};
		try {
			const { bytes } = await workspace.readerTransport.attachment({
				account_id: currentMessage.account_id,
				folder: currentMessage.folder,
				uid: currentMessage.uid,
				part_id: att.part_id
			});
			// INVARIANT: A response from a dismissed message must never trigger a file download.
			if (!active || identity !== targetIdentity) return;
			releaseDownload?.();
			releaseDownload = saveAttachment(bytes, att.filename);
			attachmentStatus = {
				...attachmentStatus,
				[att.part_id]: t(appStore.locale, 'downloaded')
			};
		} catch (err: unknown) {
			if (active && identity === targetIdentity) {
				attachmentStatus = {
					...attachmentStatus,
					[att.part_id]:
						err === 'Unsupported'
							? t(appStore.locale, 'attachmentUnavailableOffline')
							: 'Download failed. Please try again.'
				};
			}
		} finally {
			if (active && identity === targetIdentity) fetchingPartId = null;
		}
	}
</script>

<div class="flex h-full flex-col overflow-hidden bg-reader" aria-busy={loading}>
	{#if !message}
		<div class="flex flex-1 flex-col items-center justify-center p-8 text-center">
			<p class="max-w-sm text-xs text-secondary">{t(appStore.locale, 'noMessageSelected')}</p>
		</div>
	{:else}
		{#key identity}
			<WdgReaderHeader
				{message}
				{workspace}
				fallbackCategory={detail?.triage_result.category}
				{onBackToList}
			/>

			<div class="flex min-h-0 flex-1 flex-col overflow-y-auto">
				{#if conversationRow}
					<WdgConversationThread
						{workspace}
						row={conversationRow}
						activeMemberId={message.id}
						onSelectMember={(member) => (threadMember = member)}
					/>
				{/if}

				{#if loading}
					<p class="px-6 py-8 text-xs text-secondary" role="status">
						{t(appStore.locale, 'loadingMessage')}
					</p>
				{:else if error}
					<div class="px-6 py-8 text-xs">
						<p role="alert" class="text-status-danger">{error}</p>
						<button
							type="button"
							onclick={() => attempt++}
							class="mt-3 rounded-md border border-separator bg-surface px-3 py-1.5 font-medium text-accent hover:bg-field"
						>
							{t(appStore.locale, 'tryAgain')}
						</button>
					</div>
				{:else if detail}
					<WdgExtractionSummary
						badges={detail.triage_badges}
						onCopy={(val, lbl) => void workspace.copyText(val, lbl)}
					/>

					{#if detail.html_raw && hasRemoteImages}
						<div class="mx-4 mt-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs">
							<span class="flex items-center gap-2 text-secondary">
								<ShieldAlert class="h-4 w-4 shrink-0 text-status-warning" />
								<span>
									{imagesAllowed
										? t(appStore.locale, 'remoteImagesAllowed')
										: t(appStore.locale, 'remoteImagesBlocked')}
								</span>
							</span>
							<button
								type="button"
								onclick={() => (imagesFor = imagesAllowed ? '' : identity)}
								class="min-h-11 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
							>
								{imagesAllowed
									? t(appStore.locale, 'blockRemoteImages')
									: t(appStore.locale, 'allowRemoteImages')}
							</button>
						</div>
					{/if}

					{#if safeSrcDoc}
						<div class="mx-4 my-3 min-h-[260px] flex-1 overflow-hidden rounded-lg border border-separator bg-reader">
							<iframe
								title="Email Content"
								sandbox=""
								referrerpolicy="no-referrer"
								srcdoc={safeSrcDoc}
								class="h-full min-h-[260px] w-full border-0 bg-reader"
							></iframe>
						</div>
					{:else}
						<pre class="mx-4 my-3 flex-1 rounded-lg border border-separator bg-reader p-4 font-sans text-sm leading-relaxed whitespace-pre-wrap break-words text-foreground">{detail.text_plain || t(appStore.locale, 'noDisplayableContent')}</pre>
					{/if}

					<div class="mx-4 mb-3 rounded-lg border border-separator bg-surface p-3">
						<div class="flex flex-wrap items-center justify-between gap-2">
							<span class="text-xs font-medium text-foreground">On-Device AI Summary</span>
							<button
								type="button"
								disabled={aiChecking || aiBusy}
								onclick={() => void prepareSummary()}
								class="flex min-h-11 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] disabled:opacity-50 md:min-h-8"
							>
								<Sparkles class="h-3.5 w-3.5 text-accent" />
								<span>{aiChecking ? 'Checking settings…' : t(appStore.locale, 'summarizeLocalAi')}</span>
							</button>
						</div>
						{#if aiSettings}
							<div class="mt-2 space-y-2 rounded-md border border-separator bg-field p-2.5 text-xs">
								<p class="break-words text-secondary">
									Send this message’s subject and plain-text body (up to 8,000 characters) to
									{aiSettings.ai.endpoint} using {aiSettings.ai.model}?
									{aiSettings.ai.provider === 'Byok'
										? 'This leaves your device and may incur a charge.'
										: 'This goes to your local Ollama server.'}
								</p>
								<div class="flex gap-2">
									<button
										type="button"
										disabled={aiBusy}
										onclick={() => aiSettings && void runSummary(message.id, aiSettings)}
										class="rounded-md bg-accent px-3 py-1 text-xs font-medium text-accent-foreground disabled:opacity-50"
									>
										{aiBusy ? 'Summarizing…' : 'Send and summarize'}
									</button>
									<button
										type="button"
										disabled={aiBusy}
										onclick={() => (aiSettings = null)}
										class="rounded-md border border-separator bg-surface px-3 py-1 text-xs text-secondary hover:text-foreground"
									>
										Cancel
									</button>
								</div>
							</div>
						{/if}
						{#if aiError}
							<p role="alert" class="mt-2 text-xs text-status-danger">{aiError}</p>
						{/if}
						{#if aiSummary}
							<p class="mt-2 whitespace-pre-wrap break-words text-xs leading-relaxed text-secondary">
								{aiSummary}
							</p>
							<p class="mt-1 text-[11px] text-secondary">
								AI output can be inaccurate. Check the original message.
							</p>
						{/if}
					</div>

					{#if detail.attachments_meta.length > 0}
						<div class="mx-4 mb-4 rounded-lg border border-separator bg-surface p-3">
							<div class="mb-2 flex items-center gap-2 text-xs font-semibold text-foreground">
								<Paperclip class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'attachmentsTitle')}</span>
							</div>
							<div class="flex flex-col gap-2">
								{#each detail.attachments_meta as att (att.part_id)}
									{@const fetching = fetchingPartId === att.part_id}
									{@const status = attachmentStatus[att.part_id]}
									<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs">
										<div class="flex min-w-0 items-center gap-2">
											<span class="truncate font-medium text-foreground">
												{safeFilename(att.filename)}
											</span>
											<span class="text-secondary">·</span>
											<span class="font-mono text-[11px] text-secondary">{att.mime_type}</span>
											<span class="text-secondary">·</span>
											<span class="font-mono text-[11px] tabular-nums text-secondary">
												{formatBytes(att.size_bytes)}
											</span>
										</div>
										<button
											type="button"
											disabled={fetching}
											aria-label={`Download ${safeFilename(att.filename)}`}
											onclick={() => void fetchAttachment(att)}
											class="flex min-h-11 items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] disabled:opacity-50 md:min-h-8"
										>
											<Download class="h-3.5 w-3.5" />
											<span>{status || t(appStore.locale, 'fetchAttachment')}</span>
										</button>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				{/if}
			</div>
		{/key}
	{/if}
</div>
