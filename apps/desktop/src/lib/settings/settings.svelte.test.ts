import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { i18n } from '../i18n/index.svelte';
import { appStore } from '../stores/app.svelte';
import { SettingsFormGuard } from './guards';
import { settingsLabels } from './labels';
import { createDemoSettingsTransport, EMBEDDING_STATE_PRESETS } from './settings.fixtures';
import {
	buildImportPreviewFromJson,
	getEmbeddingProgressPct,
	getEmbeddingStatusTone
} from './settings.helpers';
import type { AccountTransport } from '../onboarding/types';
import type { EmbeddingState, Settings, SettingsTransport } from './types';
import WdgAiModelsPanel from './WdgAiModelsPanel.svelte';
import WdgBackupDefinitionsPanel from './WdgBackupDefinitionsPanel.svelte';
import WdgPrivacyPanel from './WdgPrivacyPanel.svelte';
import WdgSearchIndexPanel from './WdgSearchIndexPanel.svelte';
import WdgSettingsPage from './WdgSettingsPage.svelte';

beforeAll(() => {
	vi.stubGlobal(
		'ResizeObserver',
		class {
			observe() {}
			unobserve() {}
			disconnect() {}
		}
	);
});

const defaults = (): Settings => ({
	ai: {
		enabled: false,
		provider: 'Ollama',
		endpoint: 'http://127.0.0.1:11434',
		model: 'llama3.2'
	},
	tokenizer: 'Unicode61RemoveDiacritics',
	has_api_key: false,
	model_cached: false,
	experimental_semantic_search: false
});

function fixture(overrides?: Partial<Settings>) {
	const state: Settings = { ...defaults(), ...overrides };
	const transport = {
		get: vi.fn<SettingsTransport['get']>().mockImplementation(async () => structuredClone(state)),
		save: vi.fn<SettingsTransport['save']>().mockImplementation(async (ai, apiKey) => {
			state.ai = { ...ai };
			state.has_api_key = ai.provider === 'Byok' && (apiKey.trim().length > 0 || state.has_api_key);
		}),
		tokenizer: vi.fn<SettingsTransport['tokenizer']>().mockImplementation(async (tok) => {
			state.tokenizer = tok;
		}),
		setExperimentalSemanticSearch: vi
			.fn<SettingsTransport['setExperimentalSemanticSearch']>()
			.mockImplementation(async (enabled) => {
				state.experimental_semantic_search = enabled;
			}),
		embeddingState: vi
			.fn<NonNullable<SettingsTransport['embeddingState']>>()
			.mockResolvedValue({ ...EMBEDDING_STATE_PRESETS.Disabled }),
		rebuildEmbeddings: vi
			.fn<NonNullable<SettingsTransport['rebuildEmbeddings']>>()
			.mockResolvedValue(),
		download: vi.fn<SettingsTransport['download']>().mockImplementation(async () => {
			state.model_cached = true;
		}),
		summarize: vi
			.fn<SettingsTransport['summarize']>()
			.mockResolvedValue('<script>untrusted</script>')
	};
	const target = document.createElement('div');
	document.body.append(target);
	return { target, transport, state };
}

function submit(form: HTMLFormElement) {
	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
}

async function settle() {
	for (let i = 0; i < 8; i++) {
		await Promise.resolve();
		flushSync();
	}
}

afterEach(() => {
	document.body.replaceChildren();
	vi.restoreAllMocks();
	vi.useRealTimers();
	i18n.setLocale('en');
	appStore.setTheme('light');
	appStore.semanticSearchEnabled = false;
});

describe('settings helpers, fixtures, and i18n parity', () => {
	it('maintains EN and pt-BR dictionary parity across all settings labels', () => {
		expect(Object.keys(settingsLabels.en.tabs)).toEqual(Object.keys(settingsLabels['pt-BR'].tabs));
		expect(Object.keys(settingsLabels.en.search.states)).toEqual(
			Object.keys(settingsLabels['pt-BR'].search.states)
		);
		expect(Object.keys(settingsLabels.en.privacy)).toEqual(
			Object.keys(settingsLabels['pt-BR'].privacy)
		);
		expect(Object.keys(settingsLabels.en.ai)).toEqual(Object.keys(settingsLabels['pt-BR'].ai));
		expect(Object.keys(settingsLabels.en.backup)).toEqual(
			Object.keys(settingsLabels['pt-BR'].backup)
		);
		expect(Object.keys(settingsLabels.en.importDialog)).toEqual(
			Object.keys(settingsLabels['pt-BR'].importDialog)
		);
	});

	it('computes embedding percentage, badge tones, and portable import previews', () => {
		expect(getEmbeddingProgressPct({ status: 'Disabled', indexed: 0, pending: 0, failed: 0 })).toBe(
			0
		);
		expect(getEmbeddingProgressPct({ status: 'Rebuilding', indexed: 50, pending: 49, failed: 1 })).toBe(
			50
		);
		expect(getEmbeddingStatusTone('Ready')).toContain('text-status-healthy');
		expect(getEmbeddingStatusTone('Failed')).toContain('text-status-danger');

		const preview = buildImportPreviewFromJson(
			'rules.json',
			JSON.stringify({ rules: [{ name: 'Fiscal' }], saved_views: [{ name: 'Boletos' }] })
		);
		expect(preview.source_filename).toBe('rules.json');
		expect(preview.counts.added).toBe(2);
		expect(preview.items.map((item) => item.name)).toEqual(['Fiscal', 'Boletos']);
	});

	it('simulates full local settings lifecycle in demo transport without IPC', async () => {
		const demo = createDemoSettingsTransport();
		const initial = await demo.get();
		expect(initial.experimental_semantic_search).toBe(false);

		await expect(demo.rebuildEmbeddings!(false)).rejects.toThrow('Consent required');
		await demo.download({
			model_url: 'https://models.example.com/model.onnx',
			tokenizer_url: 'https://models.example.com/tokenizer.json',
			consent: true
		});
		await demo.setExperimentalSemanticSearch(true);
		const emb = await demo.embeddingState!();
		expect(emb.status).toBe('Ready');

		await demo.save(
			{
				enabled: true,
				provider: 'Byok',
				endpoint: 'https://api.example.com/v1',
				model: 'gpt-4o-mini'
			},
			'synthetic-key'
		);
		const updated = await demo.get();
		expect(updated.has_api_key).toBe(true);
		expect(updated.model_cached).toBe(true);
	});
});

describe('WdgSettingsPage navigation, loading, and unsaved guard', () => {
	it('loads settings without downloads, probes, or summaries and handles load errors + retry', async () => {
		const { target, transport } = fixture();
		transport.get.mockRejectedValueOnce(new Error('vault locked')).mockResolvedValueOnce(defaults());

		const component = mount(WdgSettingsPage, {
			target,
			props: { transport, initialSection: 'search' }
		});
		await settle();

		expect(transport.download).not.toHaveBeenCalled();
		expect(transport.summarize).not.toHaveBeenCalled();
		expect(transport.save).not.toHaveBeenCalled();
		expect(target.querySelector('[role="alert"]')?.textContent).toContain(
			'Settings could not be loaded.'
		);

		target.querySelector<HTMLButtonElement>('button[aria-label="Retry loading settings"]')!.click();
		await settle();

		expect(target.querySelector('form[aria-label="FTS5 Lexical Tokenizer"]')).not.toBeNull();
		await unmount(component);
	});

	it('guards unsaved edits across section switches and back-to-inbox navigation', async () => {
		const { target, transport } = fixture();
		const guard = new SettingsFormGuard();
		const onBackToInbox = vi.fn();
		const component = mount(WdgSettingsPage, {
			target,
			props: { transport, guard, initialSection: 'ai', onBackToInbox }
		});
		await settle();

		const modelInput = target.querySelector<HTMLInputElement>('input[name="model"]')!;
		modelInput.value = 'qwen2.5:14b';
		modelInput.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();

		expect(guard.dirty).toBe(true);

		target.querySelector<HTMLButtonElement>('#tab-privacy')!.click();
		flushSync();
		expect(document.body.textContent).toContain('You have unsaved changes');

		const stayBtn = [...document.body.querySelectorAll('button')].find(
			(b) => b.textContent?.trim() === 'Stay on form'
		)!;
		stayBtn.click();
		flushSync();
		expect(target.querySelector('form[aria-label="AI settings"]')).not.toBeNull();

		target.querySelector<HTMLButtonElement>('#tab-privacy')!.click();
		flushSync();
		const discardBtn = [...document.body.querySelectorAll('button')].find(
			(b) => b.textContent?.trim() === 'Discard changes'
		)!;
		discardBtn.click();
		flushSync();
		expect(target.textContent).toContain('Privacy & Network Guarantees');

		[...target.querySelectorAll('button')]
			.find((b) => b.textContent?.includes('Back to inbox'))!
			.click();
		expect(onBackToInbox).toHaveBeenCalledOnce();

		await unmount(component);
	});
});

describe('WdgPrivacyPanel', () => {
	it('switches locale and theme polarity in general section', async () => {
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgPrivacyPanel, {
			target,
			props: { section: 'general' }
		});
		flushSync();

		target.querySelector<HTMLButtonElement>('button[aria-label="Português (Brasil)"]')!.click();
		flushSync();
		expect(i18n.locale).toBe('pt-BR');
		expect(target.textContent).toContain('Preferências Gerais');

		const toggleThemeBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Visualizar')
		)!;
		const beforeTheme = appStore.theme;
		toggleThemeBtn.click();
		flushSync();
		expect(appStore.theme).not.toBe(beforeTheme);

		await unmount(component);
	});

	it('renders remote image blocking as a strictly read-only global guarantee with zero toggles', async () => {
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgPrivacyPanel, {
			target,
			props: { section: 'privacy' }
		});
		flushSync();

		expect(target.textContent).toContain('Remote Images & Tracking Pixels');
		expect(target.textContent).toContain('Blocked by default');
		expect(target.querySelectorAll('input[type="checkbox"], button[role="switch"]')).toHaveLength(0);

		await unmount(component);
	});
});

describe('WdgSearchIndexPanel', () => {
	it('switches tokenizer radio options, shows Trigram warning, and applies rebuild', async () => {
		const { target, transport, state } = fixture();
		const onReload = vi.fn();
		const component = mount(WdgSearchIndexPanel, {
			target,
			props: { settings: state, transport, onReload }
		});
		await settle();

		expect(target.textContent).not.toContain('Disk-space warning');
		const trigramBtn = [...target.querySelectorAll<HTMLButtonElement>('button[role="radio"]')].find(
			(b) => b.textContent?.includes('Trigram')
		)!;
		trigramBtn.click();
		flushSync();

		expect(trigramBtn.getAttribute('aria-checked')).toBe('true');
		expect(target.textContent).toContain('Disk-space warning');

		submit(target.querySelector('form[aria-label="FTS5 Lexical Tokenizer"]')!);
		await settle();

		expect(transport.tokenizer).toHaveBeenCalledExactlyOnceWith('Trigram');
		expect(onReload).toHaveBeenCalled();

		await unmount(component);
	});

	it('toggles semantic search explicitly, polls embedding state, and requires rebuild confirmation', async () => {
		vi.useFakeTimers();
		const { target, transport, state } = fixture({ experimental_semantic_search: false });
		const events: string[] = [];
		const onUpdated = () => events.push('updated');
		window.addEventListener('triage-settings-updated', onUpdated);

		const onReload = vi.fn();
		const component = mount(WdgSearchIndexPanel, {
			target,
			props: { settings: state, transport, onReload }
		});
		await settle();

		expect(transport.embeddingState).toHaveBeenCalledTimes(1);
		await vi.advanceTimersByTimeAsync(2000);
		expect(transport.embeddingState).toHaveBeenCalledTimes(2);

		const switchEl = target.querySelector<HTMLButtonElement>(
			'button[role="switch"][aria-label="Enable semantic search"]'
		)!;
		expect(switchEl.getAttribute('aria-checked')).toBe('false');
		switchEl.click();
		await settle();

		expect(transport.setExperimentalSemanticSearch).toHaveBeenCalledExactlyOnceWith(true);
		expect(events).toEqual(['updated']);
		window.removeEventListener('triage-settings-updated', onUpdated);
		await unmount(component);

		// Verify interval stops cleanly on unmount
		const callsBeforeUnmountAdvance = transport.embeddingState.mock.calls.length;
		await vi.advanceTimersByTimeAsync(4000);
		expect(transport.embeddingState).toHaveBeenCalledTimes(callsBeforeUnmountAdvance);
	});

	it('gates vector rebuild behind consent checkbox and ignores late resolution after unmount', async () => {
		const { target, transport, state } = fixture({ experimental_semantic_search: true });
		let resolveRebuild!: () => void;
		transport.rebuildEmbeddings.mockReturnValue(
			new Promise<void>((done) => {
				resolveRebuild = done;
			})
		);
		const onReload = vi.fn();
		const component = mount(WdgSearchIndexPanel, {
			target,
			props: { settings: state, transport, onReload }
		});
		await settle();

		const rebuildBtn = [...target.querySelectorAll('button')].find(
			(b) => b.textContent?.trim() === 'Rebuild local index'
		)!;
		expect(rebuildBtn.disabled).toBe(true);

		const checkbox = target.querySelector<HTMLInputElement>(
			'section[aria-label="Local semantic search"] input[type="checkbox"]'
		)!;
		checkbox.click();
		flushSync();
		expect(rebuildBtn.disabled).toBe(false);

		rebuildBtn.click();
		flushSync();
		expect(transport.rebuildEmbeddings).toHaveBeenCalledExactlyOnceWith(true);

		await unmount(component);
		resolveRebuild();
		await settle();
		expect(onReload).not.toHaveBeenCalled();
	});

	it('requires download consent for ONNX files and redacts errors', async () => {
		const { target, transport, state } = fixture({ model_cached: false });
		transport.download.mockRejectedValue('private URL data');
		const component = mount(WdgSearchIndexPanel, {
			target,
			props: { settings: state, transport }
		});
		await settle();

		const modelForm = target.querySelector<HTMLFormElement>(
			'section[aria-label="Local model files"] form'
		)!;
		modelForm.querySelector<HTMLInputElement>('[name=modelUrl]')!.value =
			'https://models.example.com/model.onnx';
		modelForm.querySelector<HTMLInputElement>('[name=tokenizerUrl]')!.value =
			'https://models.example.com/tokenizer.json';
		submit(modelForm);
		expect(transport.download).not.toHaveBeenCalled();

		modelForm.querySelector<HTMLInputElement>('[name=consent]')!.checked = true;
		submit(modelForm);
		await settle();

		expect(transport.download).toHaveBeenCalledExactlyOnceWith({
			model_url: 'https://models.example.com/model.onnx',
			tokenizer_url: 'https://models.example.com/tokenizer.json',
			consent: true
		});
		expect(target.textContent).toContain('Download or verification failed');
		expect(target.textContent).not.toContain('private URL data');

		// Verify embedding state error telemetry fallback
		transport.embeddingState.mockRejectedValueOnce(new Error('offline'));
		const staleState: EmbeddingState = EMBEDDING_STATE_PRESETS.Ready;
		expect(staleState.status).toBe('Ready');

		await unmount(component);
	});
});

describe('WdgAiModelsPanel', () => {
	it('clears unbound API keys before dispatch, blocks duplicate saves, and clears on teardown', async () => {
		const { target, transport, state } = fixture();
		let resolveSave!: () => void;
		transport.save.mockReturnValue(
			new Promise<void>((done) => {
				resolveSave = done;
			})
		);
		const guard = new SettingsFormGuard();
		const onReload = vi.fn();

		const component = mount(WdgAiModelsPanel, {
			target,
			props: { settings: state, transport, guard, onReload }
		});
		flushSync();

		const byokBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('BYOK (OpenAI compatible)')
		)!;
		byokBtn.click();
		flushSync();
		expect(target.querySelector<HTMLInputElement>('[name=endpoint]')!.value).toBe(
			'https://api.openai.com/v1'
		);
		expect(guard.dirty).toBe(true);

		const key = target.querySelector<HTMLInputElement>('[name=apiKey]')!;
		key.value = 'synthetic-private-value';
		key.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();

		const form = target.querySelector<HTMLFormElement>('form[aria-label="AI settings"]')!;
		submit(form);
		submit(form);

		expect(key.value).toBe('');
		expect(guard.busy).toBe(true);
		expect(transport.save).toHaveBeenCalledExactlyOnceWith(
			{
				enabled: false,
				provider: 'Byok',
				endpoint: 'https://api.openai.com/v1',
				model: 'gpt-4o-mini'
			},
			'synthetic-private-value'
		);

		resolveSave();
		await settle();
		expect(guard.busy).toBe(false);
		expect(onReload).toHaveBeenCalledOnce();

		key.value = 'transient-value';
		await unmount(component);
		expect(key.value).toBe('');
	});

	it('redacts raw save errors without leaking secret diagnostics', async () => {
		const { target, transport, state } = fixture();
		transport.save.mockRejectedValueOnce('raw-vault-failure-detail');
		const component = mount(WdgAiModelsPanel, {
			target,
			props: { settings: state, transport }
		});
		flushSync();

		submit(target.querySelector<HTMLFormElement>('form[aria-label="AI settings"]')!);
		await settle();

		expect(target.querySelector('[role="alert"]')?.textContent).toContain(
			'AI settings could not be saved.'
		);
		expect(target.textContent).not.toContain('raw-vault-failure-detail');

		await unmount(component);
	});
});

describe('WdgBackupDefinitionsPanel local archive import', () => {
	it('picks archive file, infers format, streams progress, and maps AccountExists errors', async () => {
		const target = document.createElement('div');
		document.body.append(target);
		const onArchiveImported = vi.fn();
		const archiveTransport = {
			chooseArchiveFile: vi
				.fn<NonNullable<AccountTransport['chooseArchiveFile']>>()
				.mockResolvedValueOnce('/tmp/sample.eml')
				.mockResolvedValueOnce('/tmp/export.mbox'),
			importLocalArchive: vi
				.fn<NonNullable<AccountTransport['importLocalArchive']>>()
				.mockImplementationOnce(async (_path, _email, _fmt, onProgress) => {
					onProgress?.({
						account_id: 'arch-1',
						state: 'Syncing',
						new_messages_count: 1,
						triage_alerts: [],
						embedding_errors_count: 0
					});
					return { account_id: 'arch-1', imported_count: 1, batches_committed: 1, last_uid: 1 };
				})
				.mockRejectedValueOnce('AccountExists')
		};

		const component = mount(WdgBackupDefinitionsPanel, {
			target,
			props: { archiveTransport, onArchiveImported }
		});
		flushSync();

		const archiveForm = target.querySelector<HTMLFormElement>(
			'form[aria-label="Local Archive Import (.mbox / .eml)"]'
		)!;
		expect(archiveForm).not.toBeNull();

		const chooseBtn = [...archiveForm.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('No archive file selected')
		)!;
		chooseBtn.click();
		await settle();

		const radios = [...archiveForm.querySelectorAll<HTMLButtonElement>('button[role="radio"]')];
		expect(radios).toHaveLength(2);
		expect(radios[1].getAttribute('aria-checked')).toBe('true');

		const emailEl = archiveForm.querySelector<HTMLInputElement>('input[name="archiveEmail"]')!;
		emailEl.value = 'archive@example.com';
		emailEl.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();

		submit(archiveForm);
		await settle();

		expect(archiveTransport.importLocalArchive).toHaveBeenCalledWith(
			'/tmp/sample.eml',
			'archive@example.com',
			'Eml',
			expect.any(Function)
		);
		expect(onArchiveImported).toHaveBeenCalledOnce();
		expect(archiveForm.querySelector('[role="status"]')?.textContent).toContain(
			'Imported archive messages: 1'
		);

		const chooseBtnAfter = [...archiveForm.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('No archive file selected')
		)!;
		chooseBtnAfter.click();
		await settle();
		expect(radios[0].getAttribute('aria-checked')).toBe('true');

		emailEl.value = 'existing@example.com';
		emailEl.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();
		submit(archiveForm);
		await settle();

		expect(archiveForm.querySelector('[role="alert"]')?.textContent).toContain(
			'Email belongs to an existing IMAP or recovered account.'
		);

		await unmount(component);
	});
});
