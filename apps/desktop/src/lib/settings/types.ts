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
	get(): Promise<Settings>;
	save(settings: AiSettings, apiKey: string): Promise<void>;
	tokenizer(tokenizer: Tokenizer): Promise<void>;
	setExperimentalSemanticSearch(enabled: boolean): Promise<void>;
	download(request: { model_url: string; tokenizer_url: string; consent: boolean }): Promise<void>;
	summarize(messageId: string, consent: boolean, expected: AiSettings): Promise<string>;
}

export type SettingsTab = 'general' | 'accounts' | 'search' | 'ai' | 'privacy' | 'rules' | 'alerts' | 'reminders';

export interface SettingsTabItem {
	id: SettingsTab;
	label: string;
}

export const SETTINGS_TABS: readonly SettingsTabItem[] = [
	{ id: 'general', label: 'General' },
	{ id: 'accounts', label: 'Accounts' },
	{ id: 'search', label: 'Search' },
	{ id: 'ai', label: 'AI' },
	{ id: 'privacy', label: 'Privacy' },
	{ id: 'rules', label: 'Rules' },
	{ id: 'alerts', label: 'Alerts' },
	{ id: 'reminders', label: 'Reminders' }
] as const;

export interface SettingsModuleProps {
	settings: Settings;
	transport: SettingsTransport;
	busy?: boolean;
	onReload?: () => void | Promise<void>;
}
