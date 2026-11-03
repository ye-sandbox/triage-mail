import type { MessageRowDto, SearchQueryDto } from '../inbox/types';
export type ReviewState = 'Known' | 'Pending' | 'Accepted' | 'Declined';
export type ReviewFilter = 'Pending' | 'Declined';
export type ReviewPreferences = {
	account_id: string;
	revision: number;
	enabled: boolean;
	hide_in_inbox: boolean;
	initializing: boolean;
};
export type SenderChoice = {
	account_id: string;
	sender: string;
	revision: number;
	state: ReviewState;
};
export interface ReviewTransport {
	preferences(accountId: string): Promise<ReviewPreferences>;
	savePreferences(value: ReviewPreferences): Promise<ReviewPreferences>;
	metadata(messageId: string): Promise<SenderChoice | null>;
	list(accountId: string, offset: number): Promise<SenderChoice[]>;
	save(value: SenderChoice): Promise<SenderChoice>;
	search(query: SearchQueryDto, state: ReviewFilter): Promise<MessageRowDto[]>;
}
