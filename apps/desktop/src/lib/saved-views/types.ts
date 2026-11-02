import type { MailboxScope, MessageRowDto, SearchMode } from '../inbox/types';
export type ExtractionKind = 'Boleto' | 'Invoice' | 'Pix' | 'Tracking' | 'Otp' | 'Unsubscribe';
export type ViewFilters = {
	is_read: boolean | null;
	is_starred: boolean | null;
	has_attachments: boolean | null;
	tags: string[];
	sender: string | null;
	sender_domain: string | null;
	extraction: ExtractionKind | null;
	due_from: string | null;
	due_to: string | null;
	upcoming_days: number | null;
};
export type SavedView = {
	version: number;
	id: string;
	revision: number;
	name: string;
	query: string;
	mode: SearchMode;
	accounts: string[];
	scope: MailboxScope | null;
	filters: ViewFilters;
};
export interface SavedViewTransport {
	list(): Promise<SavedView[]>;
	save(view: SavedView): Promise<SavedView>;
	remove(id: string, revision: number): Promise<void>;
	search(id: string, revision: number, limit: number, offset: number): Promise<MessageRowDto[]>;
}
export function blankView(
	query = '',
	mode: SearchMode = 'Lexical',
	scope?: MailboxScope
): SavedView {
	return {
		version: 1,
		id: crypto.randomUUID(),
		revision: 0,
		name: '',
		query,
		mode,
		accounts: [],
		scope: scope ?? null,
		filters: {
			is_read: null,
			is_starred: null,
			has_attachments: null,
			tags: [],
			sender: null,
			sender_domain: null,
			extraction: null,
			due_from: null,
			due_to: null,
			upcoming_days: null
		}
	};
}
