import { beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { InboxController } from './controller.svelte';
import WdgMessageList from './WdgMessageList.svelte';
import { createLiveWorkspace } from './workspace.svelte';
import { ROW_HEIGHT } from './virtual';
import { validTag, shortcut } from './triage';
import { BadgeLoader, badgeText } from './badges';
import type { InboxTransport, MessageRowDto } from './types';
import type { MessageDetailDto, ReaderTransport } from '../reader/types';

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
const row: MessageRowDto = {
	id: 'synthetic-message',
	account_id: 'local',
	folder: 'INBOX',
	uid: 1,
	subject: 'Synthetic',
	sender: 'sender@example.com',
	received_at_epoch_ms: 0,
	is_read: false,
	has_attachments: false,
	tags: [],
	folder_override: null,
	category_badge: null
};
function deferred<T>() {
	let resolve!: (result: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}
function setup() {
	const triage = vi.fn<InboxTransport['triage']>().mockResolvedValue(undefined);
	const search = vi.fn<InboxTransport['search']>().mockResolvedValue([row]);
	const inbox = new InboxController({
		triage,
		search,
		subscribe: async () => () => {}
	});
	inbox.rows = [row];
	inbox.selectedId = row.id;
	return { inbox, triage, search };
}
describe('triage mutations', () => {
	it('updates optimistically, prevents duplicates and rolls back a rejected write', async () => {
		const { inbox, triage } = setup();
		const request = deferred<void>();
		triage.mockReturnValueOnce(request.promise);
		const saving = inbox.mutate(row.id, 'MarkRead');
		expect(inbox.rows[0].is_read).toBe(true);
		await inbox.mutate(row.id, 'Trash');
		expect(triage).toHaveBeenCalledOnce();
		request.reject('private server data');
		await saving;
		expect(inbox.rows[0].is_read).toBe(false);
		expect(inbox.mutationError).toBe('Change could not be saved. Please try again.');
		expect(inbox.mutating).toBe(false);
		inbox.dispose();
	});
	it('keeps canonical attachment identity while providing folder and tag feedback', async () => {
		const { inbox, triage } = setup();
		await inbox.mutate(row.id, 'Archive');
		expect(inbox.rows[0]).toEqual({ ...row, folder_override: 'Archive' });
		expect(inbox.folderLabels[row.id]).toBe('Archive');
		await inbox.mutate(row.id, { Tag: 'Receipts' });
		expect(inbox.localTags[row.id]).toEqual(['Receipts']);
		expect(triage).toHaveBeenLastCalledWith({
			message_ids: [row.id],
			action: { Tag: 'Receipts' }
		});
		inbox.dispose();
	});
	it('rehydrates persisted metadata and clears overrides after remote MOVE refresh', async () => {
		const { inbox, search, triage } = setup();
		const saved = { ...row, tags: ['Keep'], folder_override: 'Trash' };
		search.mockResolvedValueOnce([saved]);
		await inbox.search('', 'Lexical');
		expect(inbox.localTags[row.id]).toEqual(['Keep']);
		expect(inbox.folderLabels[row.id]).toBe('Trash');
		triage.mockRejectedValueOnce('private');
		await inbox.mutate(row.id, { Tag: 'Rejected' });
		expect(inbox.localTags[row.id]).toEqual(['Keep']);
		triage.mockRejectedValueOnce('private');
		await inbox.mutate(row.id, 'Archive');
		expect(inbox.folderLabels[row.id]).toBe('Trash');
		search.mockResolvedValueOnce([{ ...saved, folder: 'Deleted', uid: 77, folder_override: null }]);
		await inbox.refresh();
		expect(inbox.folderLabels[row.id]).toBeUndefined();
		expect(inbox.rows[0].folder).toBe('Deleted');
		expect(inbox.rows[0].uid).toBe(77);
		expect(inbox.localTags[row.id]).toEqual(['Keep']);
		search.mockResolvedValueOnce([]);
		await inbox.search('other', 'Lexical');
		expect(inbox.localTags).toEqual({});
		inbox.dispose();
	});
	it('rejects invalid tags using the backend UTF-8 byte limit', async () => {
		const { inbox, triage } = setup();
		for (const tag of ['', ' ', '\ninvalid', 'é'.repeat(65)])
			await inbox.mutate(row.id, { Tag: tag });
		expect(triage).not.toHaveBeenCalled();
		expect(validTag('é'.repeat(64))).toBe(true);
		inbox.dispose();
	});
	it('defers sync refresh until the write finishes and ignores an earlier read', async () => {
		const { inbox, search, triage } = setup();
		const stale = deferred<MessageRowDto[]>();
		search.mockReturnValueOnce(stale.promise);
		const refreshing = inbox.refresh();
		const request = deferred<void>();
		triage.mockReturnValueOnce(request.promise);
		const saving = inbox.mutate(row.id, 'MarkRead');
		await inbox.refresh();
		expect(search).toHaveBeenCalledOnce();
		stale.resolve([row]);
		await refreshing;
		expect(inbox.rows[0].is_read).toBe(true);
		request.resolve();
		await saving;
		search.mockResolvedValueOnce([{ ...row, is_read: true }]);
		await inbox.refresh();
		expect(inbox.rows[0].is_read).toBe(true);
		inbox.dispose();
	});
	it('does not roll old rows into a newly selected search or resurrect after disposal', async () => {
		const { inbox, search, triage } = setup();
		const pending = deferred<void>();
		triage.mockReturnValueOnce(pending.promise);
		const saving = inbox.mutate(row.id, 'MarkRead');
		search.mockResolvedValueOnce([]);
		await inbox.search('new', 'Lexical');
		pending.reject('private');
		await saving;
		expect(inbox.rows).toEqual([]);
		expect(inbox.localTags).toEqual({});
		expect(inbox.folderLabels).toEqual({});
		inbox.dispose();
	});
});
describe('badge loading', () => {
	it('reuses extraction metadata and refetches after an authoritative sync revision', async () => {
		const body = vi.fn<ReaderTransport['body']>().mockResolvedValue({
			id: 'cached',
			triage_badges: [{ OtpCode: { code: '123456' } }]
		} as unknown as MessageDetailDto);
		const loader = new BadgeLoader({ body, attachment: vi.fn() });
		const done = vi.fn();
		loader.watch('cached', done);
		await vi.waitFor(() => expect(done).toHaveBeenCalledOnce());
		loader.watch('cached', done);
		expect(body).toHaveBeenCalledOnce();
		loader.watch('cached', done, 1);
		await vi.waitFor(() => expect(done).toHaveBeenCalledTimes(3));
		expect(body).toHaveBeenCalledTimes(2);
	});
	it('caps parallel requests and cancels queued/in-flight callbacks for unmounted rows', async () => {
		const requests = Array.from({ length: 4 }, () => deferred<MessageDetailDto>());
		const body = vi.fn<ReaderTransport['body']>();
		for (const request of requests) body.mockReturnValueOnce(request.promise);
		const attachment = vi.fn<ReaderTransport['attachment']>();
		const loader = new BadgeLoader({ body, attachment });
		const done = vi.fn();
		const stops = Array.from({ length: 12 }, (_, id) => loader.watch(String(id), done));
		expect(body).toHaveBeenCalledTimes(4);
		stops.forEach((stop) => stop());
		requests.forEach((request, id) =>
			request.resolve({
				id: String(id),
				triage_badges: []
			} as unknown as MessageDetailDto)
		);
		await vi.waitFor(() => expect(body).toHaveBeenCalledTimes(4));
		expect(done).not.toHaveBeenCalled();
		expect(attachment).not.toHaveBeenCalled();
	});
	it('formats exact cents and date without local timezone date shifting', () => {
		expect(
			badgeText({
				FebrabanBoleto: {
					amount_cents: 12345,
					due_date_iso: '2026-10-30',
					digitable_line: '001'
				}
			}).value
		).toContain('123,45');
		expect(
			badgeText({
				FebrabanBoleto: {
					amount_cents: 0,
					due_date_iso: '2026-10-30',
					digitable_line: '001'
				}
			}).value
		).toContain('2026-10-30');
		expect(badgeText({ UnsubscribeAvailable: { supports_one_click: false } }).value).toBe(
			'One-click unavailable'
		);
	});
	it('renders triage badges directly from row summary without calling badgeLoader in WdgMessageList', () => {
		const rowWithBadge: MessageRowDto = {
			...row,
			id: 'badge-row',
			triage_badges: [{ OtpCode: { code: '998877' } }]
		};
		const body = vi.fn();
		const target = document.createElement('div');
		document.body.appendChild(target);
		const inbox = new InboxController({
			triage: async () => {},
			search: async () => [rowWithBadge],
			subscribe: async () => () => {}
		});
		inbox.rows = [rowWithBadge];
		const workspace = createLiveWorkspace(inbox, {
			readerTransport: { body, attachment: vi.fn() }
		});
		const instance = mount(WdgMessageList, {
			target,
			props: { workspace }
		});
		flushSync();
		expect(target.textContent).toContain('998877');
		expect(body).not.toHaveBeenCalled();
		unmount(instance);
		inbox.dispose();
		target.remove();
	});

	it('virtualizes large lists using ROW_HEIGHT=88 without mounting all rows and supports keyboard navigation', () => {
		const rows: MessageRowDto[] = Array.from({ length: 200 }, (_, i) => ({
			...row,
			id: `row-${i}`,
			uid: i + 1,
			subject: `Message ${i}`
		}));
		const inbox = new InboxController({
			triage: async () => {},
			search: async () => rows,
			subscribe: async () => () => {}
		});
		inbox.rows = rows;
		const workspace = createLiveWorkspace(inbox, {
			readerTransport: { body: vi.fn().mockResolvedValue({ id: '', triage_badges: [] }), attachment: vi.fn() }
		});
		const target = document.createElement('div');
		document.body.appendChild(target);
		const instance = mount(WdgMessageList, { target, props: { workspace } });
		flushSync();

		const spacer = target.querySelector<HTMLElement>('[role="listbox"] > div');
		expect(spacer?.style.height).toBe(`${200 * ROW_HEIGHT}px`);
		const mountedOptions = target.querySelectorAll('[role="option"]');
		expect(mountedOptions.length).toBeGreaterThan(0);
		expect(mountedOptions.length).toBeLessThan(30);

		const listbox = target.querySelector<HTMLElement>('[role="listbox"]')!;
		listbox.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
		flushSync();
		expect(inbox.selectedId).toBe('row-0');
		listbox.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }));
		flushSync();
		expect(inbox.selectedId).toBe('row-199');

		unmount(instance);
		inbox.dispose();
		target.remove();
	});
});
describe('shortcuts', () => {
	it('requires explicit palette selection for unsubscribe and ignores repeat/composition', () => {
		expect(shortcut(new KeyboardEvent('keydown', { key: 'u' }))).toBeNull();
		expect(shortcut(new KeyboardEvent('keydown', { key: 'e', repeat: true }))).toBeNull();
		expect(shortcut(new KeyboardEvent('keydown', { key: 'e', isComposing: true }))).toBeNull();
		expect(shortcut(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))).toBe('palette');
	});
});
