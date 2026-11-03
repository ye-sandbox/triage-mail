import { invoke } from '@tauri-apps/api/core';
import type { ConversationTransport } from './types';
export const nativeConversations: ConversationTransport = {
	progress: (accountId) => invoke('cmd_conversation_progress', { accountId }),
	search: (query, context) => invoke('cmd_search_conversations', { query, context }),
	members: (query, context, id, offset) =>
		invoke('cmd_conversation_members', { query, context, id, offset }),
	preview: (query, context, id, all, action) =>
		invoke('cmd_preview_conversation', { query, context, id, all, action }),
	apply: (token) => invoke('cmd_apply_conversation', { token }),
	status: (ids) => invoke('cmd_conversation_status', { ids })
};
