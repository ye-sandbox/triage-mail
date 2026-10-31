import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import InboxView from './InboxView.svelte';
import TriageBadges from './TriageBadges.svelte';
import { InboxController } from './controller.svelte';
import type { InboxTransport, MessageRowDto } from './types';
import type { MessageDetailDto, ReaderTransport, TriageBadge } from '../reader/types';
const row: MessageRowDto = {
	id: 'local-message',
	account_id: 'local',
	folder: 'INBOX',
	uid: 1,
	subject: 'Synthetic message',
	sender: 'sender@example.com',
	received_at_epoch_ms: 0,
	is_read: false,
	has_attachments: false,
	category_badge: 'Newsletter'
};
const badges: TriageBadge[] = [
	{ OtpCode: { code: '123456' } },
	{
		FebrabanBoleto: {
			amount_cents: 12345,
			due_date_iso: '2026-10-30',
			digitable_line: '001'
		}
	},
	{ PixPayload: { payload: '<img src=x onerror=alert(1)>' } },
	{ NfeInvoice: { access_key: '1'.repeat(44) } },
	{ TrackingCode: { carrier: 'Synthetic', code: 'TEST123' } },
	{ UnsubscribeAvailable: { supports_one_click: true } }
];
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
	Element.prototype.scrollIntoView = vi.fn();
});
afterEach(async () => {
	await cleanup?.();
	cleanup = undefined;
	vi.useRealTimers();
});
function setup(supports = true) {
	const triage = vi.fn<InboxTransport['triage']>().mockResolvedValue(undefined);
	const inbox = new InboxController({
		triage,
		search: async () => [row],
		subscribe: async () => () => {}
	});
	inbox.rows = [row];
	inbox.selectedId = row.id;
	const body = vi.fn<ReaderTransport['body']>().mockResolvedValue({
		id: row.id,
		subject: row.subject,
		sender: row.sender,
		headers: '',
		text_plain: 'Synthetic body',
		html_raw: null,
		triage_result: { category: 'Newsletter', badges: [] },
		triage_badges: [{ UnsubscribeAvailable: { supports_one_click: supports } }],
		attachments_meta: []
	} satisfies MessageDetailDto);
	const attachment = vi.fn<ReaderTransport['attachment']>();
	const target = document.createElement('div');
	document.body.append(target);
	const component = mount(InboxView, {
		target,
		props: { inbox, connected: true, readerTransport: { body, attachment } }
	});
	flushSync();
	cleanup = async () => {
		inbox.dispose();
		await unmount(component);
		target.remove();
	};
	const trigger = target.querySelector<HTMLButtonElement>('[aria-haspopup=dialog]')!;
	return { inbox, triage, target, trigger, attachment };
}
function key(element: EventTarget, value: string, extra: KeyboardEventInit = {}) {
	element.dispatchEvent(
		new KeyboardEvent('keydown', {
			key: value,
			bubbles: true,
			cancelable: true,
			...extra
		})
	);
	flushSync();
}
function commandInput() {
	return document.querySelector<HTMLInputElement>('[role=combobox]')!;
}
function filter(value: string) {
	const input = commandInput();
	input.value = value;
	input.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	return input;
}
describe('keyboard palette', () => {
	it('opens using Ctrl K, filters, executes on Enter and restores focus', async () => {
		const { trigger, triage } = setup();
		trigger.focus();
		key(window, 'k', { ctrlKey: true });
		await vi.waitFor(() => expect(document.activeElement).toBe(commandInput()));
		const input = filter('unread');
		expect(document.querySelectorAll('[role=dialog] [role=option]')).toHaveLength(1);
		key(input, 'Enter');
		await vi.waitFor(() =>
			expect(triage).toHaveBeenCalledWith({
				message_ids: [row.id],
				action: 'MarkUnread'
			})
		);
		await vi.waitFor(() => expect(document.activeElement).toBe(trigger));
	});
	it('keeps focus in the dialog, supports arrow navigation, closes on Escape and resets search', async () => {
		const { trigger, triage } = setup();
		trigger.focus();
		trigger.click();
		flushSync();
		await vi.waitFor(() => expect(document.activeElement).toBe(commandInput()));
		const input = commandInput();
		key(input, 'ArrowDown');
		expect(input.getAttribute('aria-activedescendant')).toBe('triage-command-1');
		filter('archive');
		key(input, 'Escape');
		await vi.waitFor(() => expect(document.querySelector('[role=dialog]')).toBeNull());
		expect(triage).not.toHaveBeenCalled();
		trigger.click();
		flushSync();
		await vi.waitFor(() => expect(commandInput()?.value).toBe(''));
	});
	it('does not trigger shortcuts from editable fields and validates local tag input', async () => {
		const { target, trigger, triage } = setup();
		const search = target.querySelector('input[type=search]')!;
		key(search, 'e');
		expect(triage).not.toHaveBeenCalled();
		trigger.focus();
		key(window, 't');
		await vi.waitFor(() => expect(document.querySelector('#local-tag')).not.toBeNull());
		const input = document.querySelector<HTMLInputElement>('#local-tag')!;
		input.value = 'é'.repeat(65);
		input.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();
		expect(document.querySelector<HTMLButtonElement>('button[type=submit]')!.disabled).toBe(true);
		input.value = 'Receipts';
		input.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();
		document
			.querySelector<HTMLFormElement>('[role=dialog] form')!
			.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		await vi.waitFor(() =>
			expect(triage).toHaveBeenCalledWith({
				message_ids: [row.id],
				action: { Tag: 'Receipts' }
			})
		);
	});
	it('requires supported one-click metadata and an explicit selection before sending', async () => {
		const { trigger, triage, attachment } = setup();
		await vi.waitFor(() => expect(document.body.textContent).toContain('One-click available'));
		expect(triage).not.toHaveBeenCalled();
		trigger.click();
		flushSync();
		filter('unsubscribe');
		const option = document.querySelector<HTMLButtonElement>('[role=dialog] [role=option]')!;
		expect(option.getAttribute('aria-disabled')).toBe('false');
		option.click();
		flushSync();
		await vi.waitFor(() =>
			expect(triage).toHaveBeenCalledWith({
				message_ids: [row.id],
				action: 'OneClickUnsubscribe'
			})
		);
		expect(attachment).not.toHaveBeenCalled();
	});
	it('keeps unsupported unsubscribe disabled', async () => {
		const { trigger, triage } = setup(false);
		trigger.click();
		flushSync();
		filter('unsubscribe');
		const option = document.querySelector<HTMLButtonElement>('[role=dialog] [role=option]')!;
		expect(option.getAttribute('aria-disabled')).toBe('true');
		option.click();
		flushSync();
		expect(triage).not.toHaveBeenCalled();
	});
});
describe('extracted badge UI', () => {
	it('renders all extraction values as accessible text without interpreting payload markup', async () => {
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(TriageBadges, { target, props: { badges } });
		flushSync();
		expect(target.querySelectorAll('li')).toHaveLength(6);
		expect(target.textContent).toContain('123,45');
		expect(target.textContent).toContain('2026-10-30');
		expect(target.textContent).toContain('<img src=x onerror=alert(1)>');
		expect(target.querySelector('img')).toBeNull();
		expect(target.textContent).toContain('TEST123');
		expect(target.textContent).toContain('1'.repeat(44));
		await unmount(component);
		target.remove();
	});
});
