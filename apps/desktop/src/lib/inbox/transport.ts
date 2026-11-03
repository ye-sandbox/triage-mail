import { nativeConversations } from '../conversations/transport';
import { nativeReview } from '../sender-review/transport';
import { nativeCategories } from '../categories/transport';
import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import type { FolderDto, InboxTransport, MessageRowDto, SyncProgressEvent } from './types';

export function nativeAvailable() {
	return isTauri();
}
export const nativeTransport: InboxTransport = {
	conversations: nativeConversations,
	categories: nativeCategories,
	review: nativeReview,
	savedViews: {
		list: () => invoke('cmd_list_saved_views'),
		save: (view) => invoke('cmd_save_saved_view', { view }),
		remove: (id, revision) => invoke('cmd_delete_saved_view', { id, revision }),
		search: (id, revision, limit, offset) =>
			invoke('cmd_search_saved_view', { id, revision, limit, offset })
	},
	listFolders: (accountId) => invoke<FolderDto[]>('cmd_list_folders', { accountId }),
	triage: (mutation) => invoke<void>('cmd_triage_action', { mutation }),
	search: (query) => invoke<MessageRowDto[]>('cmd_search_messages', { query }),
	async subscribe(handler) {
		let active = true;
		const channel = new Channel<SyncProgressEvent>();
		channel.onmessage = (event) => {
			if (active) handler(event);
		};
		await invoke('cmd_subscribe_sync', { onProgress: channel });
		// INVARIANT: A replacement subscription belongs to this webview; teardown silences late callbacks.
		return () => {
			active = false;
			channel.onmessage = () => {};
		};
	}
};
