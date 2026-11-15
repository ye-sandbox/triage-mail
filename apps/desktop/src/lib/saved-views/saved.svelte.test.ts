import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeAll, expect, it, vi } from 'vitest';
import { i18n } from '../i18n/index.svelte';
import { InboxController, PAGE_SIZE } from '../inbox/controller.svelte';
import type { InboxTransport, MessageRowDto } from '../inbox/types';
import { createLiveWorkspace } from '../inbox/workspace.svelte';
import InboxSession from '../onboarding/InboxSession.svelte';
import type { SettingsTransport } from '../settings/types';
import SidebarNav from '../shell/SidebarNav.svelte';
import { appStore } from '../stores/app.svelte';
import { blankView, type SavedViewTransport } from './types';

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

let cleanup = () => {};
afterEach(() => {
	cleanup();
	cleanup = () => {};
	document.body.innerHTML = '';
	vi.restoreAllMocks();
	vi.useRealTimers();
	i18n.setLocale('en');
	appStore.savedViews = [];
	appStore.setScope({ kind: 'inbox' });
	appStore.registerShellHooks({});
});

async function settle() {
	for (let i = 0; i < 16; i++) await Promise.resolve();
	flushSync();
}

function row(id: number): MessageRowDto {
	return {
		id: `m-${id}`,
		account_id: 'a',
		folder: 'INBOX',
		uid: id,
		subject: 'Synthetic',
		sender: 'sender@example.org',
		received_at_epoch_ms: 0,
		is_read: false,
		has_attachments: false,
		tags: [],
		folder_override: null,
		category_badge: null
	};
}

function fixture() {
	const view = { ...blankView(), id: 'view', revision: 1, name: 'Clients' };
	let catalog = [view];
	const saved: SavedViewTransport = {
		list: vi.fn(async () => catalog),
		save: vi.fn(async (v) => {
			const updated = { ...v, revision: v.revision + 1 };
			catalog = catalog.map((old) => (old.id === updated.id ? updated : old));
			return updated;
		}),
		remove: vi.fn(async (id) => {
			catalog = catalog.filter((v) => v.id !== id);
		}),
		search: vi.fn(async () => [])
	};
	const transport: InboxTransport = {
		savedViews: saved,
		search: vi.fn(async () => []),
		triage: vi.fn(async () => {}),
		subscribe: vi.fn(async () => () => {})
	};
	const settingsTransport: SettingsTransport = {
		get: vi.fn(async () => ({
			ai: {
				enabled: false,
				provider: 'Ollama' as const,
				endpoint: 'http://127.0.0.1:11434',
				model: 'qwen2.5:7b-instruct'
			},
			tokenizer: 'Unicode61RemoveDiacritics' as const,
			has_api_key: false,
			model_cached: false,
			experimental_semantic_search: false
		})),
		save: vi.fn(async () => {}),
		tokenizer: vi.fn(async () => {}),
		setExperimentalSemanticSearch: vi.fn(async () => {}),
		download: vi.fn(async () => {}),
		summarize: vi.fn(async () => '')
	};
	const inbox = new InboxController(transport);
	return { view, saved, transport, settingsTransport, inbox };
}

it('uses native saved search for all pages and refreshes, invalidating stale scope responses', async () => {
	const { inbox, view, saved, transport } = fixture();
	let resolve!: (rows: MessageRowDto[]) => void;
	vi.mocked(transport.search).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	const old = inbox.search('old', 'Lexical');
	vi.mocked(saved.search)
		.mockResolvedValueOnce(Array.from({ length: PAGE_SIZE }, (_, i) => row(i + 1)))
		.mockResolvedValueOnce([row(201)])
		.mockResolvedValueOnce([row(300)]);
	await inbox.selectSavedView(view);
	resolve([row(999)]);
	await old;
	expect(inbox.rows[0].id).toBe('m-1');
	await inbox.loadMore();
	expect(saved.search).toHaveBeenLastCalledWith('view', 1, PAGE_SIZE, PAGE_SIZE);
	await inbox.refresh();
	expect(inbox.rows.map((r) => r.id)).toEqual(['m-300']);
	expect(transport.search).toHaveBeenCalledTimes(1);
	await inbox.selectScope();
	expect(inbox.savedView).toBeNull();
	expect(transport.search).toHaveBeenCalledTimes(2);
	inbox.dispose();
});

it('cannot replace another scope with a late saved-view response or mutation rollback', async () => {
	const { inbox, view, saved } = fixture();
	let resolve!: (rows: MessageRowDto[]) => void;
	vi.mocked(saved.search).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	const old = inbox.selectSavedView(view);
	await inbox.selectScope();
	resolve([row(1)]);
	await old;
	expect(inbox.rows).toEqual([]);
	expect(inbox.loading).toBe(false);
	expect(inbox.savedView).toBeNull();
	inbox.dispose();
});

it('selects saved views from SidebarNav and drives native saved search through workspace scope hook', async () => {
	const { inbox, view, saved } = fixture();
	const workspace = createLiveWorkspace(inbox, { connected: true });
	appStore.savedViews = [{ id: view.id, name: view.name, query: view.query }];
	appStore.registerShellHooks({
		onScopeChange: (scope) => void workspace.applyShellScope(scope)
	});

	const target = document.createElement('div');
	document.body.appendChild(target);
	const component = mount(SidebarNav, { target });
	cleanup = () => {
		inbox.dispose();
		void unmount(component);
	};
	flushSync();

	const button = [...target.querySelectorAll('button')].find(
		(b) => b.textContent?.trim() === 'Clients'
	)!;
	expect(button).toBeTruthy();
	button.click();
	await settle();

	expect(appStore.activeScope).toEqual({ kind: 'saved_view', view_id: 'view' });
	expect(saved.search).toHaveBeenCalledWith('view', 1, PAGE_SIZE, 0);
	expect(inbox.savedView?.id).toBe('view');
});

it('reloads newly imported saved views into SidebarNav via InboxSession on triage-definitions-imported', async () => {
	const { transport, settingsTransport, saved, view } = fixture();
	const target = document.createElement('div');
	document.body.appendChild(target);
	const sessionComponent = mount(InboxSession, {
		target,
		props: {
			connected: true,
			inboxTransport: transport,
			settingsTransport
		}
	});
	const navContainer = document.createElement('div');
	document.body.appendChild(navContainer);
	const navComponent = mount(SidebarNav, { target: navContainer });
	cleanup = () => {
		void unmount(sessionComponent);
		void unmount(navComponent);
	};
	await settle();

	expect(navContainer.textContent).toContain('Clients');

	vi.mocked(saved.list).mockResolvedValue([
		{ ...view, id: 'imported', name: 'Imported synthetic view' }
	]);
	window.dispatchEvent(new Event('triage-definitions-imported'));
	await settle();

	expect(navContainer.textContent).toContain('Imported synthetic view');
});
