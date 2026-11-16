import { Channel, invoke } from '@tauri-apps/api/core';
import type { SyncProgressEvent } from '../inbox/types';
import type { AccountTransport, LocalArchiveImportResult, VaultStatus } from './types';
export const nativeAccountTransport: AccountTransport = {
	list: () => invoke('cmd_list_accounts'),
	vaultStatus: () => invoke<VaultStatus>('cmd_vault_status'),
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
	cancel: () => invoke('cmd_cancel_authorization'),
	chooseArchiveFile: () => invoke<string | null>('cmd_choose_transfer_file', { save: false }),
	importLocalArchive(path, emailDisplay, format, onProgress) {
		let active = true;
		const onProgressChannel = new Channel<SyncProgressEvent>();
		onProgressChannel.onmessage = (event) => {
			if (active) onProgress?.(event);
		};
		return invoke<LocalArchiveImportResult>('cmd_import_local_archive', {
			path,
			emailDisplay,
			format,
			onProgress: onProgressChannel
		}).finally(() => {
			active = false;
			onProgressChannel.onmessage = () => {};
		});
	}
};
