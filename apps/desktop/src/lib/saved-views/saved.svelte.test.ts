import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { InboxController, PAGE_SIZE } from '../inbox/controller.svelte';
import type { InboxTransport, MessageRowDto } from '../inbox/types';
import { blankView, type SavedView, type SavedViewTransport } from './types';
import SavedViews from './SavedViews.svelte';
import { i18n } from '../i18n/index.svelte';

let cleanup = () => {};
afterEach(() => {
	cleanup();
	cleanup = () => {};
	document.body.innerHTML = '';
	vi.restoreAllMocks();
	vi.useRealTimers();
	i18n.locale = 'en';
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
			const saved = { ...v, revision: v.revision + 1 };
			catalog = catalog.map((old) => (old.id === saved.id ? saved : old));
			return saved;
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
	const inbox = new InboxController(transport);
	return { view, saved, transport, inbox };
}
function click(text: string) {
	const b = [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);
	expect(b).toBeTruthy();
	b!.click();
	flushSync();
}
function render(inbox: InboxController) {
	const target = document.createElement('div');
	document.body.appendChild(target);
	const component = mount(SavedViews, {
		target,
		props: { inbox, connected: true, accounts: [] }
	});
	flushSync();
	cleanup = () => {
		inbox.dispose();
		void unmount(component);
	};
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
it('creates an unsaved template and protects dirty edits on cancellation', async () => {
	const { inbox, saved } = fixture();
	render(inbox);
	await settle();
	click('Upcoming bills');
	expect(saved.save).not.toHaveBeenCalled();
	const name = document.querySelector<HTMLInputElement>('input[maxlength="120"]')!;
	name.value = 'Edited';
	name.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
	click('Cancel');
	expect(confirm).toHaveBeenCalled();
	expect(document.querySelector('[role=dialog]')).toBeTruthy();
	confirm.mockReturnValue(true);
	click('Cancel');
	await settle();
	expect(saved.save).not.toHaveBeenCalled();
});
it('saves an edited definition and deletes the selected view back to All Mail', async () => {
	const { inbox, view, saved } = fixture();
	render(inbox);
	await settle();
	click('Clients');
	await settle();
	expect(inbox.savedView?.id).toBe('view');
	click('Edit');
	const input = document.querySelector<HTMLInputElement>('input[maxlength="120"]')!;
	input.value = 'Renamed';
	input.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	document
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(saved.save).toHaveBeenCalledWith(
		expect.objectContaining({ name: 'Renamed', revision: 1 })
	);
	expect(inbox.savedView?.revision).toBe(2);
	vi.spyOn(window, 'confirm').mockReturnValue(true);
	click('Delete');
	await settle();
	expect(saved.remove).toHaveBeenCalledWith('view', 2);
	expect(inbox.savedView).toBeNull();
});
it('keeps failed saves editable and ignores completion after teardown', async () => {
	const { inbox, saved } = fixture();
	render(inbox);
	await settle();
	click('Edit');
	vi.mocked(saved.save).mockRejectedValueOnce('InvalidRequest');
	document
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(document.querySelector('[role=alert]')?.textContent).toContain('unavailable');
	expect(document.querySelector('form')).toBeTruthy();
	let resolve!: (v: SavedView) => void;
	vi.mocked(saved.save).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	document
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(document.querySelector('fieldset')!.disabled).toBe(true);
	cleanup();
	cleanup = () => {};
	resolve({ ...blankView(), revision: 1 });
	await settle();
	expect(inbox.savedView).toBeNull();
});
