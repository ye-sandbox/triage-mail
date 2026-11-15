import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import type { Settings, SettingsTransport } from '../settings/types';
import { appStore } from '../stores/app.svelte';
import { InboxController, PAGE_SIZE } from './controller.svelte';
import type { InboxTransport, MessageRowDto, SyncProgressEvent } from './types';
import { ROW_HEIGHT, visibleRange } from './virtual';
import WdgInboxStatsBar from './WdgInboxStatsBar.svelte';
import WdgMessageList from './WdgMessageList.svelte';
import WdgMessageReader from './WdgMessageReader.svelte';
import InboxSession from '../onboarding/InboxSession.svelte';
import { createLiveWorkspace } from './workspace.svelte';

function row(index: number): MessageRowDto {
	return {
		id: `message-${index}`,
		account_id: 'account-local',
		folder: 'INBOX',
		uid: index + 1,
		subject: `Subject ${index}`,
		sender: 'sender@example.com',
		received_at_epoch_ms: 1700000000000,
		is_read: false,
		has_attachments: false,
		tags: [],
		folder_override: null,
		category_badge: null
	};
}

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

function mockSettingsTransport(experimentalSemanticSearch = false): SettingsTransport {
	const current: Settings = {
		ai: {
			enabled: false,
			provider: 'Ollama',
			endpoint: 'http://127.0.0.1:11434',
			model: 'qwen2.5:7b-instruct'
		},
		tokenizer: 'Unicode61RemoveDiacritics',
		has_api_key: false,
		model_cached: false,
		experimental_semantic_search: experimentalSemanticSearch
	};
	return {
		get: vi.fn(async () => ({ ...current })),
		save: vi.fn(async () => {}),
		tokenizer: vi.fn(async () => {}),
		setExperimentalSemanticSearch: vi.fn(async () => {}),
		download: vi.fn(async () => {}),
		summarize: vi.fn(async () => '')
	};
}

function setup() {
	const search = vi.fn<InboxTransport['search']>().mockResolvedValue([]);
	let handler: (event: SyncProgressEvent) => void = () => {};
	const stop = vi.fn();
	const transport: InboxTransport = {
		triage: async () => {},
		search,
		subscribe: async (callback) => {
			handler = callback;
			return stop;
		}
	};
	const inbox = new InboxController(transport);
	const workspace = createLiveWorkspace(inbox, {
		connected: true,
		readerTransport: {
			body: vi.fn().mockResolvedValue({
				id: 'message-0',
				account_id: 'account-local',
				folder: 'INBOX',
				uid: 1,
				subject: 'Subject 0',
				sender: 'sender@example.com',
				received_at_epoch_ms: 1700000000000,
				html_raw: null,
				text_plain: 'Plain body',
				attachments_meta: [],
				triage_badges: [],
				triage_result: {
					category: 'Personal',
					priority: 'Low',
					reasons: []
				}
			}),
			attachment: vi.fn()
		},
		settingsTransport: mockSettingsTransport()
	});
	return {
		inbox,
		workspace,
		transport,
		search,
		stop,
		sync: () =>
			handler({
				account_id: 'account-local',
				state: 'ResyncRequired',
				new_messages_count: 1,
				triage_alerts: []
			})
	};
}

beforeAll(() => {
	// jsdom has no layout engine; browser QA verifies actual resize/scroll behavior.
	vi.stubGlobal(
		'ResizeObserver',
		class {
			observe() {}
			unobserve() {}
			disconnect() {}
		}
	);
});

afterEach(() => {
	vi.useRealTimers();
	document.body.innerHTML = '';
	appStore.sidebarOpen = true;
	appStore.setScope({ kind: 'inbox' });
});

describe('inbox requests and synchronization', () => {
	it('ignores stale results and forwards the contracted search mode', async () => {
		const { inbox, search } = setup();
		inbox.setExperimentalSemanticSearch(true);
		const old = deferred<MessageRowDto[]>();
		search.mockReturnValueOnce(old.promise).mockResolvedValueOnce([row(2)]);
		const first = inbox.search('old', 'Lexical');
		await inbox.search('invoice', 'Hybrid');
		old.resolve([row(1)]);
		await first;
		expect(inbox.rows.map((item) => item.id)).toEqual(['message-2']);
		expect(search).toHaveBeenLastCalledWith({
			query: 'invoice',
			mode: 'Hybrid',
			limit: PAGE_SIZE,
			offset: 0
		});
		inbox.dispose();
	});
	it('paginates without duplicates and retains selection on authoritative refresh', async () => {
		const { inbox, search } = setup();
		const page = Array.from({ length: PAGE_SIZE }, (_, i) => row(i));
		search
			.mockResolvedValueOnce(page)
			.mockResolvedValueOnce([
				row(199),
				{ ...row(200), tags: ['Paged'], folder_override: 'Archive' }
			]);
		await inbox.search('', 'Lexical');
		inbox.selectedId = row(50).id;
		await inbox.loadMore();
		expect(search).toHaveBeenLastCalledWith({
			query: '',
			mode: 'Lexical',
			limit: PAGE_SIZE,
			offset: PAGE_SIZE
		});
		expect(inbox.rows).toHaveLength(201);
		expect(inbox.localTags['message-200']).toEqual(['Paged']);
		expect(inbox.folderLabels['message-200']).toBe('Archive');
		search
			.mockResolvedValueOnce([row(999), ...page.slice(0, 199)])
			.mockResolvedValueOnce([row(199), row(200)]);
		await inbox.refresh();
		expect(inbox.selectedId).toBe('message-50');
		expect(inbox.rows[0].id).toBe('message-999');
		expect(inbox.localTags['message-200']).toEqual([]);
		expect(inbox.folderLabels['message-200']).toBeUndefined();
		expect(inbox.hasMore).toBe(false);
		inbox.dispose();
	});
	it('replays a sync refresh deferred while initial search is pending', async () => {
		vi.useFakeTimers();
		const { inbox, search, sync, stop } = setup();
		const pending = deferred<MessageRowDto[]>();
		search.mockReturnValueOnce(pending.promise).mockResolvedValueOnce([row(3)]);
		const connecting = inbox.connect();
		await Promise.resolve();
		sync();
		await vi.advanceTimersByTimeAsync(300);
		pending.resolve([row(1)]);
		await connecting;
		await vi.advanceTimersByTimeAsync(300);
		expect(inbox.rows[0].id).toBe('message-3');
		inbox.dispose();
		expect(stop).toHaveBeenCalledOnce();
		sync();
		await vi.advanceTimersByTimeAsync(500);
		expect(search).toHaveBeenCalledTimes(2);
	});
	it('never displays raw private IPC errors', async () => {
		const { inbox, search } = setup();
		search.mockRejectedValueOnce('sensitive provider response');
		await inbox.search('test', 'Semantic');
		expect(inbox.error).toBe('Mail could not be loaded. Please try again.');
		inbox.dispose();
	});
	it('surfaces embedding failure telemetry in sync status', async () => {
		let handler!: (event: SyncProgressEvent) => void;
		const inbox = new InboxController({
			triage: async () => {},
			search: async () => [],
			subscribe: async (cb) => {
				handler = cb;
				return () => {};
			}
		});
		await inbox.connect();
		expect(inbox.syncStatus).toBe('Up to date');
		handler({
			account_id: 'acct-1',
			state: 'Idle',
			new_messages_count: 5,
			triage_alerts: [],
			embedding_errors_count: 2
		});
		expect(inbox.totalEmbeddingErrors).toBe(2);
		expect(inbox.syncStatus).toBe('Up to date (2 indexing warnings)');
		inbox.dispose();
	});
});

describe('virtualized message list', () => {
	it('bounds the render window across 100,000 messages', () => {
		for (let top = 0; top < 100000 * ROW_HEIGHT; top += 7919) {
			const range = visibleRange(100000, top, 800);
			expect(range.end - range.start).toBeLessThanOrEqual(21);
			expect(range.offset).toBe(range.start * ROW_HEIGHT);
		}
	});
	it('mounts bounded DOM, supports keyboard selection and anchors sync insertions', async () => {
		const { inbox, workspace } = setup();
		inbox.rows = Array.from({ length: 100000 }, (_, i) => row(i));
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgMessageList, { target, props: { workspace } });
		flushSync();
		await tick();
		const list = target.querySelector<HTMLElement>('[role=listbox]')!;
		Object.defineProperty(list, 'clientHeight', { value: 600 });
		list.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
		flushSync();
		expect(inbox.selectedId).toBe('message-0');
		list.scrollTop = 88000;
		list.dispatchEvent(new Event('scroll'));
		flushSync();
		await tick();
		expect(target.querySelectorAll('[role=option]').length).toBeLessThanOrEqual(20);
		expect(list.hasAttribute('aria-activedescendant')).toBe(false);
		inbox.rows = [row(100001), ...inbox.rows];
		flushSync();
		await tick();
		expect(list.scrollTop).toBe(88088);
		expect(inbox.selectedId).toBe('message-0');
		await unmount(component);
		inbox.dispose();
		target.remove();
	});
});

describe('inbox toolbar and reader header', () => {
	it('renders persisted local metadata in WdgMessageReader when selected', async () => {
		const { inbox, workspace, search } = setup();
		search.mockResolvedValueOnce([{ ...row(0), tags: ['Keep'], folder_override: 'Archive' }]);
		await inbox.search('', 'Lexical');
		inbox.selectedId = 'message-0';
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgMessageReader, {
			target,
			props: { workspace }
		});
		flushSync();
		expect(target.textContent).toContain('Archive');
		expect(target.textContent).toContain('#Keep');
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('debounces search text and applies the selected matching mode via WdgInboxStatsBar', async () => {
		vi.useFakeTimers();
		const { inbox, workspace, search } = setup();
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgInboxStatsBar, {
			target,
			props: { workspace }
		});
		flushSync();
		const input = target.querySelector<HTMLInputElement>('input[type=search]')!;
		input.value = 'invoice';
		input.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();
		await vi.advanceTimersByTimeAsync(199);
		expect(search).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(1);
		expect(search).toHaveBeenLastCalledWith({
			query: 'invoice',
			mode: 'Lexical',
			limit: PAGE_SIZE,
			offset: 0
		});

		vi.useRealTimers();
		const trigger = target.querySelector<HTMLButtonElement>('button[aria-label="Search mode"]')!;
		trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
		await vi.waitFor(() =>
			expect(document.body.querySelectorAll('[role="option"]').length).toBe(3)
		);

		const items = [...document.body.querySelectorAll<HTMLElement>('[role="option"]')];
		expect(items.map((el) => el.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'Lexical',
			'Meaning (local) OFF',
			'Words + meaning (local) OFF'
		]);

		inbox.setExperimentalSemanticSearch(true);
		flushSync();
		items[2].dispatchEvent(new MouseEvent('pointerup', { bubbles: true, cancelable: true }));
		flushSync();

		await vi.waitFor(() =>
			expect(search).toHaveBeenLastCalledWith({
				query: 'invoice',
				mode: 'Hybrid',
				limit: PAGE_SIZE,
				offset: 0
			})
		);
		await unmount(component);
		inbox.dispose();
		target.remove();
	});
});

describe('inbox motion interactions', () => {
	it('toggles sidebar expanded state immediately via WdgInboxStatsBar', async () => {
		const { inbox, workspace } = setup();
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgInboxStatsBar, {
			target,
			props: { workspace }
		});
		flushSync();
		const toggle = target.querySelector<HTMLButtonElement>('[aria-label="Toggle sidebar"]')!;
		expect(toggle.getAttribute('aria-expanded')).toBe('true');
		toggle.click();
		flushSync();
		expect(toggle.getAttribute('aria-expanded')).toBe('false');
		expect(appStore.sidebarOpen).toBe(false);
		toggle.click();
		flushSync();
		expect(toggle.getAttribute('aria-expanded')).toBe('true');
		expect(appStore.sidebarOpen).toBe(true);
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('updates reader content immediately and restores keyboard focus to listbox on back navigation', async () => {
		const { transport } = setup();
		vi.mocked(transport.search).mockResolvedValue([row(0), row(1)]);
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(InboxSession, {
			target,
			props: {
				connected: true,
				inboxTransport: transport,
				settingsTransport: mockSettingsTransport()
			}
		});
		flushSync();
		await vi.waitFor(() => expect(target.querySelectorAll('[role="option"]')).toHaveLength(2));

		const options = target.querySelectorAll<HTMLElement>('[role="option"]');
		options[0].click();
		flushSync();
		options[1].click();
		flushSync();

		expect(target.querySelector('h1')?.textContent).toContain('Subject 1');

		const back = Array.from(target.querySelectorAll('button')).find((button) =>
			button.textContent?.includes('Back to list')
		)!;
		back.click();
		flushSync();
		await tick();

		await vi.waitFor(() =>
			expect(document.activeElement).toBe(target.querySelector('[role=listbox]'))
		);
		await unmount(component);
		target.remove();
	});
});
