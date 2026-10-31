import { Channel, invoke } from '@tauri-apps/api/core';
import type { AccountTransport } from './types';
export const nativeAccountTransport: AccountTransport = {
	list: () => invoke('cmd_list_accounts'),
	unlock: (passphrase) => invoke('cmd_unlock_vault', { passphrase }),
	add: (config, credentials) => invoke('cmd_add_account', { config, credentials }),
	authorize(config, authorization, ready) {
		let active = true;
		const onReady = new Channel<null>();
		onReady.onmessage = () => {
			if (active) ready();
		};
		return invoke<string>('cmd_authorize_account', { config, authorization, onReady }).finally(
			() => {
				active = false;
				onReady.onmessage = () => {};
			}
		);
	},
	cancel: () => invoke('cmd_cancel_authorization')
};
