import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { nativeReaderTransport } from './transport';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue({}) }));
describe('reader IPC wire contract', () => {
	it('uses camelCase invocation arguments and unchanged snake_case DTO fields', async () => {
		await nativeReaderTransport.body('message-local');
		expect(invoke).toHaveBeenLastCalledWith('cmd_get_message_body', { messageId: 'message-local' });
		const request = { account_id: 'local', folder: 'INBOX', uid: 1, part_id: '2' };
		await nativeReaderTransport.attachment(request);
		expect(invoke).toHaveBeenLastCalledWith('cmd_fetch_attachment', { request });
	});
});
