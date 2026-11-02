import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { nativeTransport } from './transport';
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn().mockResolvedValue(undefined),
	Channel: class {},
	isTauri: () => true
}));
describe('triage IPC wire contract', () => {
	it('preserves the externally tagged enum and snake_case message IDs in the mutation envelope', async () => {
		for (const action of [
			'MarkRead',
			'MarkUnread',
			'Archive',
			'Trash',
			'OneClickUnsubscribe',
			'Star',
			'Unstar',
			'Unsnooze',
			{ Snooze: { until_epoch_ms: 1800000000000 } },
			{ Tag: 'Receipts' }
		] as const) {
			const mutation = { message_ids: ['synthetic-message'], action };
			await nativeTransport.triage(mutation);
			expect(invoke).toHaveBeenLastCalledWith('cmd_triage_action', { mutation });
		}
	});
});

it('uses camelCase account arguments and preserves scoped enum and server folder paths', async () => {
	await nativeTransport.listFolders?.('first');
	expect(invoke).toHaveBeenLastCalledWith('cmd_list_folders', { accountId: 'first' });
	const query = {
		query: 'invoice',
		mode: 'Lexical' as const,
		limit: 200,
		offset: 400,
		scope: { AccountView: { account_id: 'first', view: { Folder: 'Projects/Local' } } }
	};
	await nativeTransport.search(query);
	expect(invoke).toHaveBeenLastCalledWith('cmd_search_messages', { query });
});
