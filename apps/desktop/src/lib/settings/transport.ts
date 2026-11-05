import { invoke } from '@tauri-apps/api/core';
import type { SettingsTransport } from './types';

export const nativeSettingsTransport: SettingsTransport = {
	embeddingState: () => invoke('cmd_embedding_state'),
	rebuildEmbeddings: (consent) => invoke('cmd_rebuild_embeddings', { consent }),
	get: () => invoke('cmd_get_settings'),
	save: (settings, apiKey) => invoke('cmd_save_settings', { settings, apiKey }),
	tokenizer: (tokenizer) => invoke('cmd_set_tokenizer', { tokenizer }),
	setExperimentalSemanticSearch: (enabled) =>
		invoke('cmd_set_experimental_semantic_search', { enabled }),
	download: (request) => invoke('cmd_download_model', { request }),
	summarize: (messageId, consent, expected) =>
		invoke('cmd_summarize_message', { messageId, consent, expected }),
};
