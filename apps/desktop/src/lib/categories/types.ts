import type { MessageRowDto, SearchQueryDto } from '../inbox/types';
export const categories = [
	'Personal',
	'Financial',
	'Shopping',
	'Newsletter',
	'Notifications',
	'Security'
] as const;
export type Category = (typeof categories)[number];
export type CategoryMetadata = {
	automatic: Category;
	effective: Category;
	reason:
		| 'SecuritySignal'
		| 'FinancialSignal'
		| 'PurchaseSignal'
		| 'NewsletterSignal'
		| 'NotificationSignal'
		| 'Fallback';
	version: number;
	source: 'Automatic' | 'Message' | 'Sender';
	correction_revision: number;
	sender_revision: number;
	sender: string | null;
	durable: boolean;
};
export type SenderOverride = {
	account_id: string;
	sender: string;
	revision: number;
	category: Category | null;
};
export interface CategoryTransport {
	metadata(messageId: string): Promise<CategoryMetadata>;
	correct(
		messageId: string,
		revision: number,
		category: Category | null
	): Promise<CategoryMetadata>;
	list(offset?: number): Promise<SenderOverride[]>;
	save(value: SenderOverride): Promise<SenderOverride>;
	search(query: SearchQueryDto, category: Category): Promise<MessageRowDto[]>;
}
