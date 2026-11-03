import type { MessageRowDto, SearchQueryDto, MailboxScope } from '../inbox/types';
import type { Category } from '../categories/types';
import type { ReviewFilter } from '../sender-review/types';
export type ConversationSource =
	| 'Messages'
	| { Category: Category }
	| { Review: ReviewFilter }
	| { Saved: { id: string; revision: number } };
export type ConversationContext = {
	source: ConversationSource;
	scope: MailboxScope | null;
	generation: string | null;
};
export type ConversationRow = {
	conversation_id: string;
	representative: MessageRowDto;
	matching_count: number;
	total_count: number;
	unread_count: number;
};
export type ConversationPage = { generation: string; rows: ConversationRow[] };
export type ConversationMember = {
	message: MessageRowDto;
	matching: boolean;
	pending: boolean;
};
export type ConversationMembers = {
	generation: string;
	rows: ConversationMember[];
	total: number;
};
export type ConversationAction = 'MarkRead' | 'MarkUnread' | 'Archive' | { Tag: string };
export type ConversationPreview = {
	token: string;
	generation: string;
	action: ConversationAction;
	messages: MessageRowDto[];
	outside_count: number;
};
export type MutationStatus = {
	message_id: string;
	pending: boolean;
	failed: boolean;
	attempts: number;
};
export interface ConversationTransport {
	progress(accountId?: string): Promise<{ indexed: number; total: number; ready: boolean }>;
	search(query: SearchQueryDto, context: ConversationContext): Promise<ConversationPage>;
	members(
		query: SearchQueryDto,
		context: ConversationContext,
		id: string,
		offset: number
	): Promise<ConversationMembers>;
	preview(
		query: SearchQueryDto,
		context: ConversationContext,
		id: string,
		all: boolean,
		action: ConversationAction
	): Promise<ConversationPreview>;
	apply(token: string): Promise<string[]>;
	status(ids: string[]): Promise<MutationStatus[]>;
}
