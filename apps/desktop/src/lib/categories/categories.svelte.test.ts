import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { InboxController, PAGE_SIZE } from '../inbox/controller.svelte';
import type { InboxTransport, MessageRowDto } from '../inbox/types';
import type { CategoryMetadata, CategoryTransport } from './types';
import WdgCategoryOverrides from '../automation/WdgCategoryOverrides.svelte';
import { SettingsFormGuard } from '../settings/guards';
import { i18n } from '../i18n/index.svelte';
let cleanup = () => {};
afterEach(() => {
	cleanup();
	cleanup = () => {};
	document.body.innerHTML = '';
	vi.useRealTimers();
	i18n.locale = 'en';
});
async function settle() {
	for (let n = 0; n < 20; n++) await Promise.resolve();
	flushSync();
}
const row: MessageRowDto = {
	id: 'm',
	account_id: 'a',
	folder: 'INBOX',
	uid: 1,
	subject: 'Notification',
	sender: 'sender@example.org',
	received_at_epoch_ms: 0,
	is_read: false,
	has_attachments: false,
	category_badge: 'Notifications',
	tags: [],
	folder_override: null
};
const metadata: CategoryMetadata = {
	automatic: 'Notifications',
	effective: 'Notifications',
	reason: 'NotificationSignal',
	version: 1,
	source: 'Automatic',
	correction_revision: 0,
	sender_revision: 0,
	sender: 'sender@example.org',
	durable: false
};
function fixture() {
	const categories: CategoryTransport = {
		metadata: vi.fn(async () => ({ ...metadata })),
		correct: vi.fn(async () => ({ ...metadata })),
		list: vi.fn(async () => []),
		save: vi.fn(async (v) => ({ ...v, revision: v.revision + 1 })),
		search: vi.fn(async () => [])
	};
	const transport: InboxTransport = {
		categories,
		search: vi.fn(async () => []),
		triage: vi.fn(async () => {}),
		subscribe: vi.fn(async () => () => {})
	};
	const inbox = new InboxController(transport);
	return { categories, transport, inbox };
}
function button(text: string) {
	const value = [...document.querySelectorAll('button')].find(
		(b) => b.textContent?.trim() === text
	);
	expect(value).toBeTruthy();
	return value!;
}
it('uses category native search for pages and refresh, discarding stale ordinary results', async () => {
	const { inbox, categories, transport } = fixture();
	let resolve!: (rows: MessageRowDto[]) => void;
	vi.mocked(transport.search).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	const stale = inbox.search('old', 'Lexical');
	vi.mocked(categories.search)
		.mockResolvedValueOnce(Array.from({ length: PAGE_SIZE }, (_, n) => ({ ...row, id: `m-${n}` })))
		.mockResolvedValueOnce([{ ...row, id: 'last' }])
		.mockResolvedValueOnce([row]);
	await inbox.selectCategory('Notifications');
	resolve([{ ...row, id: 'stale' }]);
	await stale;
	expect(inbox.rows).toHaveLength(PAGE_SIZE);
	await inbox.loadMore();
	expect(inbox.rows.at(-1)?.id).toBe('last');
	await inbox.refresh();
	expect(inbox.rows).toEqual([row]);
	expect(categories.search).toHaveBeenNthCalledWith(
		2,
		expect.objectContaining({ offset: PAGE_SIZE }),
		'Notifications'
	);
	await inbox.selectScope();
	expect(inbox.category).toBeNull();
	inbox.dispose();
});

describe('WdgCategoryOverrides (UI v2)', () => {
	it('renders absent metadata cleanly, updates category, guards reset until settled, and localizes', async () => {
		const { categories } = fixture();
		const guard = new SettingsFormGuard();
		vi.mocked(categories.list).mockResolvedValue([
			{ account_id: 'a', sender: 'sender@example.org', revision: 4, category: 'Financial' }
		]);

		const component = mount(WdgCategoryOverrides, {
			target: document.body,
			props: {
				guard,
				transport: categories,
				accounts: [
					{
						account_id: 'a',
						email_display: 'user@example.com',
						imap_host: 'imap.example.com',
						imap_port: 993,
						auth_method: 'AppPassword'
					}
				]
			}
		});
		cleanup = () => void unmount(component);
		await settle();

		expect(document.body.textContent).toContain('sender@example.org');
		expect(document.body.textContent).toContain('SenderOverride');

		const select = document.body.querySelector<HTMLSelectElement>('select')!;
		select.value = 'Security';
		select.dispatchEvent(new Event('change', { bubbles: true }));
		await settle();
		expect(categories.save).toHaveBeenCalledWith({
			account_id: 'a',
			sender: 'sender@example.org',
			revision: 4,
			category: 'Security'
		});

		let resolveReset!: (v: import('./types').SenderOverride) => void;
		vi.mocked(categories.save).mockReturnValueOnce(new Promise((r) => (resolveReset = r)));
		button('Reset').click();
		flushSync();
		expect(guard.isBlocked()).toBe(true);
		expect(categories.save).toHaveBeenLastCalledWith({
			account_id: 'a',
			sender: 'sender@example.org',
			revision: 4,
			category: null
		});
		resolveReset({ account_id: 'a', sender: 'sender@example.org', revision: 5, category: null });
		await settle();
		expect(guard.isBlocked()).toBe(false);

		i18n.locale = 'pt-BR';
		flushSync();
		expect(document.body.textContent).toContain('Substituições Locais de Categoria');
	});
});
