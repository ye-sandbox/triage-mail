import type { EmbeddingState, Settings, SettingsTransport } from './types';

export type PortableItemKind = 'rule' | 'saved_view' | 'category';
export type PortableDiffAction = 'added' | 'updated' | 'conflicting';

export interface PortableDefinitionDiffItem {
	id: string;
	kind: PortableItemKind;
	name: string;
	action: PortableDiffAction;
	summary: string;
	resolution?: 'keep_local' | 'overwrite';
}

export interface PortableImportPreview {
	source_filename: string;
	schema_version: string;
	exported_at_iso: string;
	counts: {
		added: number;
		updated: number;
		conflicting: number;
	};
	items: PortableDefinitionDiffItem[];
}

export const ONNX_MODEL_META = {
	revision: 'Xenova/paraphrase-multilingual-MiniLM-L12-v2@rev-8f31c2e',
	cacheDir: '$XDG_DATA_HOME/triage-mail/models',
	artifactSizeMb: 118.4,
	vectorDim: 384
} as const;

export const INITIAL_DEMO_SETTINGS: Settings = {
	ai: {
		enabled: true,
		provider: 'Ollama',
		endpoint: 'http://127.0.0.1:11434',
		model: 'qwen2.5:7b-instruct'
	},
	tokenizer: 'Unicode61RemoveDiacritics',
	has_api_key: true,
	model_cached: false,
	experimental_semantic_search: false
};

export const EMBEDDING_STATE_PRESETS: Record<EmbeddingState['status'], EmbeddingState> = {
	Ready: {
		status: 'Ready',
		indexed: 14820,
		pending: 0,
		failed: 0
	},
	Rebuilding: {
		status: 'Rebuilding',
		indexed: 9410,
		pending: 5402,
		failed: 8
	},
	MissingModelOrRuntime: {
		status: 'MissingModelOrRuntime',
		indexed: 0,
		pending: 14820,
		failed: 0
	},
	Failed: {
		status: 'Failed',
		indexed: 6120,
		pending: 8658,
		failed: 42
	},
	Disabled: {
		status: 'Disabled',
		indexed: 0,
		pending: 0,
		failed: 0
	},
	Unavailable: {
		status: 'Unavailable',
		indexed: 0,
		pending: 0,
		failed: 0
	}
};

export const PORTABLE_IMPORT_PREVIEW_FIXTURE: PortableImportPreview = {
	source_filename: 'triage-definitions-2026-11-14.json',
	schema_version: 'triage-portable-v1',
	exported_at_iso: '2026-11-14T12:15:00Z',
	counts: {
		added: 3,
		updated: 2,
		conflicting: 2
	},
	items: [
		{
			id: 'def_rule_nfe_archive',
			kind: 'rule',
			name: 'Auto-tag NF-e XML Invoices',
			action: 'added',
			summary: 'Matches NF-e 44-digit key; applies tag "fiscal/nfe"'
		},
		{
			id: 'def_rule_pix_receipt',
			kind: 'rule',
			name: 'Flag Incoming Pix Receipts',
			action: 'added',
			summary: 'Matches Pix payload header; notifies DesktopNotification'
		},
		{
			id: 'def_view_tax_quarter',
			kind: 'saved_view',
			name: 'Q4 Tax & Boletos',
			action: 'added',
			summary: 'Query: category:Financial has:boleto'
		},
		{
			id: 'def_rule_boleto_payable',
			kind: 'rule',
			name: 'Boleto Payable Alert',
			action: 'updated',
			summary: 'Adds threshold reminder tag "due-soon" without modifying destination'
		},
		{
			id: 'def_cat_notifications',
			kind: 'category',
			name: 'Notifications',
			action: 'updated',
			summary: 'Includes alerts@example.org and updates@example.net domain matchers'
		},
		{
			id: 'def_rule_newsletter_digest',
			kind: 'rule',
			name: 'Newsletter Digest Hold',
			action: 'conflicting',
			summary: 'Local rule archives immediately; imported rule assigns "reading-list" in Inbox',
			resolution: 'keep_local'
		},
		{
			id: 'def_view_security_otps',
			kind: 'saved_view',
			name: 'Security & 2FA Codes',
			action: 'conflicting',
			summary: 'Local view includes category:Security; imported view filters by otp:true only',
			resolution: 'keep_local'
		}
	]
};

export function createDemoSettingsTransport(): SettingsTransport {
	let current: Settings = structuredClone(INITIAL_DEMO_SETTINGS);
	let embedding: EmbeddingState = structuredClone(EMBEDDING_STATE_PRESETS.Disabled);
	let shellPrefs = {
		close_to_tray: true,
		minimize_to_tray: false
	};

	return {
		async get() {
			return structuredClone(current);
		},
		async save(ai, apiKey) {
			const hasKey =
				ai.provider === 'Ollama' ? false : apiKey.trim().length > 0 ? true : current.has_api_key;
			current = {
				...current,
				ai: { ...ai },
				has_api_key: hasKey
			};
		},
		async tokenizer(tokenizer) {
			current = { ...current, tokenizer };
		},
		async setExperimentalSemanticSearch(enabled) {
			current = { ...current, experimental_semantic_search: enabled };
			embedding = enabled
				? current.model_cached
					? structuredClone(EMBEDDING_STATE_PRESETS.Ready)
					: structuredClone(EMBEDDING_STATE_PRESETS.MissingModelOrRuntime)
				: structuredClone(EMBEDDING_STATE_PRESETS.Disabled);
		},
		async embeddingState() {
			return structuredClone(embedding);
		},
		async rebuildEmbeddings(consent) {
			if (!consent) throw new Error('Consent required');
			embedding = structuredClone(EMBEDDING_STATE_PRESETS.Ready);
		},
		async download(request) {
			if (!request.consent) throw new Error('Consent required');
			current = { ...current, model_cached: true };
			if (current.experimental_semantic_search) {
				embedding = structuredClone(EMBEDDING_STATE_PRESETS.Ready);
			}
		},
		async summarize() {
			return 'Local AI summary preview.';
		},
		async shellPreferences() {
			return { ...shellPrefs };
		},
		async saveShellPreferences(preferences) {
			shellPrefs = {
				close_to_tray: preferences.close_to_tray,
				minimize_to_tray: preferences.minimize_to_tray
			};
			return { ...shellPrefs };
		},
		async quitApp() {}
	};
}
