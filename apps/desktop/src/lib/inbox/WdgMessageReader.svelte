<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import ShieldAlert from '@lucide/svelte/icons/shield-alert';
	import { getFontCssStack, getReaderPaletteColors } from '$lib/appearance/palettes';
	import { t } from '$lib/i18n';
	import { hasBlockedRemoteImages, sanitizeEmailHtml } from '$lib/security/sanitize';
	import type { EphemeralAttachmentPreview } from '$lib/reader/preview';
	import type { AttachmentMeta, MessageDetailDto } from '$lib/reader/types';
	import { appStore } from '$lib/stores/app.svelte';
	import { previewAttachmentAction, saveAttachmentAction } from './reader-attachments';
	import type { MessageRowDto } from './types';
	import WdgAttachmentPreviewModal from '../reader/WdgAttachmentPreviewModal.svelte';
	import WdgConversationThread from './WdgConversationThread.svelte';
	import WdgExtractionSummary from './WdgExtractionSummary.svelte';
	import WdgReaderAiSummary from './WdgReaderAiSummary.svelte';
	import WdgReaderAttachments from './WdgReaderAttachments.svelte';
	import WdgReaderHeader from './WdgReaderHeader.svelte';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = { workspace?: InboxWorkspace; onBackToList?: () => void };
	let { workspace = workspaceRegistry.active!, onBackToList }: Props = $props();

	let threadMember = $state<MessageRowDto | null>(null);
	const rootMessage = $derived(workspace.selected);
	const conversationRow = $derived(
		workspace.inbox.conversations && rootMessage ? workspace.inbox.conversationRows[rootMessage.id] : undefined
	);
	const message = $derived(conversationRow && threadMember ? threadMember : rootMessage);

	$effect(() => {
		rootMessage?.id;
		workspace.inbox.conversations;
		untrack(() => (threadMember = null));
	});

	const identity = $derived(
		message ? JSON.stringify([message.id, message.account_id, message.folder, message.uid]) : ''
	);

	let detail = $state.raw<MessageDetailDto | null>(null);
	let loading = $state(false);
	let error = $state('');
	let attempt = $state(0);
	let imagesFor = $state('');
	let savingPartId = $state<string | null>(null);
	let previewingPartId = $state<string | null>(null);
	let attachmentStatus = $state<Record<string, string>>({});
	let currentPreviewRef: EphemeralAttachmentPreview | null = null;
	let activePreview = $state.raw<EphemeralAttachmentPreview | null>(null);
	let previewAttachmentMeta = $state.raw<AttachmentMeta | null>(null);
	let active = true;

	function closePreview() {
		// INVARIANT: Always revoke ephemeral blob URLs and zero transient buffers immediately.
		currentPreviewRef?.release();
		currentPreviewRef = null;
		activePreview = null;
		previewAttachmentMeta = null;
	}

	function setAttachmentStatus(partId: string, statusText: string | null) {
		if (statusText === null) {
			const next = { ...attachmentStatus };
			delete next[partId];
			attachmentStatus = next;
		} else {
			attachmentStatus = { ...attachmentStatus, [partId]: statusText };
		}
	}

	onDestroy(() => {
		active = false;
		closePreview();
	});

	$effect(() => {
		identity;
		untrack(() => {
			closePreview();
			savingPartId = null;
			previewingPartId = null;
			attachmentStatus = {};
		});
		return () => {
			currentPreviewRef?.release();
			currentPreviewRef = null;
		};
	});

	$effect(() => {
		const id = message?.id;
		attempt;
		if (!id) {
			detail = null;
			loading = false;
			error = '';
			return;
		}
		let currentActive = true;
		loading = true;
		error = '';
		detail = null;
		void workspace.readerTransport
			.body(id)
			.then((result) => {
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
	const hasRemoteImages = $derived(Boolean(detail?.html_raw && hasBlockedRemoteImages(detail.html_raw)));
	const safeSrcDoc = $derived(
		detail?.html_raw
			? sanitizeEmailHtml(detail.html_raw, {
					allowRemoteImages: imagesAllowed,
					theme: appStore.theme,
					textScale: appStore.appearance.textScale,
					fontFamily: getFontCssStack(appStore.appearance.font),
					paletteColors: getReaderPaletteColors(appStore.appearance.palette, appStore.theme)
				})
			: ''
	);

	async function saveAttachmentAs(att: AttachmentMeta) {
		if (!message || savingPartId === att.part_id) return;
		const targetIdentity = identity;
		savingPartId = att.part_id;
		await saveAttachmentAction(
			{
				workspace,
				message,
				isStillActive: () => active && identity === targetIdentity,
				setStatus: setAttachmentStatus
			},
			att
		);
		if (active && identity === targetIdentity) savingPartId = null;
	}

	async function previewAttachment(att: AttachmentMeta) {
		if (!message || previewingPartId === att.part_id) return;
		const targetIdentity = identity;
		previewingPartId = att.part_id;
		await previewAttachmentAction(
			{
				workspace,
				message,
				isStillActive: () => active && identity === targetIdentity,
				setStatus: setAttachmentStatus
			},
			att,
			(preview) => {
				closePreview();
				currentPreviewRef = preview;
				activePreview = preview;
				previewAttachmentMeta = att;
			}
		);
		if (active && identity === targetIdentity) previewingPartId = null;
	}
</script>

<div class="flex h-full flex-col overflow-hidden bg-reader" aria-busy={loading}>
	{#if !message}
		<div class="flex flex-1 flex-col items-center justify-center p-8 text-center">
			<p class="max-w-sm text-xs text-secondary">{t(appStore.locale, 'noMessageSelected')}</p>
		</div>
	{:else}
		{#key identity}
			<WdgReaderHeader {message} {workspace} fallbackCategory={detail?.triage_result.category} {onBackToList} />

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
					<p class="px-6 py-8 text-xs text-secondary" role="status">{t(appStore.locale, 'loadingMessage')}</p>
				{:else if error}
					<div class="px-6 py-8 text-xs">
						<p role="alert" class="text-status-danger">{error}</p>
						<button
							type="button"
							onclick={() => attempt++}
							class="mt-3 rounded-md border border-separator bg-surface px-3 py-2 font-medium text-accent hover:bg-field"
						>
							{t(appStore.locale, 'tryAgain')}
						</button>
					</div>
				{:else if detail}
					<WdgExtractionSummary badges={detail.triage_badges} onCopy={(val, lbl) => void workspace.copyText(val, lbl)} />

					{#if detail.html_raw && hasRemoteImages}
						<div class="mx-4 mt-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs">
							<span class="flex items-center gap-2 text-secondary">
								<ShieldAlert class="h-4 w-4 shrink-0 text-status-warning" aria-hidden="true" />
								<span>{imagesAllowed ? t(appStore.locale, 'remoteImagesAllowed') : t(appStore.locale, 'remoteImagesBlocked')}</span>
							</span>
							<button
								type="button"
								onclick={() => (imagesFor = imagesAllowed ? '' : identity)}
								class="min-h-11 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
							>
								{imagesAllowed ? t(appStore.locale, 'blockRemoteImages') : t(appStore.locale, 'allowRemoteImages')}
							</button>
						</div>
					{/if}

					{#if safeSrcDoc}
						<div class="mx-4 my-3 min-h-[260px] flex-1 overflow-hidden rounded-lg border border-separator bg-reader">
							<iframe title="Email Content" sandbox="" referrerpolicy="no-referrer" srcdoc={safeSrcDoc} class="h-full min-h-[260px] w-full border-0 bg-reader"></iframe>
						</div>
					{:else}
						<pre class="mx-4 my-3 flex-1 rounded-lg border border-separator bg-reader p-4 font-sans text-sm leading-relaxed whitespace-pre-wrap break-words text-foreground">{detail.text_plain || t(appStore.locale, 'noDisplayableContent')}</pre>
					{/if}

					<WdgReaderAiSummary {workspace} messageId={message.id} {identity} />

					{#if detail.attachments_meta.length > 0}
						<WdgReaderAttachments
							attachments={detail.attachments_meta}
							{savingPartId}
							{previewingPartId}
							{attachmentStatus}
							onPreview={previewAttachment}
							onSaveAs={saveAttachmentAs}
						/>
					{/if}
				{/if}
			</div>
		{/key}
	{/if}
	<WdgAttachmentPreviewModal
		preview={activePreview}
		onClose={closePreview}
		onSaveAs={previewAttachmentMeta ? () => previewAttachmentMeta && void saveAttachmentAs(previewAttachmentMeta) : undefined}
		saving={previewAttachmentMeta !== null && savingPartId === previewAttachmentMeta.part_id}
	/>
</div>
