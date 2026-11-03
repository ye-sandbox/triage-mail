// INVARIANT: Wire fields and enum casing mirror the established Rust IPC DTOs.
export type SearchMode = 'Lexical' | 'Semantic' | 'Hybrid';
export type MailboxView = 'Inbox' | 'Starred' | 'Snoozed' | { Folder: string };
export type MailboxScope =
	'GlobalAllMail' | { AccountView: { account_id: string; view: MailboxView } };
export type FolderDto = {
	account_id: string;
	folder_name: string;
	display_name: string;
	delimiter: string | null;
	role: string;
	is_selectable: boolean;
	total_messages: number | null;
	unread_messages: number | null;
};
export type SearchQueryDto = {
	query: string;
	mode: SearchMode;
	limit: number;
	offset: number;
	scope?: MailboxScope;
};
export type MessageRowDto = {
	id: string;
	account_id: string;
	folder: string;
	uid: number;
	subject: string;
	sender: string;
	received_at_epoch_ms: number;
	is_read: boolean;
	is_starred?: boolean;
	snooze_until_epoch_ms?: number | null;
	has_attachments: boolean;
	category_badge: string | null;
	tags: string[];
	folder_override: string | null;
	triage_badges?: import('../reader/types').TriageBadge[];
};
export type SyncState =
	'Syncing' | 'Idle' | 'Retrying' | 'MutationPending' | 'MutationFailed' | 'ResyncRequired';
export type SyncProgressEvent = {
	account_id: string;
	state: SyncState;
	new_messages_count: number;
	triage_alerts: unknown[];
	embedding_errors_count?: number;
};
export interface InboxTransport {
	conversations?: import('../conversations/types').ConversationTransport;
	review?: import('../sender-review/types').ReviewTransport;
	categories?: import('../categories/types').CategoryTransport;
	savedViews?: import('../saved-views/types').SavedViewTransport;
	listFolders?(accountId: string): Promise<FolderDto[]>;
	triage(mutation: import('./triage').TriageMutation): Promise<void>;
	search(query: SearchQueryDto): Promise<MessageRowDto[]>;
	subscribe(handler: (event: SyncProgressEvent) => void): Promise<() => void>;
}
