import { t } from '$lib/i18n';
import {
	canPreviewAttachment,
	createEphemeralPreview,
	type EphemeralAttachmentPreview
} from '$lib/reader/preview';
import type { AttachmentMeta } from '$lib/reader/types';
import { appStore } from '$lib/stores/app.svelte';
import type { MessageRowDto } from './types';
import type { InboxWorkspace } from './workspace.svelte';

export interface AttachmentActionContext {
	workspace: InboxWorkspace;
	message: MessageRowDto;
	isStillActive: () => boolean;
	setStatus: (partId: string, text: string | null) => void;
}

export async function saveAttachmentAction(
	ctx: AttachmentActionContext,
	att: AttachmentMeta
): Promise<void> {
	const saver = ctx.workspace.readerTransport.saveAttachment;
	if (!saver) {
		ctx.setStatus(att.part_id, t(appStore.locale, 'saveAttachmentError'));
		return;
	}
	ctx.setStatus(att.part_id, t(appStore.locale, 'savingAttachment'));
	try {
		const saved = await saver({
			account_id: ctx.message.account_id,
			folder: ctx.message.folder,
			uid: ctx.message.uid,
			part_id: att.part_id
		});
		// INVARIANT: A response from a dismissed message must never mutate active UI state.
		if (!ctx.isStillActive()) return;
		ctx.setStatus(att.part_id, saved === null ? null : t(appStore.locale, 'savedAttachment'));
	} catch (err: unknown) {
		if (ctx.isStillActive()) {
			ctx.setStatus(
				att.part_id,
				err === 'Unsupported'
					? t(appStore.locale, 'attachmentUnavailableOffline')
					: t(appStore.locale, 'saveAttachmentError')
			);
		}
	}
}

export async function previewAttachmentAction(
	ctx: AttachmentActionContext,
	att: AttachmentMeta,
	onReady: (preview: EphemeralAttachmentPreview) => void
): Promise<void> {
	if (!canPreviewAttachment(att)) {
		ctx.setStatus(att.part_id, t(appStore.locale, 'previewAttachmentUnsupported'));
		return;
	}
	ctx.setStatus(att.part_id, t(appStore.locale, 'previewingAttachment'));
	try {
		const { bytes } = await ctx.workspace.readerTransport.attachment({
			account_id: ctx.message.account_id,
			folder: ctx.message.folder,
			uid: ctx.message.uid,
			part_id: att.part_id
		});
		if (!ctx.isStillActive()) {
			bytes.fill(0);
			return;
		}
		onReady(createEphemeralPreview(bytes, att));
		ctx.setStatus(att.part_id, null);
	} catch (err: unknown) {
		if (ctx.isStillActive()) {
			ctx.setStatus(
				att.part_id,
				err === 'Unsupported'
					? t(appStore.locale, 'attachmentUnavailableOffline')
					: t(appStore.locale, 'previewAttachmentError')
			);
		}
	}
}
