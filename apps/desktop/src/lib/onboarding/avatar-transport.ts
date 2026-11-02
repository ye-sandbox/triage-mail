import { invoke } from '@tauri-apps/api/core';
import type { AvatarTransport } from './avatar-types';

export const nativeAvatarTransport: AvatarTransport = {
	get: (accountId) => invoke('cmd_get_account_avatar', { accountId }),
	set: (accountId, bytes) => invoke('cmd_set_account_avatar', { accountId, bytes }),
	remove: (accountId) => invoke('cmd_remove_account_avatar', { accountId })
};
