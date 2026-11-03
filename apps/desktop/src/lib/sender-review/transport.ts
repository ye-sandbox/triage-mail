import { invoke } from '@tauri-apps/api/core';
import type { ReviewTransport } from './types';
export const nativeReview: ReviewTransport = {
	preferences: (accountId) => invoke('cmd_review_preferences', { accountId }),
	savePreferences: (value) => invoke('cmd_save_review_preferences', { value }),
	metadata: (messageId) => invoke('cmd_review_metadata', { messageId }),
	list: (accountId, offset) => invoke('cmd_list_sender_choices', { accountId, offset }),
	save: (value) => invoke('cmd_save_sender_choice', { value }),
	search: (query, reviewState) => invoke('cmd_search_sender_review', { query, reviewState })
};
