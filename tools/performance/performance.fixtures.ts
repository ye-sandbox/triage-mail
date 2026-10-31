import type { InboxTransport, MessageRowDto } from '$lib/inbox/types';
import type { ReaderTransport } from '$lib/reader/types';

export let rows: MessageRowDto[] = [];
export function createRows() {
	rows = Array.from({ length: 100_000 }, (_, i) => ({
		id: `synthetic-${i}`,
		account_id: 'synthetic',
		folder: 'INBOX',
		uid: i + 1,
		subject: `Synthetic invoice and shipment ${i}`,
		sender: 'sender@example.org',
		received_at_epoch_ms: 1_700_000_000_000 + i,
		is_read: i % 2 === 0,
		has_attachments: false,
		category_badge: 'Financial',
		tags: [],
		folder_override: null
	}));
	return rows;
}
export let pageCalls = 0;
export const transport: InboxTransport = {
	search: async ({ offset, limit }) => {
		pageCalls++;
		return rows.slice(offset, offset + limit);
	},
	subscribe: async () => () => {},
	triage: async () => {}
};
export const reader: ReaderTransport = {
	body: async (id) => ({
		id,
		subject: 'Synthetic',
		sender: 'sender@example.org',
		headers: '',
		text_plain: '',
		html_raw: null,
		triage_result: { category: 'Financial', badges: [] },
		triage_badges: [],
		attachments_meta: []
	}),
	attachment: async () => {
		throw new Error('No attachments in fixture');
	}
};
