export interface EmbeddingState {
	status:
		| 'Disabled'
		| 'MissingModelOrRuntime'
		| 'Rebuilding'
		| 'Failed'
		| 'Ready'
		| 'Unavailable';
	indexed: number;
	pending: number;
	failed: number;
}
export type Tokenizer = 'Unicode61RemoveDiacritics' | 'Trigram';
export interface AiSettings {
	enabled: boolean;
	provider: 'Ollama' | 'Byok';
	endpoint: string;
	model: string;
}
export interface Settings {
	ai: AiSettings;
	tokenizer: Tokenizer;
	has_api_key: boolean;
	model_cached: boolean;
	experimental_semantic_search: boolean;
}
export interface SettingsTransport {
	embeddingState?(): Promise<EmbeddingState>;
	rebuildEmbeddings?(consent: boolean): Promise<void>;
	get(): Promise<Settings>;
	save(settings: AiSettings, apiKey: string): Promise<void>;
	tokenizer(tokenizer: Tokenizer): Promise<void>;
	setExperimentalSemanticSearch(enabled: boolean): Promise<void>;
	download(request: {
		model_url: string;
		tokenizer_url: string;
		consent: boolean;
	}): Promise<void>;
	summarize(
		messageId: string,
		consent: boolean,
		expected: AiSettings,
	): Promise<string>;
}
