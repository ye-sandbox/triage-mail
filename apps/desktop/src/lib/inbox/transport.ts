import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import type { InboxTransport, MessageRowDto, SyncProgressEvent } from './types';

export function nativeAvailable() {
	return isTauri();
}
export const nativeTransport: InboxTransport = {
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
