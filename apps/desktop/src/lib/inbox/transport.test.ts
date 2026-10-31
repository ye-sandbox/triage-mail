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
			{ Tag: 'Receipts' }
		] as const) {
			const mutation = { message_ids: ['synthetic-message'], action };
			await nativeTransport.triage(mutation);
			expect(invoke).toHaveBeenLastCalledWith('cmd_triage_action', { mutation });
		}
	});
});
