import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import Sidebar from './Sidebar.svelte';
import SearchToolbar from './SearchToolbar.svelte';
import InboxView from './InboxView.svelte';
import InboxSession from '../onboarding/InboxSession.svelte';
import { InboxController } from './controller.svelte';
import type { InboxTransport, MessageRowDto, SearchMode } from './types';
import type { Settings, SettingsTransport } from '$lib/settings/types';

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

function createMockInboxTransport(): InboxTransport {
	return {
		subscribe: vi.fn(async () => vi.fn()),
		search: vi.fn(async () => []),
		triage: vi.fn(async () => {})
	};
}

function createMockSettingsTransport(initialSettings?: Partial<Settings>): SettingsTransport & {
	currentSettings: Settings;
} {
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
		...initialSettings
	};

	return {
		currentSettings,
		get: vi.fn(async () => ({ ...currentSettings })),
		save: vi.fn(async (ai, key) => {
			currentSettings.ai = { ...ai };
			currentSettings.has_api_key = Boolean(key);
		}),
		tokenizer: vi.fn(async (t) => {
			currentSettings.tokenizer = t;
		}),
		setExperimentalSemanticSearch: vi.fn(async (enabled) => {
			currentSettings.experimental_semantic_search = enabled;
		}),
		download: vi.fn(async () => {
			currentSettings.model_cached = true;
		}),
		summarize: vi.fn(async () => 'Mock summary')
	};
}

describe('Settings shell integration and persistent inbox', () => {
	let cleanupFns: Array<() => void> = [];

	afterEach(() => {
		for (const cleanup of cleanupFns) {
			cleanup();
		}
		cleanupFns = [];
		document.body.innerHTML = '';
	});

	it('renders Settings gear button in Sidebar footer when openSettings is provided', () => {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const openSettings = vi.fn();

		const instance = mount(Sidebar, {
			target,
			props: {
				status: 'Idle',
				openSettings
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		const settingsBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]');
		expect(settingsBtn).not.toBeNull();
		expect(settingsBtn?.getAttribute('aria-haspopup')).toBe('dialog');
		expect(settingsBtn?.textContent).toContain('Settings');

		settingsBtn?.click();
		expect(openSettings).toHaveBeenCalledTimes(1);
	});

	it('renders Settings gear icon button in SearchToolbar', () => {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const openSettings = vi.fn();
		const toggleSidebar = vi.fn();
		const refresh = vi.fn();

		const instance = mount(SearchToolbar, {
			target,
			props: {
				sidebarOpen: false,
				toggleSidebar,
				refresh,
				openSettings
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		const settingsBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]');
		expect(settingsBtn).not.toBeNull();
		expect(settingsBtn?.getAttribute('aria-haspopup')).toBe('dialog');

		settingsBtn?.click();
		expect(openSettings).toHaveBeenCalledTimes(1);
	});

	it('suspends inbox keyboard shortcuts when settingsOpen is true', async () => {
		const transport = createMockInboxTransport();
		const inbox = new InboxController(transport);
		const sampleRow: MessageRowDto = {
			id: 'msg-1',
			account_id: 'acc-1',
			folder: 'INBOX',
			uid: 1,
			subject: 'Test Subject',
			sender: 'sender@example.com',
			received_at_epoch_ms: 1000,
			is_read: false,
			has_attachments: false,
			category_badge: null,
			tags: [],
			folder_override: null,
			triage_badges: []
		};
		inbox.rows = [sampleRow];
		inbox.selectedId = 'msg-1';

		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(InboxView, {
			target,
			props: {
				inbox,
				connected: true,
				settingsOpen: true
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		// Trigger keyboard shortcuts: 'r' to mark read, 'e' to archive
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'r' }));
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'e' }));
		flushSync();

		// Because settingsOpen is true, shortcuts must be suspended and mutate must not be called
		expect(transport.triage).not.toHaveBeenCalled();

		// Opening command palette should also be suppressed
		const paletteBtn = target.querySelector<HTMLButtonElement>('button[aria-keyshortcuts="Meta+K Control+K"]');
		paletteBtn?.click();
		flushSync();
		expect(target.querySelector('[role="combobox"]')).toBeNull();
	});

	it('mounts InboxSession with persistent inbox and renders SettingsDialog as sibling overlay', async () => {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const settingsTransport = createMockSettingsTransport({
			experimental_semantic_search: false
		});

		const instance = mount(InboxSession, {
			target,
			props: {
				connected: true,
				settingsTransport
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		// Initially, settings dialog is not open
		expect(target.querySelector('[role="dialog"]')).toBeNull();
		expect(target.querySelector('main')).not.toBeNull();

		// Click settings gear button in sidebar
		const settingsBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]');
		expect(settingsBtn).not.toBeNull();
		settingsBtn?.click();
		flushSync();

		// Await microtasks for SettingsDialog load
		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		// SettingsDialog is mounted as sibling overlay
		const dialog = target.querySelector('[role="dialog"]');
		expect(dialog).not.toBeNull();
		expect(dialog?.getAttribute('aria-modal')).toBe('true');

		// The main inbox is still mounted in the background!
		expect(target.querySelector('main')).not.toBeNull();

		// Close dialog via close button
		const closeBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Close settings"]');
		closeBtn?.click();
		flushSync();

		// Dialog closes and main inbox is still intact
		expect(target.querySelector('[role="dialog"]')).toBeNull();
		expect(target.querySelector('main')).not.toBeNull();
	});
});
