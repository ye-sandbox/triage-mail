import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { InboxController, PAGE_SIZE } from '../inbox/controller.svelte';
import type { InboxTransport, MessageRowDto } from '../inbox/types';
import type { ReviewTransport, SenderChoice, ReviewPreferences } from './types';
import ReviewActions from './ReviewActions.svelte';
import ReviewModule from './ReviewModule.svelte';
import WdgSenderReviewQueue from '../automation/WdgSenderReviewQueue.svelte';
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
	for (let n = 0; n < 30; n++) await Promise.resolve();
	flushSync();
}
const row: MessageRowDto = {
	id: 'm',
	account_id: 'a',
	folder: 'INBOX',
	uid: 1,
	subject: 'Notice',
	sender: 'sender@example.org',
	received_at_epoch_ms: 0,
	is_read: false,
	has_attachments: false,
	category_badge: null,
	tags: [],
	folder_override: null
};
const choice: SenderChoice = {
	account_id: 'a',
	sender: 'sender@example.org',
	revision: 1,
	state: 'Pending'
};
const pref: ReviewPreferences = {
	account_id: 'a',
	revision: 0,
	enabled: false,
	hide_in_inbox: false,
	initializing: false
};
function fixture() {
	const review: ReviewTransport = {
		preferences: vi.fn(async () => ({ ...pref })),
		savePreferences: vi.fn(async (v) => ({ ...v, revision: v.revision + 1 })),
		metadata: vi.fn(async () => ({ ...choice })),
		list: vi.fn(async () => [{ ...choice }]),
		save: vi.fn(async (v) => ({ ...v, revision: v.revision + 1 })),
		search: vi.fn(async () => [])
	};
	const transport: InboxTransport = {
		review,
		search: vi.fn(async () => []),
		triage: vi.fn(async () => {}),
		subscribe: vi.fn(async () => () => {})
	};
	return { review, transport, inbox: new InboxController(transport) };
}
function button(text: string) {
	const found = [...document.querySelectorAll('button')].find(
		(b) => b.textContent?.trim() === text
	);
	expect(found).toBeTruthy();
	return found!;
}
function check(index: number) {
	const input = document.querySelectorAll<HTMLInputElement>('input[type=checkbox]')[index];
	input.checked = true;
	input.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
}
it('uses native review pages and discards late results across review and mailbox changes', async () => {
	const { review, transport, inbox } = fixture();
	let resolve!: (rows: MessageRowDto[]) => void;
	vi.mocked(transport.search).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	const old = inbox.search('old', 'Lexical');
	vi.mocked(review.search)
		.mockResolvedValueOnce(Array.from({ length: PAGE_SIZE }, (_, n) => ({ ...row, id: `m-${n}` })))
		.mockResolvedValueOnce([{ ...row, id: 'last' }]);
	await inbox.selectReview('Pending');
	resolve([{ ...row, id: 'stale' }]);
	await old;
	expect(inbox.rows).toHaveLength(PAGE_SIZE);
	await inbox.loadMore();
	expect(inbox.rows.at(-1)?.id).toBe('last');
	expect(review.search).toHaveBeenNthCalledWith(
		2,
		expect.objectContaining({ offset: PAGE_SIZE }),
		'Pending'
	);
	let late!: (rows: MessageRowDto[]) => void;
	vi.mocked(review.search).mockReturnValueOnce(new Promise((r) => (late = r)));
	const pending = inbox.refresh();
	await inbox.selectScope();
	late([row]);
	await pending;
	expect(inbox.rows).toEqual([]);
	expect(inbox.review).toBeNull();
	inbox.dispose();
});
it('requires exact sender and account consent and never sends a triage mutation', async () => {
	vi.useFakeTimers();
	const { review, transport, inbox } = fixture();
	inbox.rows = [row];
	const component = mount(ReviewActions, {
		target: document.body,
		props: { inbox, message: row, accountLabel: 'Account A' }
	});
	cleanup = () => {
		inbox.dispose();
		void unmount(component);
	};
	await settle();
	expect(document.body.textContent).toContain('does not authenticate');
	expect(document.body.textContent).toContain('Account A');
	expect(button('Decline sender').disabled).toBe(true);
	check(0);
	button('Decline sender').click();
	await settle();
	expect(review.save).toHaveBeenCalledWith({ ...choice, state: 'Declined' });
	expect(transport.triage).not.toHaveBeenCalled();
});
it('keeps unresolved senders visible and ignores metadata after unmount', async () => {
	const { review, inbox } = fixture();
	vi.mocked(review.metadata).mockResolvedValueOnce(null);
	const component = mount(ReviewActions, {
		target: document.body,
		props: { inbox, message: row, accountLabel: 'A' }
	});
	await settle();
	expect(document.body.textContent).toContain('This message stays visible');
	expect(document.querySelector('input')).toBeNull();
	await unmount(component);
	let resolve!: (v: SenderChoice) => void;
	vi.mocked(review.metadata).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	const next = mount(ReviewActions, {
		target: document.body,
		props: { inbox, message: row, accountLabel: 'A' }
	});
	flushSync();
	await unmount(next);
	resolve(choice);
	await settle();
	expect(document.body.textContent).toBe('');
	inbox.dispose();
});
it('guards settings drafts and writes, keeps hiding separate, and renders Portuguese', async () => {
	const { review } = fixture();
	const guard = new SettingsFormGuard();
	const account = {
		account_id: 'a',
		email_display: 'reader@example.com',
		imap_host: 'imap.example.com',
		imap_port: 993,
		auth_method: 'AppPassword' as const
	};
	const component = mount(ReviewModule, {
		target: document.body,
		props: { guard, transport: review, accounts: [account] }
	});
	cleanup = () => void unmount(component);
	await settle();
	check(0);
	expect(guard.needsConfirmation()).toBe(true);
	let resolve!: (v: ReviewPreferences) => void;
	vi.mocked(review.savePreferences).mockReturnValueOnce(new Promise((r) => (resolve = r)));
	button('Save preferences').click();
	await settle();
	expect(guard.isBlocked()).toBe(true);
	expect(review.savePreferences).toHaveBeenCalledWith({
		...pref,
		enabled: true,
		hide_in_inbox: false
	});
	resolve({ ...pref, revision: 1, enabled: true, initializing: true });
	await settle();
	expect(guard.isBlocked()).toBe(false);
	expect(guard.needsConfirmation()).toBe(false);
	i18n.locale = 'pt-BR';
	flushSync();
	expect(document.body.textContent).toContain('Novos remetentes');
	expect(document.body.textContent).toContain('não autentica');
});

describe('WdgSenderReviewQueue (UI v2)', () => {
	it('requires explicit sender consent, guards preferences writes, polls while initializing, and supports pt-BR', async () => {
		const { review } = fixture();
		const guard = new SettingsFormGuard();
		const account = {
			account_id: 'a',
			email_display: 'reader@example.com',
			imap_host: 'imap.example.com',
			imap_port: 993,
			auth_method: 'AppPassword' as const
		};

		const component = mount(WdgSenderReviewQueue, {
			target: document.body,
			props: { guard, transport: review, accounts: [account] }
		});
		cleanup = () => void unmount(component);
		await settle();

		expect(document.body.textContent).toContain('does not authenticate');
		expect(document.body.textContent).toContain('sender@example.org');

		// Accept / Decline disabled until per-sender exact consent is checked
		expect(button('Accept sender').disabled).toBe(true);
		expect(button('Decline sender').disabled).toBe(true);

		const rowConsent = document.body.querySelector<HTMLInputElement>('input[type=checkbox]')!;
		rowConsent.checked = true;
		rowConsent.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();
		expect(button('Decline sender').disabled).toBe(false);

		button('Decline sender').click();
		await settle();
		expect(review.save).toHaveBeenCalledWith({ ...choice, state: 'Declined' });

		// Toggle enable switch -> marks SettingsFormGuard dirty
		const enableSwitch = document.body.querySelector<HTMLButtonElement>(
			'button[aria-label="Toggle enable sender review"]'
		)!;
		enableSwitch.click();
		flushSync();
		expect(guard.needsConfirmation()).toBe(true);

		let resolvePref!: (v: ReviewPreferences) => void;
		vi.mocked(review.savePreferences).mockReturnValueOnce(new Promise((r) => (resolvePref = r)));
		button('Save preferences').click();
		await settle();
		expect(guard.isBlocked()).toBe(true);

		resolvePref({ ...pref, revision: 1, enabled: true, initializing: true });
		await settle();
		expect(guard.isBlocked()).toBe(false);

		i18n.locale = 'pt-BR';
		flushSync();
		expect(document.body.textContent).toContain('Fila de Revisão de Novos Remetentes');
		expect(document.body.textContent).toContain('não autentica');
	});
});
