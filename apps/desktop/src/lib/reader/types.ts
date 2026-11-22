// INVARIANT: Wire fields and externally tagged enums mirror Rust; these adapters never change IPC.
export type TriageBadge =
	| { OtpCode: { code: string } }
	| {
			FebrabanBoleto: {
				digitable_line: string;
				amount_cents: number | null;
				due_date_iso: string | null;
			};
	  }
	| { PixPayload: { payload: string } }
	| { NfeInvoice: { access_key: string } }
	| { TrackingCode: { carrier: string; code: string } }
	| { UnsubscribeAvailable: { supports_one_click: boolean } };
export type AttachmentMeta = {
	part_id: string;
	filename: string;
	mime_type: string;
	size_bytes: number;
	content_id: string | null;
	is_inline: boolean;
};
export type MessageDetailDto = {
	id: string;
	subject: string;
	sender: string;
	recipients?: string;
	headers: string;
	text_plain: string | null;
	html_raw: string | null;
	triage_result: {
		category: 'Security' | 'Financial' | 'Shopping' | 'Newsletter' | 'Personal' | 'Notifications';
		badges: TriageBadge[];
	};
	triage_badges: TriageBadge[];
	attachments_meta: AttachmentMeta[];
};
export type AttachmentRequestDto = {
	account_id: string;
	folder: string;
	uid: number;
	part_id: string;
};
export type AttachmentBytesDto = { bytes: number[] };
export type AttachmentSavedDto = { bytes_written: number };
import type { Category, CategoryMetadata } from '../categories/types';
import type { AiSettings } from '../settings/types';

export interface ReaderTransport {
	body(messageId: string): Promise<MessageDetailDto>;
	attachment(request: AttachmentRequestDto): Promise<AttachmentBytesDto>;
	saveAttachment?(request: AttachmentRequestDto): Promise<AttachmentSavedDto | null>;
	summarize?(messageId: string, consent: boolean, expected: AiSettings): Promise<string>;
	categoryMetadata?(messageId: string): Promise<CategoryMetadata>;
	correctCategory?(
		messageId: string,
		revision: number,
		category: Category | null
	): Promise<CategoryMetadata>;
}
