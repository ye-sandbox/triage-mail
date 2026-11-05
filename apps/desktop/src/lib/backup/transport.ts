import { invoke } from '@tauri-apps/api/core';
import type { Rule } from '../rules/types';
import type { SavedView } from '../saved-views/types';

export interface Definitions {
	version: number;
	rules: Rule[];
	views: SavedView[];
	accounts: string[];
	destinations: string[];
}
export interface Mapping {
	accounts: Record<string, string>;
	destinations: Record<string, string>;
}
export const transfer = {
	choose: (save: boolean) => invoke<string | null>('cmd_choose_transfer_file', { save }),
	backup: (path: string, passphrase: string) =>
		invoke<void>('cmd_write_backup', { path, passphrase }),
	export: (path: string) => invoke<void>('cmd_export_definitions', { path }),
	preview: (path: string) => invoke<Definitions>('cmd_preview_definitions', { path }),
	import: (path: string, mapping: Mapping, expected: Definitions, consent: boolean) =>
		invoke<void>('cmd_import_definitions', {
			path,
			mapping,
			expected,
			consent
		}),
	available: () => invoke<boolean>('cmd_recovery_available'),
	restore: (path: string, passphrase: string, consent: boolean) =>
		invoke<void>('cmd_restore_backup', { path, passphrase, consent }),
	destinations: () => invoke<{ id: string; name: string }[]>('cmd_list_alert_destinations')
};
export type TransferTransport = typeof transfer;
