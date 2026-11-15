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
		const ai = { enabled: true, provider: 'Ollama' as const, endpoint: 'http://127.0.0.1:11434', model: 'llama3' };
		await nativeReaderTransport.summarize?.('message-local', true, ai);
		expect(invoke).toHaveBeenLastCalledWith('cmd_summarize_message', {
			messageId: 'message-local',
			consent: true,
			expected: ai
		});
		await nativeReaderTransport.categoryMetadata?.('message-local');
		expect(invoke).toHaveBeenLastCalledWith('cmd_category_metadata', { messageId: 'message-local' });
		await nativeReaderTransport.correctCategory?.('message-local', 2, 'Financial');
		expect(invoke).toHaveBeenLastCalledWith('cmd_correct_category', {
			messageId: 'message-local',
			revision: 2,
			category: 'Financial'
		});
	});
});
