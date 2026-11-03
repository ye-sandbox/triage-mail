import { invoke } from '@tauri-apps/api/core';
import type { CategoryTransport } from './types';
export const nativeCategories: CategoryTransport = {
	metadata: (messageId) => invoke('cmd_category_metadata', { messageId }),
	correct: (messageId, revision, category) =>
		invoke('cmd_correct_category', { messageId, revision, category }),
	list: (offset = 0) => invoke('cmd_list_category_overrides', { offset }),
	save: (value) => invoke('cmd_save_category_override', { value }),
	search: (query, category) => invoke('cmd_search_category', { query, category })
};
