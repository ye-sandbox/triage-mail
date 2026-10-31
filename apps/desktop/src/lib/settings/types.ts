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
}
export interface SettingsTransport {
	get(): Promise<Settings>;
	save(settings: AiSettings, apiKey: string): Promise<void>;
	tokenizer(tokenizer: Tokenizer): Promise<void>;
	download(request: { model_url: string; tokenizer_url: string; consent: boolean }): Promise<void>;
	summarize(messageId: string, consent: boolean, expected: AiSettings): Promise<string>;
}
