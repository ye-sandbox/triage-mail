import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import InboxSession from '../onboarding/InboxSession.svelte';
import { appStore } from '$lib/stores/app.svelte';
import WdgCommandPalette from './WdgCommandPalette.svelte';
import WdgExtractionSummary from './WdgExtractionSummary.svelte';
import { InboxController } from './controller.svelte';
import { createLiveWorkspace, workspaceRegistry } from './workspace.svelte';
import type { InboxTransport, MessageRowDto } from './types';
import type { MessageDetailDto, ReaderTransport, TriageBadge } from '../reader/types';
import type { Settings, SettingsTransport } from '../settings/types';

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
	tags: [],
	folder_override: null,
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
	appStore.commandOpen = false;
	appStore.setLocale('en');
	await cleanup?.();
	cleanup = undefined;
	workspaceRegistry.setActive(null);
	vi.useRealTimers();
});

function createSettingsMock(): SettingsTransport {
	const settings: Settings = {
		ai: { enabled: false, provider: 'Ollama', endpoint: 'http://127.0.0.1:11434', model: 'llama3' },
		tokenizer: 'Unicode61RemoveDiacritics',
		has_api_key: false,
		model_cached: false,
		experimental_semantic_search: false
	};
	return {
		get: vi.fn(async () => settings),
		save: vi.fn(async () => {}),
		tokenizer: vi.fn(async () => {}),
		setExperimentalSemanticSearch: vi.fn(async () => {}),
		download: vi.fn(async () => {}),
		summarize: vi.fn(async () => '')
	};
}

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

	const workspace = createLiveWorkspace(inbox, {
		connected: true,
		readerTransport: { body, attachment },
		settingsTransport: createSettingsMock()
	});
	workspace.recordRowBadges(row.id, [
		{ UnsubscribeAvailable: { supports_one_click: supports } }
	]);

	const restoreSpy = vi.fn();
	const target = document.createElement('div');
	document.body.append(target);
	const palette = mount(WdgCommandPalette, {
		target,
		props: { workspace, restoreFocus: restoreSpy }
	});
	flushSync();
	cleanup = async () => {
		inbox.dispose();
		await unmount(palette);
		target.remove();
	};
	return { inbox, workspace, triage, target, attachment, restoreSpy };
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

async function filter(value: string) {
	const input = commandInput();
	input.value = value;
	input.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	await tick();
	await tick();
	flushSync();
	return input;
}

describe('UI v2 WdgCommandPalette', () => {
	it('opens via appStore.commandOpen, filters actions, executes on Enter, and invokes restoreFocus', async () => {
		const { triage, restoreSpy } = setup();
		appStore.commandOpen = true;
		flushSync();
		await vi.waitFor(() => expect(document.activeElement).toBe(commandInput()));

		const input = await filter('unread');
		const options = document.querySelectorAll('[role=dialog] [role=option]');
		expect(options.length).toBe(1);
		expect(options[0].textContent).toContain('Mark unread');

		key(input, 'Enter');
		await vi.waitFor(() =>
			expect(triage).toHaveBeenCalledWith({
				message_ids: [row.id],
				action: 'MarkUnread'
			})
		);
		await vi.waitFor(() => expect(restoreSpy).toHaveBeenCalled());
	});

	it('supports arrow navigation, closes on Escape without executing, and resets filter on reopen', async () => {
		const { triage } = setup();
		appStore.commandOpen = true;
		flushSync();
		await vi.waitFor(() => expect(document.activeElement).toBe(commandInput()));
		const input = commandInput();
		key(input, 'ArrowDown');
		expect(input.getAttribute('aria-activedescendant')).toBeTruthy();

		await filter('archive');
		key(input, 'Escape');
		await vi.waitFor(() => expect(document.querySelector('[role=dialog]')).toBeNull());
		expect(triage).not.toHaveBeenCalled();

		appStore.commandOpen = true;
		flushSync();
		await vi.waitFor(() => expect(commandInput()?.value).toBe(''));
	});

	it('does not trigger shortcuts from editable fields and validates local tag 128-byte UTF-8 input', async () => {
		const triage = vi.fn<InboxTransport['triage']>().mockResolvedValue(undefined);
		const target = document.createElement('div');
		document.body.append(target);
		const session = mount(InboxSession, {
			target,
			props: {
				connected: true,
				inboxTransport: {
					triage,
					search: async () => [row],
					subscribe: async () => () => {}
				},
				settingsTransport: createSettingsMock()
			}
		});
		const palette = mount(WdgCommandPalette, { target });
		flushSync();
		cleanup = async () => {
			await unmount(palette);
			await unmount(session);
			target.remove();
		};

		await vi.waitFor(() => expect(target.querySelector('[role="option"]')).not.toBeNull());
		target.querySelector<HTMLElement>('[role="option"]')?.click();
		flushSync();

		const search = target.querySelector<HTMLInputElement>('input[type=search]')!;
		key(search, 'e');
		expect(triage).not.toHaveBeenCalled();

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

	it('requires supported one-click metadata and explicit selection before sending unsubscribe', async () => {
		const { triage, attachment } = setup(true);
		appStore.commandOpen = true;
		flushSync();
		await vi.waitFor(() => expect(commandInput()).not.toBeNull());

		await filter('unsubscribe');
		const option = document.querySelector<HTMLElement>('[role=dialog] [role=option]')!;
		expect(option.getAttribute('aria-disabled')).not.toBe('true');
		expect(triage).not.toHaveBeenCalled();

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

	it('opens WdgSnoozeSheet state when selecting Snooze from the palette', async () => {
		const { workspace } = setup(true);
		appStore.commandOpen = true;
		flushSync();
		await vi.waitFor(() => expect(commandInput()).not.toBeNull());

		const command = await filter('snooze');
		key(command, 'Enter');
		await vi.waitFor(() => expect(workspace.snoozeOpen).toBe(true));
		expect(workspace.snoozeTargetId).toBe(row.id);
	});

	it('keeps unsupported unsubscribe disabled in palette', async () => {
		const { triage } = setup(false);
		appStore.commandOpen = true;
		flushSync();
		await vi.waitFor(() => expect(commandInput()).not.toBeNull());

		await filter('unsubscribe');
		const option = document.querySelector<HTMLElement>('[role=dialog] [role=option]')!;
		expect(option.getAttribute('aria-disabled')).toBe('true');
		option.click();
		flushSync();
		expect(triage).not.toHaveBeenCalled();
	});
});

describe('UI v2 extracted badge UI (WdgExtractionSummary)', () => {
	it('renders all extraction values as accessible text without interpreting payload markup', async () => {
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgExtractionSummary, { target, props: { badges } });
		flushSync();
		expect(target.textContent).toContain('123.45');
		appStore.setLocale('pt-BR');
		flushSync();
		expect(target.textContent).toContain('123,45');
		expect(target.textContent).toContain('2026-10-30');
		expect(target.textContent).toContain('<img src=x onerror=alert(1)>');
		expect(target.querySelector('img')).toBeNull();
		expect(target.textContent).toContain('TEST123');
		expect(target.textContent).toContain('1'.repeat(44));
		expect(target.textContent).toContain('RFC 8058');
		await unmount(component);
		target.remove();
	});
});
