import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { InboxController, PAGE_SIZE } from './controller.svelte';
import { Mailboxes, folderGroups } from './mailboxes.svelte';
import InboxView from './InboxView.svelte';
import SnoozeForm from './SnoozeForm.svelte';
import type { FolderDto, InboxTransport, MailboxScope, MessageRowDto } from './types';
import type { AccountConfig } from '../onboarding/types';
import { i18n } from '../i18n/index.svelte';
const accounts: AccountConfig[] = ['first', 'second'].map((account_id) => ({
	account_id,
	email_display: `${account_id}@example.com`,
	imap_host: 'imap.example.com',
	imap_port: 993,
	auth_method: 'AppPassword'
}));
const row: MessageRowDto = {
	id: 'message',
	account_id: 'first',
	folder: 'INBOX',
	uid: 7,
	subject: 'Synthetic',
	sender: 'sender@example.com',
	received_at_epoch_ms: 0,
	is_read: false,
	is_starred: false,
	snooze_until_epoch_ms: null,
	has_attachments: true,
	tags: [],
	folder_override: null,
	category_badge: null
};
const scope = (account_id: string): MailboxScope => ({
	AccountView: { account_id, view: 'Inbox' }
});
const folder = (folder_name: string, role = 'other', is_selectable = true): FolderDto => ({
	account_id: 'first',
	folder_name,
	display_name: folder_name,
	delimiter: '/',
	role,
	is_selectable,
	total_messages: null,
	unread_messages: null
});
function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}
function setup() {
	const search = vi.fn<InboxTransport['search']>().mockResolvedValue([]);
	const triage = vi.fn<InboxTransport['triage']>().mockResolvedValue(undefined);
	const listFolders = vi
		.fn()
		.mockResolvedValue([
			folder('INBOX', 'inbox'),
			folder('Spam', 'junk'),
			folder('Sent', 'sent'),
			folder('Projects/Local')
		]);
	const inbox = new InboxController({
		search,
		triage,
		listFolders,
		subscribe: async () => () => {}
	});
	return { inbox, search, triage, listFolders };
}
let cleanup: (() => Promise<void>) | undefined;
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
beforeEach(() => {
	localStorage.clear();
	i18n.setLocale('en');
});
afterEach(async () => {
	await cleanup?.();
	cleanup = undefined;
	vi.useRealTimers();
	document.body.innerHTML = '';
});

describe('mailbox scope isolation', () => {
	it('drops a delayed global search and clears reader and pagination when switching accounts', async () => {
		const { inbox, search } = setup();
		const old = deferred<MessageRowDto[]>();
		search.mockReturnValueOnce(old.promise);
		const initial = inbox.search('invoice', 'Lexical');
		inbox.selectedId = row.id;
		search.mockResolvedValueOnce([{ ...row, account_id: 'second', id: 'second-message' }]);
		await inbox.selectScope(scope('second'));
		old.resolve([row]);
		await initial;
		expect(inbox.rows.map((r) => r.id)).toEqual(['second-message']);
		expect(inbox.selectedId).toBeNull();
		expect(inbox.hasMore).toBe(false);
		expect(search).toHaveBeenLastCalledWith({
			query: 'invoice',
			mode: 'Lexical',
			limit: PAGE_SIZE,
			offset: 0,
			scope: scope('second')
		});
		inbox.dispose();
	});
	it('scopes deep pages and ignores late pagination after a view switch', async () => {
		const { inbox, search } = setup();
		search.mockResolvedValueOnce(
			Array.from({ length: PAGE_SIZE }, (_, i) => ({ ...row, id: `first-${i}` }))
		);
		await inbox.selectScope(scope('first'));
		const old = deferred<MessageRowDto[]>();
		search.mockReturnValueOnce(old.promise);
		const more = inbox.loadMore();
		expect(search).toHaveBeenLastCalledWith(
			expect.objectContaining({ scope: scope('first'), offset: PAGE_SIZE })
		);
		await inbox.selectScope({ AccountView: { account_id: 'first', view: 'Starred' } });
		old.resolve([{ ...row, id: 'late-page' }]);
		await more;
		expect(inbox.rows).toEqual([]);
		expect(inbox.loading).toBe(false);
		inbox.dispose();
	});
	it('keeps scoped refreshes and does not accept an old refresh in a new account', async () => {
		const { inbox, search } = setup();
		await inbox.selectScope(scope('first'));
		const old = deferred<MessageRowDto[]>();
		search.mockReturnValueOnce(old.promise);
		const refresh = inbox.refresh();
		expect(search).toHaveBeenLastCalledWith(expect.objectContaining({ scope: scope('first') }));
		await inbox.selectScope(scope('second'));
		old.resolve([row]);
		await refresh;
		expect(inbox.rows).toEqual([]);
		await inbox.selectScope();
		expect(search.mock.calls.at(-1)?.[0]).not.toHaveProperty('scope');
		inbox.dispose();
	});
	it.each([true, false])(
		'isolates delayed mutation feedback and rollback (success=%s)',
		async (success) => {
			const { inbox, search, triage } = setup();
			search.mockResolvedValueOnce([row]);
			await inbox.selectScope(scope('first'));
			const old = deferred<void>();
			triage.mockReturnValueOnce(old.promise);
			const mutation = inbox.mutate(row.id, 'Star');
			expect(inbox.rows[0].is_starred).toBe(true);
			await inbox.selectScope(scope('second'));
			if (success) old.resolve();
			else old.reject('private diagnostics');
			await mutation;
			expect(inbox.rows).toEqual([]);
			expect(inbox.mutationError).toBe('');
			expect(inbox.mutationNotice).toBe('');
			inbox.dispose();
		}
	);
	it('preserves IMAP identity through star/snooze actions and rolls back rejected snooze', async () => {
		const { inbox, triage } = setup();
		inbox.rows = [row];
		await inbox.mutate(row.id, 'Star');
		expect(inbox.rows[0].is_starred).toBe(true);
		await inbox.mutate(row.id, 'Unstar');
		expect(inbox.rows[0].is_starred).toBe(false);
		const until_epoch_ms = Date.now() + 60000;
		await inbox.mutate(row.id, { Snooze: { until_epoch_ms } });
		expect(inbox.rows[0]).toMatchObject({
			account_id: 'first',
			folder: 'INBOX',
			uid: 7,
			snooze_until_epoch_ms: until_epoch_ms
		});
		triage.mockRejectedValueOnce('InvalidRequest');
		await inbox.mutate(row.id, 'Unsnooze');
		expect(inbox.rows[0].snooze_until_epoch_ms).toBe(until_epoch_ms);
		await inbox.mutate(row.id, 'Archive');
		expect(inbox.rows[0].snooze_until_epoch_ms).toBeNull();
		inbox.dispose();
	});
});

describe('folder catalogs and local preferences', () => {
	it('preserves cached folders on failure, ignores stale results and filters other accounts', async () => {
		const load = vi.fn().mockResolvedValue([folder('Archive', 'archive')]);
		const boxes = new Mailboxes(load);
		await boxes.refresh('first');
		load.mockRejectedValueOnce('private');
		await boxes.refresh('first');
		expect(boxes.catalogs.first.folders[0].folder_name).toBe('Archive');
		expect(boxes.catalogs.first.error).toBe(true);
		const old = deferred<FolderDto[]>();
		load.mockReturnValueOnce(old.promise);
		const pending = boxes.refresh('first');
		load.mockResolvedValueOnce([folder('New'), { ...folder('Foreign'), account_id: 'second' }]);
		await boxes.refresh('first');
		old.resolve([folder('Old')]);
		await pending;
		expect(boxes.catalogs.first.folders.map((f) => f.folder_name)).toEqual(['New']);
		boxes.dispose();
	});
	it('invalidates catalogs on account removal and teardown', async () => {
		const old = deferred<FolderDto[]>();
		const boxes = new Mailboxes(() => old.promise);
		boxes.reconcile(accounts);
		boxes.reconcile([]);
		old.resolve([folder('Private')]);
		await old.promise;
		await tick();
		expect(boxes.catalogs).toEqual({});
		boxes.dispose();
	});
	it('persists independent disclosures using only opaque IDs and handles malformed preferences', () => {
		localStorage.setItem('triage-mail.mailbox-expansion', '{broken');
		const boxes = new Mailboxes(async () => []);
		boxes.toggle('first');
		boxes.toggle('second');
		expect(JSON.parse(localStorage.getItem('triage-mail.mailbox-expansion')!)).toEqual([
			'first',
			'second'
		]);
		const remount = new Mailboxes(async () => []);
		expect(remount.expanded).toEqual(['first', 'second']);
		boxes.toggle('first');
		expect(boxes.expanded).toEqual(['second']);
		boxes.dispose();
		remount.dispose();
	});
	it('groups only actual selectable server folders and recognizes the native junk role', () => {
		const groups = folderGroups([
			folder('INBOX', 'inbox'),
			folder('Spam', 'junk'),
			folder('Container', 'other', false),
			folder('Sent', 'sent')
		]);
		expect(groups.special.map((f) => f.folder_name)).toEqual(['Spam']);
		expect(groups.other.map((f) => f.folder_name)).toEqual(['Sent']);
	});
});

describe('account navigation components', () => {
	async function mountInbox() {
		const { inbox, search, listFolders, triage } = setup();
		const avatarTransport = { get: vi.fn().mockResolvedValue(null), set: vi.fn(), remove: vi.fn() };
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(InboxView, {
			target,
			props: { inbox, connected: true, accounts, avatarTransport }
		});
		flushSync();
		cleanup = async () => {
			inbox.dispose();
			await unmount(component);
			target.remove();
		};
		await vi.waitFor(() => expect(listFolders).toHaveBeenCalledTimes(2));
		await tick();
		flushSync();
		return { inbox, search, triage, target, avatarTransport };
	}
	it('expands accounts independently, scopes real folders, and preserves expansion on remount', async () => {
		const { target, search, inbox, avatarTransport } = await mountInbox();
		const headers = accounts.map((a) =>
			target.querySelector<HTMLButtonElement>(`button[title="${a.email_display}"]`)!
		);
		headers[0].click();
		headers[1].click();
		flushSync();
		expect(headers.map((h) => h.getAttribute('aria-expanded'))).toEqual(['true', 'true']);
		expect(inbox.scope).toBeUndefined();
		expect(search).not.toHaveBeenCalled();
		const nav = target.querySelector('nav')!;
		const spam = Array.from(nav.querySelectorAll('button')).find((b) => b.textContent === 'Spam')!;
		spam.click();
		flushSync();
		await tick();
		expect(search).toHaveBeenLastCalledWith(
			expect.objectContaining({
				scope: { AccountView: { account_id: 'first', view: { Folder: 'Spam' } } }
			})
		);
		expect(spam.getAttribute('aria-current')).toBe('page');
		expect(target.querySelector('h1')?.textContent).toContain('first@example.com');
		expect(avatarTransport.get).toHaveBeenCalledWith('first');
		expect(target.querySelectorAll('img')).toHaveLength(0);
		expect(JSON.parse(localStorage.getItem('triage-mail.mailbox-expansion')!)).toEqual([
			'first',
			'second'
		]);
		headers[0].click();
		flushSync();
		expect(headers[1].getAttribute('aria-expanded')).toBe('true');
	});
	it('keeps the shell mounted for account additions/removal and restores disclosures on remount', async () => {
		const { inbox, search, listFolders } = setup();
		const avatarTransport = { get: vi.fn().mockResolvedValue(null), set: vi.fn(), remove: vi.fn() };
		const props = $state({ inbox, connected: true, accounts: [accounts[0]], avatarTransport });
		const target = document.createElement('div');
		document.body.append(target);
		let component = mount(InboxView, { target, props });
		flushSync();
		cleanup = async () => {
			inbox.dispose();
			await unmount(component);
		};
		const main = target.querySelector('main');
		const first = target.querySelector<HTMLButtonElement>('button[title="first@example.com"]')!;
		first.focus();
		first.click();
		flushSync();
		props.accounts = accounts;
		flushSync();
		expect(target.querySelector('main')).toBe(main);
		expect(target.querySelector('button[title="second@example.com"]')).not.toBeNull();
		expect(first.getAttribute('aria-expanded')).toBe('true');
		await vi.waitFor(() => expect(listFolders).toHaveBeenCalledWith('second'));
		await inbox.selectScope(scope('second'));
		props.accounts = [accounts[0]];
		flushSync();
		await tick();
		expect(inbox.scope).toBeUndefined();
		expect(search.mock.calls.at(-1)?.[0]).not.toHaveProperty('scope');
		await unmount(component);
		component = mount(InboxView, { target, props });
		flushSync();
		expect(
			target.querySelector('button[title="first@example.com"]')?.getAttribute('aria-expanded')
		).toBe('true');
	});
	it('localizes account views, controls and empty states in Portuguese', async () => {
		i18n.setLocale('pt-BR');
		const { target } = await mountInbox();
		target.querySelector<HTMLButtonElement>('button[title="first@example.com"]')!.click();
		flushSync();
		expect(target.textContent).toContain('Todos os emails');
		expect(target.textContent).toContain('Com estrela');
		expect(target.textContent).toContain('Adiados');
		expect(target.textContent).toContain('Nenhuma mensagem em cache');
	});
	it('refreshes expired snoozes and keeps canonical reader/attachment identity', async () => {
		vi.useFakeTimers();
		const { inbox, search } = setup();
		inbox.rows = [{ ...row, snooze_until_epoch_ms: Date.now() + 1000 }];
		inbox.selectedId = row.id;
		const target = document.createElement('div');
		document.body.append(target);
		const body = vi.fn().mockResolvedValue({
			id: row.id,
			headers: '',
			text_plain: '',
			html_raw: null,
			attachments_meta: [],
			triage_badges: [],
			triage_result: { badges: [] }
		});
		const attachment = vi.fn();
		const component = mount(InboxView, {
			target,
			props: { inbox, connected: true, readerTransport: { body, attachment } }
		});
		flushSync();
		cleanup = async () => {
			inbox.dispose();
			await unmount(component);
		};
		expect(target.querySelector('[aria-label="Message state"]')?.textContent).toContain(
			'Snoozed until'
		);
		expect(body).toHaveBeenCalledWith(row.id);
		expect(attachment).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(1002);
		expect(search).toHaveBeenCalled();
	});
});

describe('snooze deadline input', () => {
	it('converts local date/time into UTC and rejects expired dates without sending', async () => {
		const execute = vi.fn();
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(SnoozeForm, { target, props: { execute } });
		flushSync();
		cleanup = async () => {
			await unmount(component);
		};
		const input = target.querySelector<HTMLInputElement>('input')!;
		const form = target.querySelector('form')!;
		const submit = () =>
			form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		input.value = '2000-01-01T12:00';
		input.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();
		submit();
		flushSync();
		expect(execute).not.toHaveBeenCalled();
		expect(target.querySelector('[role=alert]')).not.toBeNull();
		const date = new Date(Date.now() + 86400000);
		const local = new Date(date.getTime() - date.getTimezoneOffset() * 60000)
			.toISOString()
			.slice(0, 16);
		input.value = local;
		input.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();
		submit();
		expect(execute).toHaveBeenCalledWith({ Snooze: { until_epoch_ms: new Date(local).getTime() } });
	});
});
