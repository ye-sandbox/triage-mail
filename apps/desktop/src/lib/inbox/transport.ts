import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import type { FolderDto, InboxTransport, MessageRowDto, SyncProgressEvent } from './types';

export function nativeAvailable() {
	return isTauri();
}
export const nativeTransport: InboxTransport = {
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
