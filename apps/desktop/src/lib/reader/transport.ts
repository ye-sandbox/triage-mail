import { invoke } from '@tauri-apps/api/core';
import type { ReaderTransport, MessageDetailDto, AttachmentBytesDto } from './types';
export const nativeReaderTransport: ReaderTransport = {
	body: (messageId) => invoke<MessageDetailDto>('cmd_get_message_body', { messageId }),
	attachment: (request) => invoke<AttachmentBytesDto>('cmd_fetch_attachment', { request })
};
