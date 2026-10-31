// INVARIANT: Wire fields and enum casing mirror the established Rust IPC DTOs.
export type SearchMode = 'Lexical' | 'Semantic' | 'Hybrid';
export type SearchQueryDto = {
	query: string;
	mode: SearchMode;
	limit: number;
	offset: number;
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
	has_attachments: boolean;
	category_badge: string | null;
};
export type SyncState =
	'Syncing' | 'Idle' | 'Retrying' | 'MutationPending' | 'MutationFailed' | 'ResyncRequired';
export type SyncProgressEvent = {
	account_id: string;
	state: SyncState;
	new_messages_count: number;
	triage_alerts: unknown[];
};
export interface InboxTransport {
	triage(mutation: import('./triage').TriageMutation): Promise<void>;
	search(query: SearchQueryDto): Promise<MessageRowDto[]>;
	subscribe(handler: (event: SyncProgressEvent) => void): Promise<() => void>;
}
