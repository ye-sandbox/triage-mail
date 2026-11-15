import { invoke } from '@tauri-apps/api/core';
import type { Category, CategoryMetadata } from '../categories/types';
import type { AiSettings } from '../settings/types';
import type { ReaderTransport, MessageDetailDto, AttachmentBytesDto } from './types';
export const nativeReaderTransport: ReaderTransport = {
	body: (messageId) => invoke<MessageDetailDto>('cmd_get_message_body', { messageId }),
	attachment: (request) => invoke<AttachmentBytesDto>('cmd_fetch_attachment', { request }),
	summarize: (messageId, consent, expected: AiSettings) =>
		invoke<string>('cmd_summarize_message', { messageId, consent, expected }),
	categoryMetadata: (messageId) => invoke<CategoryMetadata>('cmd_category_metadata', { messageId }),
	correctCategory: (messageId, revision, category: Category | null) =>
		invoke<CategoryMetadata>('cmd_correct_category', { messageId, revision, category })
};
