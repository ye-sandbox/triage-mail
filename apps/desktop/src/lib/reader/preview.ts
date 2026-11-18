import type { AttachmentMeta } from './types';

export const MAX_PREVIEW_BYTES = 10 * 1024 * 1024;

export type PreviewKind = 'image' | 'text' | 'pdf';

const RASTER_IMAGE_MIMES = new Set(['image/png', 'image/jpeg', 'image/webp', 'image/gif']);
const TEXT_MIMES = new Set(['text/plain', 'text/csv', 'application/json', 'application/xml']);

export function normalizeMimeType(mimeType: string): string {
	return (mimeType.split(';', 1)[0] ?? '').trim().toLowerCase();
}

export function classifyPreviewMime(mimeType: string): PreviewKind | null {
	const normalized = normalizeMimeType(mimeType);
	// INVARIANT: SVG can carry scripts/event handlers and must never enter inline preview.
	if (normalized === 'image/svg+xml') return null;
	if (RASTER_IMAGE_MIMES.has(normalized)) return 'image';
	if (TEXT_MIMES.has(normalized)) return 'text';
	if (normalized === 'application/pdf') return 'pdf';
	return null;
}

export function canPreviewAttachment(
	att: Pick<AttachmentMeta, 'mime_type' | 'size_bytes'>
): boolean {
	return (
		classifyPreviewMime(att.mime_type) !== null &&
		att.size_bytes >= 0 &&
		att.size_bytes <= MAX_PREVIEW_BYTES
	);
}

export interface EphemeralAttachmentPreview {
	kind: PreviewKind;
	filename: string;
	mimeType: string;
	sizeBytes: number;
	blobUrl: string | null;
	textContent: string | null;
	release: () => void;
}

export function createEphemeralPreview(
	rawBytes: number[] | Uint8Array,
	att: Pick<AttachmentMeta, 'filename' | 'mime_type' | 'size_bytes'>
): EphemeralAttachmentPreview {
	const kind = classifyPreviewMime(att.mime_type);
	if (!kind) {
		if (Array.isArray(rawBytes)) rawBytes.fill(0);
		else rawBytes.fill(0);
		throw new Error('Unsupported preview MIME type');
	}
	const mimeType = normalizeMimeType(att.mime_type);
	let buffer: Uint8Array<ArrayBuffer>;
	if (rawBytes instanceof Uint8Array && rawBytes.buffer instanceof ArrayBuffer) {
		buffer = rawBytes as Uint8Array<ArrayBuffer>;
	} else {
		buffer = new Uint8Array(rawBytes);
		rawBytes.fill(0);
	}
	if (buffer.byteLength > MAX_PREVIEW_BYTES) {
		buffer.fill(0);
		throw new Error('Preview attachment exceeds 10 MiB limit');
	}

	let released = false;
	if (kind === 'text') {
		const textContent = new TextDecoder('utf-8', { fatal: false }).decode(buffer);
		// INVARIANT: Zero transient byte buffer immediately once UTF-8 text is decoded.
		buffer.fill(0);
		return {
			kind,
			filename: att.filename,
			mimeType,
			sizeBytes: att.size_bytes,
			blobUrl: null,
			textContent,
			release() {
				if (released) return;
				released = true;
			}
		};
	}

	const blob = new Blob([buffer], { type: mimeType });
	const blobUrl = URL.createObjectURL(blob);
	return {
		kind,
		filename: att.filename,
		mimeType,
		sizeBytes: att.size_bytes,
		blobUrl,
		textContent: null,
		release() {
			if (released) return;
			released = true;
			// INVARIANT: Revoke ephemeral blob URL and wipe transient buffer immediately on teardown.
			URL.revokeObjectURL(blobUrl);
			buffer.fill(0);
		}
	};
}
