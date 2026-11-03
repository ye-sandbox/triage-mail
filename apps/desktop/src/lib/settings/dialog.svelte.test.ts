import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SettingsDialog from './SettingsDialog.svelte';
import GeneralModule from './modules/GeneralModule.svelte';
import SearchModule from './modules/SearchModule.svelte';
import AiModule from './modules/AiModule.svelte';
import PrivacyModule from './modules/PrivacyModule.svelte';
import type { Settings, SettingsTab, SettingsTransport, Tokenizer } from './types';
import { SETTINGS_TABS } from './types';

function createMockTransport(overrides?: Partial<Settings>) {
	const currentSettings: Settings = {
		ai: {
			enabled: false,
			provider: 'Ollama',
			endpoint: 'http://127.0.0.1:11434',
			model: 'llama3'
		},
		tokenizer: 'Unicode61RemoveDiacritics',
		has_api_key: false,
		model_cached: false,
		experimental_semantic_search: false,
		...overrides
	};

	let failGet = false;

	const transport: SettingsTransport & {
		getCalls: number;
		saveCalls: Array<{ settings: Settings['ai']; apiKey: string }>;
		tokenizerCalls: Tokenizer[];
		semanticCalls: boolean[];
		downloadCalls: Array<{
			model_url: string;
			tokenizer_url: string;
			consent: boolean;
		}>;
		setFailGet: (fail: boolean) => void;
	} = {
		getCalls: 0,
		saveCalls: [],
		tokenizerCalls: [],
		semanticCalls: [],
		downloadCalls: [],
		setFailGet(fail: boolean) {
			failGet = fail;
		},
		async get() {
			transport.getCalls++;
			if (failGet) {
				throw new Error('Failed to load settings');
			}
			return { ...currentSettings };
		},
		async save(aiSettings, apiKey) {
			transport.saveCalls.push({ settings: aiSettings, apiKey });
			currentSettings.ai = { ...aiSettings };
			currentSettings.has_api_key = Boolean(apiKey);
		},
		async tokenizer(tok) {
			transport.tokenizerCalls.push(tok);
			currentSettings.tokenizer = tok;
		},
		async setExperimentalSemanticSearch(enabled) {
			transport.semanticCalls.push(enabled);
			currentSettings.experimental_semantic_search = enabled;
		},
		async download(req) {
			transport.downloadCalls.push(req);
			currentSettings.model_cached = true;
		},
		async summarize() {
			return 'Mock summary';
		}
	};

	return transport;
}

async function flushMicrotasks() {
	await Promise.resolve();
	await Promise.resolve();
	flushSync();
}

describe('SettingsDialog and typed modules', () => {
	let cleanupFns: Array<() => void> = [];

	afterEach(() => {
		for (const cleanup of cleanupFns) {
			cleanup();
		}
		cleanupFns = [];
		document.body.innerHTML = '';
	});

	function mountDialog(props: {
		open?: boolean;
		close: () => void;
		initialTab?: SettingsTab;
		transport?: SettingsTransport;
	}) {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const instance = mount(SettingsDialog, {
			target,
			props
		});
		flushSync();
		const destroy = () => {
			unmount(instance);
			target.remove();
		};
		cleanupFns.push(destroy);
		return { target, destroy };
	}

	it('exports expected SETTINGS_TABS definitions', () => {
		expect(SETTINGS_TABS.map((t) => t.id)).toEqual([
			'general',
			'accounts',
			'search',
			'ai',
			'privacy',
			'rules',
			'alerts',
			'reminders',
			'categories'
		]);
	});

	it('does not render dialog content when open is false', () => {
		const close = vi.fn();
		const transport = createMockTransport();
		const { target } = mountDialog({ open: false, close, transport });

		expect(target.querySelector('[role="dialog"]')).toBeNull();
		expect(transport.getCalls).toBe(0);
	});

	it('renders dialog and loads settings when open is true', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		const { target } = mountDialog({ open: true, close, transport });

		await flushMicrotasks();

		const dialog = target.querySelector('[role="dialog"]');
		expect(dialog).not.toBeNull();
		expect(dialog?.getAttribute('aria-modal')).toBe('true');
		expect(transport.getCalls).toBe(1);

		// Default initial tab is general
		const generalTab = target.querySelector('#tab-general');
		expect(generalTab?.getAttribute('aria-selected')).toBe('true');
		expect(target.querySelector('[aria-label="General settings"]')).not.toBeNull();
	});

	it('navigates between modules on tab click', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		const { target } = mountDialog({ open: true, close, transport });

		await flushMicrotasks();

		// Switch to Search tab
		const searchTab = target.querySelector<HTMLButtonElement>('#tab-search');
		expect(searchTab).not.toBeNull();
		searchTab?.click();
		flushSync();

		expect(searchTab?.getAttribute('aria-selected')).toBe('true');
		expect(target.querySelector('[aria-label="Search settings"]')).not.toBeNull();

		// Switch to AI tab
		const aiTab = target.querySelector<HTMLButtonElement>('#tab-ai');
		expect(aiTab).not.toBeNull();
		aiTab?.click();
		flushSync();

		expect(aiTab?.getAttribute('aria-selected')).toBe('true');
		expect(target.querySelector('[aria-label="AI settings module"]')).not.toBeNull();

		// Switch to Privacy tab
		const privacyTab = target.querySelector<HTMLButtonElement>('#tab-privacy');
		expect(privacyTab).not.toBeNull();
		privacyTab?.click();
		flushSync();

		expect(privacyTab?.getAttribute('aria-selected')).toBe('true');
		expect(target.querySelector('[aria-label="Privacy settings"]')).not.toBeNull();
	});

	it('respects custom initialTab prop', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		const { target } = mountDialog({
			open: true,
			close,
			initialTab: 'search',
			transport
		});

		await flushMicrotasks();

		const searchTab = target.querySelector('#tab-search');
		expect(searchTab?.getAttribute('aria-selected')).toBe('true');
		expect(target.querySelector('[aria-label="Search settings"]')).not.toBeNull();
	});

	it('triggers close when clicking the close button', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		const { target } = mountDialog({ open: true, close, transport });

		await flushMicrotasks();

		const closeButton = target.querySelector<HTMLButtonElement>(
			'button[aria-label="Close settings"]'
		);
		expect(closeButton).not.toBeNull();
		closeButton?.click();

		expect(close).toHaveBeenCalledTimes(1);
	});

	it('triggers close on Escape keydown', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		mountDialog({ open: true, close, transport });

		await flushMicrotasks();

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
		expect(close).toHaveBeenCalledTimes(1);
	});

	it('triggers close on backdrop presentation click', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		const { target } = mountDialog({ open: true, close, transport });

		await flushMicrotasks();

		const backdrop = target.querySelector<HTMLDivElement>('div[role="presentation"]');
		expect(backdrop).not.toBeNull();
		backdrop?.dispatchEvent(new MouseEvent('click', { bubbles: true }));

		expect(close).toHaveBeenCalledTimes(1);
	});

	it('shows error state and handles retry when settings load fails', async () => {
		const close = vi.fn();
		const transport = createMockTransport();
		transport.setFailGet(true);

		const { target } = mountDialog({ open: true, close, transport });
		await flushMicrotasks();

		const alert = target.querySelector('[role="alert"]');
		expect(alert).not.toBeNull();
		expect(alert?.textContent).toContain('Settings could not be loaded.');

		// Now allow loading and click retry
		transport.setFailGet(false);
		const retryButton = target.querySelector<HTMLButtonElement>(
			'button[aria-label="Retry loading settings"]'
		);
		expect(retryButton).not.toBeNull();
		retryButton?.click();
		await flushMicrotasks();

		expect(target.querySelector('[role="alert"]')).toBeNull();
		expect(target.querySelector('[aria-label="General settings"]')).not.toBeNull();
	});

	describe('SearchModule standalone tests', () => {
		it('toggles experimental semantic search and updates tokenizer', async () => {
			const transport = createMockTransport({
				experimental_semantic_search: false,
				tokenizer: 'Unicode61RemoveDiacritics'
			});
			const onReload = vi.fn();

			const target = document.createElement('div');
			document.body.appendChild(target);
			const instance = mount(SearchModule, {
				target,
				props: {
					settings: {
						ai: {
							enabled: false,
							provider: 'Ollama',
							endpoint: 'http://127.0.0.1:11434',
							model: 'llama3'
						},
						tokenizer: 'Unicode61RemoveDiacritics',
						has_api_key: false,
						model_cached: false,
						experimental_semantic_search: false
					},
					transport,
					onReload
				}
			});
			flushSync();
			cleanupFns.push(() => {
				unmount(instance);
				target.remove();
			});

			const checkbox = target.querySelector<HTMLInputElement>('input[type="checkbox"]');
			expect(checkbox).not.toBeNull();
			expect(checkbox?.checked).toBe(false);

			checkbox?.click();
			await flushMicrotasks();

			expect(transport.semanticCalls).toEqual([true]);
			expect(onReload).toHaveBeenCalled();

			const select = target.querySelector<HTMLSelectElement>('select[name="tokenizer"]');
			expect(select).not.toBeNull();
			select!.value = 'Trigram';

			const form = target.querySelector<HTMLFormElement>('form');
			form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
			await flushMicrotasks();

			expect(transport.tokenizerCalls).toEqual(['Trigram']);
		});
	});

	describe('GeneralModule standalone tests', () => {
		it('renders language selector and app summary', () => {
			const transport = createMockTransport();
			const target = document.createElement('div');
			document.body.appendChild(target);
			const instance = mount(GeneralModule, {
				target,
				props: {
					settings: {
						ai: {
							enabled: false,
							provider: 'Ollama',
							endpoint: 'http://127.0.0.1:11434',
							model: 'llama3'
						},
						tokenizer: 'Unicode61RemoveDiacritics',
						has_api_key: false,
						model_cached: false,
						experimental_semantic_search: false
					},
					transport
				}
			});
			flushSync();
			cleanupFns.push(() => {
				unmount(instance);
				target.remove();
			});

			expect(target.textContent).toContain('Interface language');
			expect(target.textContent).toContain('About triage-mail');
		});
	});

	describe('PrivacyModule standalone tests', () => {
		it('renders privacy and telemetry guarantees', () => {
			const target = document.createElement('div');
			document.body.appendChild(target);
			const instance = mount(PrivacyModule, {
				target,
				props: {}
			});
			flushSync();
			cleanupFns.push(() => {
				unmount(instance);
				target.remove();
			});

			expect(target.textContent).toContain('Remote Images & Tracking Pixels');
			expect(target.textContent).toContain('Zero Telemetry & Network Isolation');
			expect(target.textContent).toContain('Local Vault & Database Encryption');
		});
	});

	describe('AiModule standalone tests', () => {
		it('renders AI summary form and model settings', () => {
			const transport = createMockTransport();
			const target = document.createElement('div');
			document.body.appendChild(target);
			const instance = mount(AiModule, {
				target,
				props: {
					settings: {
						ai: {
							enabled: false,
							provider: 'Ollama',
							endpoint: 'http://127.0.0.1:11434',
							model: 'llama3'
						},
						tokenizer: 'Unicode61RemoveDiacritics',
						has_api_key: false,
						model_cached: false,
						experimental_semantic_search: false
					},
					transport
				}
			});
			flushSync();
			cleanupFns.push(() => {
				unmount(instance);
				target.remove();
			});

			expect(target.querySelector('form[aria-label="AI settings"]')).not.toBeNull();
			expect(target.querySelector('section[aria-label="Local model files"]')).not.toBeNull();
		});
	});
});
