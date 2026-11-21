import { Channel, invoke } from '@tauri-apps/api/core';
import type { ConnectionDiagnosticCode, SyncProgressEvent } from '../inbox/types';
import type { AccountAlias, AccountAliasMap } from './aliases';
import type { AccountSubscription } from './subscriptions';
import type {
	AccountTransport,
	ConnectionTestResult,
	LocalArchiveImportResult,
	VaultStatus
} from './types';
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
	testConnection: (config, credentials) =>
		invoke<ConnectionTestResult>('cmd_test_account_connection', { config, credentials }),
	reauthorize(reauthorization, ready) {
		let active = true;
		const onReady = new Channel<null>();
		onReady.onmessage = () => {
			if (active) ready();
		};
		return invoke<string>('cmd_reauthorize_account', { reauthorization, onReady }).finally(() => {
			active = false;
			onReady.onmessage = () => {};
		});
	},
	diagnostics: () => invoke<Record<string, ConnectionDiagnosticCode>>('cmd_account_diagnostics'),
	subscriptions: () => invoke<Record<string, AccountSubscription>>('cmd_account_subscriptions'),
	setBadge: (accountId, overrideMode) =>
		invoke<AccountSubscription>('cmd_set_account_badge', { accountId, overrideMode }),
	aliases: () => invoke<AccountAliasMap>('cmd_account_aliases'),
	setAliases: (accountId, aliases) =>
		invoke<AccountAlias[]>('cmd_set_account_aliases', { accountId, aliases }),
	openGoogleAppPasswords: () => invoke<void>('cmd_open_google_app_passwords'),
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
