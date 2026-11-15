import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import InboxSession from '../onboarding/InboxSession.svelte';
import WdgInboxStatsBar from './WdgInboxStatsBar.svelte';
import { InboxController } from './controller.svelte';
import { createLiveWorkspace } from './workspace.svelte';
import type { InboxTransport, MessageRowDto } from './types';
import type { Settings, SettingsTransport } from '$lib/settings/types';
import { appStore } from '$lib/stores/app.svelte';

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

function createMockInboxTransport(rows: MessageRowDto[] = []): InboxTransport {
	return {
		subscribe: vi.fn(async () => vi.fn()),
		search: vi.fn(async () => rows),
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
		tokenizer: vi.fn(async (tok) => {
			currentSettings.tokenizer = tok;
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

describe('UI v2 Settings shell integration and persistent inbox', () => {
	let cleanupFns: Array<() => void> = [];

	afterEach(() => {
		for (const cleanup of cleanupFns) {
			cleanup();
		}
		cleanupFns = [];
		document.body.innerHTML = '';
		appStore.setScope({ kind: 'inbox' });
	});

	it('renders Settings trigger button in WdgInboxStatsBar when openSettings is provided', () => {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const openSettings = vi.fn();
		const workspace = createLiveWorkspace(new InboxController(createMockInboxTransport()), {
			connected: true
		});

		const instance = mount(WdgInboxStatsBar, {
			target,
			props: {
				workspace,
				openSettings
			}
		});
		flushSync();
		cleanupFns.push(() => {
			workspace.inbox.dispose();
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

	it('suspends inbox keyboard shortcuts while SettingsDialog is open in InboxSession', async () => {
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
		const inboxTransport = createMockInboxTransport([sampleRow]);
		const settingsTransport = createMockSettingsTransport();
		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(InboxSession, {
			target,
			props: {
				connected: true,
				inboxTransport,
				settingsTransport
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		await vi.waitFor(() => expect(target.querySelector('[role="option"]')).not.toBeNull());
		target.querySelector<HTMLElement>('[role="option"]')?.click();
		flushSync();

		target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]')?.click();
		await Promise.resolve();
		flushSync();
		expect(target.querySelector('[role="dialog"]')).not.toBeNull();

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'r', bubbles: true }));
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'e', bubbles: true }));
		flushSync();

		expect(inboxTransport.triage).not.toHaveBeenCalled();
	});

	it('mounts InboxSession with persistent listbox and renders SettingsDialog as sibling overlay', async () => {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const inboxTransport = createMockInboxTransport([]);
		const settingsTransport = createMockSettingsTransport({
			experimental_semantic_search: false
		});

		const instance = mount(InboxSession, {
			target,
			props: {
				connected: true,
				inboxTransport,
				settingsTransport
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		expect(target.querySelector('[role="dialog"]')).toBeNull();
		const listbox = target.querySelector('[role="listbox"][aria-label="Messages"]');
		expect(listbox).not.toBeNull();

		const settingsBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]');
		expect(settingsBtn).not.toBeNull();
		settingsBtn?.click();
		flushSync();

		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		const dialog = target.querySelector('[role="dialog"]');
		expect(dialog).not.toBeNull();
		expect(dialog?.getAttribute('aria-modal')).toBe('true');
		expect(target.querySelector('[role="listbox"][aria-label="Messages"]')).toBe(listbox);

		const closeBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Close settings"]');
		closeBtn?.click();
		flushSync();

		expect(target.querySelector('[role="dialog"]')).toBeNull();
		expect(target.querySelector('[role="listbox"][aria-label="Messages"]')).toBe(listbox);
	});

	it('activates fixture demo workspace cleanly when connected is false (outside Tauri)', async () => {
		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(InboxSession, {
			target,
			props: {
				connected: false
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		await vi.waitFor(() => expect(target.querySelectorAll('[role="option"]').length).toBeGreaterThan(0));
		expect(target.textContent).toContain('Your verification code');
	});
});
