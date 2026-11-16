import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { InboxController } from '../inbox/controller.svelte';
import WdgMessageReader from '../inbox/WdgMessageReader.svelte';
import { createLiveWorkspace } from '../inbox/workspace.svelte';
import type { MessageRowDto } from '../inbox/types';
import type { CategoryMetadata } from '../categories/types';
import type { Settings, SettingsTransport } from '../settings/types';
import type { MessageDetailDto, ReaderTransport } from './types';
import { safeFilename, saveAttachment } from './download';

const row = (id: string): MessageRowDto => ({
	id,
	account_id: 'local',
	folder: 'INBOX',
	uid: id === 'first' ? 1 : 2,
	subject: id,
	sender: 'sender@example.com',
	received_at_epoch_ms: 0,
	is_read: false,
	has_attachments: true,
	tags: [],
	folder_override: null,
	category_badge: 'Personal'
});

const detail = (id: string, html_raw: string | null = null): MessageDetailDto => ({
	id,
	subject: id,
	sender: 'sender@example.com',
	headers: '',
	text_plain: 'Plain <script> text',
	html_raw,
	triage_result: { category: 'Personal', badges: [] },
	triage_badges: [],
	attachments_meta: [
		{
			part_id: '2',
			filename: '../invoice.pdf',
			mime_type: 'application/pdf',
			size_bytes: 3,
			content_id: null,
			is_inline: false
		}
	]
});

function deferred<T>() {
	let resolve!: (result: T) => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<T>((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}

function setup(readerOverrides: Partial<ReaderTransport> = {}, aiEnabled = false) {
	const body = vi.fn<ReaderTransport['body']>();
	const attachment = vi.fn<ReaderTransport['attachment']>().mockResolvedValue({ bytes: [1, 2, 3] });
	const summarize = vi.fn<NonNullable<ReaderTransport['summarize']>>().mockResolvedValue('Concise local summary.');
	const categoryMetadata = vi
		.fn<NonNullable<ReaderTransport['categoryMetadata']>>()
		.mockResolvedValue({
			automatic: 'Personal',
			effective: 'Personal',
			reason: 'Fallback',
			version: 1,
			source: 'Automatic',
			correction_revision: 0,
			sender_revision: 0,
			sender: 'sender@example.com',
			durable: true
		} satisfies CategoryMetadata);
	const correctCategory = vi
		.fn<NonNullable<ReaderTransport['correctCategory']>>()
		.mockResolvedValue({
			automatic: 'Personal',
			effective: 'Financial',
			reason: 'Fallback',
			version: 1,
			source: 'Message',
			correction_revision: 1,
			sender_revision: 0,
			sender: 'sender@example.com',
			durable: true
		} satisfies CategoryMetadata);

	const settings: Settings = {
		ai: {
			enabled: aiEnabled,
			provider: 'Ollama',
			endpoint: 'http://127.0.0.1:11434',
			model: 'llama3.2:3b'
		},
		tokenizer: 'Unicode61RemoveDiacritics',
		has_api_key: false,
		model_cached: false,
		experimental_semantic_search: false
	};

	const settingsTransport: SettingsTransport = {
		get: vi.fn(async () => ({ ...settings, ai: { ...settings.ai } })),
		save: vi.fn(async () => {}),
		tokenizer: vi.fn(async () => {}),
		setExperimentalSemanticSearch: vi.fn(async () => {}),
		download: vi.fn(async () => {}),
		summarize: vi.fn(async () => 'Fallback summary')
	};

	const firstRow = row('first');
	const secondRow = row('second');
	const inbox = new InboxController({
		triage: vi.fn(async () => {}),
		search: vi.fn(async () => [firstRow, secondRow]),
		subscribe: vi.fn(async () => () => {})
	});
	inbox.rows = [firstRow, secondRow];
	inbox.selectedId = firstRow.id;

	const workspace = createLiveWorkspace(inbox, {
		connected: true,
		readerTransport: {
			body,
			attachment,
			summarize,
			categoryMetadata,
			correctCategory,
			...readerOverrides
		},
		settingsTransport
	});

	const target = document.createElement('div');
	document.body.append(target);
	const render = () => {
		const component = mount(WdgMessageReader, { target, props: { workspace } });
		flushSync();
		return component;
	};
	return {
		inbox,
		workspace,
		firstRow,
		secondRow,
		target,
		render,
		body,
		attachment,
		summarize,
		settingsTransport
	};
}

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

describe('UI v2 message reader (WdgMessageReader)', () => {
	it('allows images only after an explicit click and resets consent on message identity changes', async () => {
		const { inbox, secondRow, target, render, body } = setup();
		body.mockImplementation(async (id) =>
			detail(id, '<img src="https://images.example.com/pixel.png" onerror="alert(1)">')
		);
		const component = render();
		await vi.waitFor(() => expect(target.querySelector('iframe')).not.toBeNull());
		const frame = () => target.querySelector('iframe')!;
		expect(frame().srcdoc).not.toContain('src="https://images.example.com');
		const allow = Array.from(target.querySelectorAll('button')).find((button) =>
			button.textContent?.includes('Allow remote images for this message')
		)!;
		allow.click();
		flushSync();
		expect(frame().srcdoc).toContain('src="https://images.example.com/pixel.png"');
		expect(frame().srcdoc).not.toContain('onerror');
		expect(frame().getAttribute('sandbox')).toBe('');
		inbox.selectedId = secondRow.id;
		flushSync();
		await vi.waitFor(() => expect(target.querySelector('iframe')).not.toBeNull());
		expect(frame().srcdoc).not.toContain('src="https://images.example.com');
		expect(frame().srcdoc).toContain("connect-src 'none'");
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('ignores stale success and clears previous content on selection changes', async () => {
		const first = deferred<MessageDetailDto>();
		const second = deferred<MessageDetailDto>();
		const { inbox, secondRow, target, render, body, attachment } = setup();
		body.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
		const component = render();
		expect(target.textContent).toContain('Loading message');
		inbox.selectedId = secondRow.id;
		flushSync();
		second.resolve(detail('second', '<p>Current body</p>'));
		await vi.waitFor(() => expect(target.querySelector('iframe')).not.toBeNull());
		first.resolve(detail('first', '<p>Stale private body</p>'));
		await tick();
		flushSync();
		const frame = target.querySelector('iframe')!;
		expect(frame.srcdoc).toContain('Current body');
		expect(frame.srcdoc).not.toContain('Stale private body');
		expect(frame.getAttribute('sandbox')).toBe('');
		expect(frame.getAttribute('referrerpolicy')).toBe('no-referrer');
		expect(attachment).not.toHaveBeenCalled();
		inbox.selectedId = null;
		flushSync();
		expect(target.querySelector('iframe')).toBeNull();
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('redacts errors, supports retry, and renders plain text without interpreting markup', async () => {
		const { inbox, target, render, body } = setup();
		body.mockRejectedValueOnce('private provider data').mockResolvedValueOnce(detail('first'));
		const component = render();
		await vi.waitFor(() => expect(target.querySelector('[role=alert]')).not.toBeNull());
		expect(target.textContent).not.toContain('private provider');
		const retryBtn = Array.from(target.querySelectorAll('button')).find((b) =>
			b.textContent?.includes('Try again')
		)!;
		retryBtn.click();
		flushSync();
		await vi.waitFor(() =>
			expect(target.querySelector('pre')?.textContent).toBe('Plain <script> text')
		);
		expect(target.querySelector('script')).toBeNull();
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('ignores stale errors after clearing selection and rejects mismatched detail identity', async () => {
		const { inbox, secondRow, target, render, body } = setup();
		const pending = deferred<MessageDetailDto>();
		body.mockReturnValueOnce(pending.promise).mockResolvedValueOnce(detail('wrong-message'));
		const component = render();
		inbox.selectedId = null;
		flushSync();
		pending.reject('private stale error');
		await tick();
		expect(target.querySelector('[role=alert]')).toBeNull();
		inbox.selectedId = secondRow.id;
		flushSync();
		await vi.waitFor(() => expect(target.querySelector('[role=alert]')).not.toBeNull());
		expect(target.querySelector('pre')).toBeNull();
		expect(target.textContent).not.toContain('wrong-message');
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('downloads only on click, rejects duplicate clicks, and releases its object URL', async () => {
		const { inbox, target, render, body, attachment } = setup();
		body.mockResolvedValue(detail('first'));
		const pending = deferred<{ bytes: number[] }>();
		attachment.mockReturnValue(pending.promise);
		const create = vi.fn().mockReturnValue('blob:local');
		const revoke = vi.fn();
		vi.stubGlobal(
			'URL',
			Object.assign(class extends URL {}, {
				createObjectURL: create,
				revokeObjectURL: revoke
			})
		);
		const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Download invoice.pdf"]')).not.toBeNull()
		);
		expect(attachment).not.toHaveBeenCalled();
		const button = target.querySelector<HTMLButtonElement>('[aria-label="Download invoice.pdf"]')!;
		button.click();
		button.click();
		flushSync();
		expect(attachment).toHaveBeenCalledExactlyOnceWith({
			account_id: 'local',
			folder: 'INBOX',
			uid: 1,
			part_id: '2'
		});
		pending.resolve({ bytes: [1, 2, 3] });
		await vi.waitFor(() => expect(click).toHaveBeenCalledOnce());
		expect(create).toHaveBeenCalledOnce();
		await unmount(component);
		inbox.dispose();
		expect(revoke).toHaveBeenCalledWith('blob:local');
		target.remove();
	});

	it('ignores a dismissed message download and redacts attachment failures', async () => {
		const { inbox, target, render, body, attachment } = setup();
		body.mockResolvedValue(detail('first'));
		const pending = deferred<{ bytes: number[] }>();
		const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
		attachment.mockRejectedValueOnce('private error').mockReturnValueOnce(pending.promise);
		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Download invoice.pdf"]')).not.toBeNull()
		);
		target.querySelector<HTMLButtonElement>('[aria-label="Download invoice.pdf"]')!.click();
		await vi.waitFor(() => expect(target.textContent).toContain('Download failed'));
		expect(target.textContent).not.toContain('private error');
		target.querySelector<HTMLButtonElement>('[aria-label="Download invoice.pdf"]')!.click();
		inbox.selectedId = null;
		flushSync();
		pending.resolve({ bytes: [1] });
		await tick();
		expect(click).not.toHaveBeenCalled();
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('shows clear offline archive message when attachment download returns Unsupported', async () => {
		const { inbox, target, render, body, attachment } = setup();
		body.mockResolvedValue(detail('first'));
		attachment.mockRejectedValueOnce('Unsupported');
		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Download invoice.pdf"]')).not.toBeNull()
		);
		target.querySelector<HTMLButtonElement>('[aria-label="Download invoice.pdf"]')!.click();
		await vi.waitFor(() =>
			expect(target.textContent).toContain(
				'Offline archive — attachment binary is not stored for on-demand fetch.'
			)
		);
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('requires explicit user consent before invoking AI summarization', async () => {
		const { inbox, target, render, body, summarize, settingsTransport } = setup({}, true);
		body.mockResolvedValue(detail('first'));
		const component = render();
		await vi.waitFor(() =>
			expect(
				Array.from(target.querySelectorAll('button')).find((b) =>
					b.textContent?.includes('Summarize with Local AI')
				)
			).toBeDefined()
		);

		const prepBtn = Array.from(target.querySelectorAll('button')).find((b) =>
			b.textContent?.includes('Summarize with Local AI')
		)!;
		prepBtn.click();
		await vi.waitFor(() => expect(target.textContent).toContain('llama3.2:3b'));
		expect(settingsTransport.get).toHaveBeenCalledOnce();
		expect(summarize).not.toHaveBeenCalled();

		const sendBtn = Array.from(target.querySelectorAll('button')).find((b) =>
			b.textContent?.includes('Send and summarize')
		)!;
		sendBtn.click();
		await vi.waitFor(() => expect(target.textContent).toContain('Concise local summary.'));
		expect(summarize).toHaveBeenCalledWith(
			'first',
			true,
			expect.objectContaining({ endpoint: 'http://127.0.0.1:11434', model: 'llama3.2:3b' })
		);
		await unmount(component);
		inbox.dispose();
		target.remove();
	});
});

it('normalizes path/control filenames and revokes failed downloads', () => {
	expect(safeFilename('..\\folder/' + String.fromCodePoint(0x202e) + ' invoice?.pdf')).toBe(
		'_ invoice_.pdf'
	);
	expect(safeFilename('../../..')).toBe('attachment');
	const revoke = vi.fn();
	vi.stubGlobal(
		'URL',
		Object.assign(class extends URL {}, {
			createObjectURL: () => 'blob:test',
			revokeObjectURL: revoke
		})
	);
	vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {
		throw new Error('Save failed');
	});
	expect(() => saveAttachment([1], 'file')).toThrow('Save failed');
	expect(revoke).toHaveBeenCalledWith('blob:test');
	expect(document.querySelector('a[download]')).toBeNull();
});
