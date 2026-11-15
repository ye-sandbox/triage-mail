import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import InboxSession from '../onboarding/InboxSession.svelte';
import WdgInboxStatsBar from './WdgInboxStatsBar.svelte';
import { InboxController } from './controller.svelte';
import { createLiveWorkspace, workspaceRegistry } from './workspace.svelte';
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

describe('UI v2 Settings route navigation and persistent inbox shell', () => {
	let cleanupFns: Array<() => void> = [];

	afterEach(() => {
		for (const cleanup of cleanupFns) {
			cleanup();
		}
		cleanupFns = [];
		document.body.innerHTML = '';
		appStore.setScope({ kind: 'inbox' });
		appStore.consumePendingRuleSeedMessage();
	});

	it('renders Settings route trigger in WdgInboxStatsBar without modal attributes', () => {
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
		expect(settingsBtn?.hasAttribute('aria-haspopup')).toBe(false);
		expect(settingsBtn?.textContent).toContain('Settings');

		settingsBtn?.click();
		expect(openSettings).toHaveBeenCalledTimes(1);
	});

	it('navigates to /settings and /automation (with rule seed) while syncing settings updates in InboxSession', async () => {
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
		const settingsTransport = createMockSettingsTransport({
			experimental_semantic_search: false
		});
		const onNavigate = vi.fn();
		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(InboxSession, {
			target,
			props: {
				connected: true,
				inboxTransport,
				settingsTransport,
				onNavigate
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		await vi.waitFor(() => expect(target.querySelector('[role="option"]')).not.toBeNull());
		expect(target.querySelector('[role="dialog"]')).toBeNull();
		const listbox = target.querySelector('[role="listbox"][aria-label="Messages"]');
		expect(listbox).not.toBeNull();

		target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]')?.click();
		flushSync();
		expect(onNavigate).toHaveBeenCalledWith('/settings');
		expect(target.querySelector('[role="dialog"]')).toBeNull();

		target.querySelector<HTMLElement>('[role="option"]')?.click();
		flushSync();
		target.querySelector<HTMLButtonElement>('button[title="Create Rule"]')?.click();
		flushSync();

		expect(appStore.consumePendingRuleSeedMessage()).toBe('msg-1');
		expect(onNavigate).toHaveBeenCalledWith('/automation');

		settingsTransport.currentSettings.experimental_semantic_search = true;
		window.dispatchEvent(new Event('triage-settings-updated'));
		await vi.waitFor(() =>
			expect(workspaceRegistry.current?.inbox.experimentalSemanticSearch).toBe(true)
		);
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
