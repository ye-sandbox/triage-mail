import { expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { nativeSettingsTransport } from './transport';
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn().mockResolvedValue(undefined),
}));

it('uses native settings wire names and explicit consent envelopes', async () => {
	const ai = {
		enabled: true,
		provider: 'Ollama' as const,
		endpoint: 'http://127.0.0.1:11434',
		model: 'model',
	};
	await nativeSettingsTransport.embeddingState!();
	await nativeSettingsTransport.rebuildEmbeddings!(true);
	await nativeSettingsTransport.get();
	await nativeSettingsTransport.save(ai, 'synthetic-private-value');
	await nativeSettingsTransport.tokenizer('Trigram');
	await nativeSettingsTransport.setExperimentalSemanticSearch(true);
	await nativeSettingsTransport.download({
		model_url: 'https://models.example.com/m',
		tokenizer_url: 'https://models.example.com/t',
		consent: true,
	});
	await nativeSettingsTransport.summarize('message-one', true, ai);
	expect(vi.mocked(invoke).mock.calls).toEqual([
		['cmd_embedding_state'],
		['cmd_rebuild_embeddings', { consent: true }],
		['cmd_get_settings'],
		['cmd_save_settings', { settings: ai, apiKey: 'synthetic-private-value' }],
		['cmd_set_tokenizer', { tokenizer: 'Trigram' }],
		['cmd_set_experimental_semantic_search', { enabled: true }],
		[
			'cmd_download_model',
			{
				request: {
					model_url: 'https://models.example.com/m',
					tokenizer_url: 'https://models.example.com/t',
					consent: true,
				},
			},
		],
		[
			'cmd_summarize_message',
			{ messageId: 'message-one', consent: true, expected: ai },
		],
	]);
});
