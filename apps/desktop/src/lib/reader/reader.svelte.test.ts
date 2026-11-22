import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { InboxController } from '../inbox/controller.svelte';
import WdgMessageReader from '../inbox/WdgMessageReader.svelte';
import { createLiveWorkspace } from '../inbox/workspace.svelte';
import { appStore } from '../stores/app.svelte';
import type { MessageRowDto } from '../inbox/types';
import type { CategoryMetadata } from '../categories/types';
import type { Settings, SettingsTransport } from '../settings/types';
import type { MessageDetailDto, ReaderTransport } from './types';
import { safeFilename } from './download';
import {
	MAX_PREVIEW_BYTES,
	canPreviewAttachment,
	classifyPreviewMime,
	createEphemeralPreview
} from './preview';

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
	const saveAttachment = vi
		.fn<NonNullable<ReaderTransport['saveAttachment']>>()
		.mockResolvedValue({ bytes_written: 3 });
	const summarize = vi
		.fn<NonNullable<ReaderTransport['summarize']>>()
		.mockResolvedValue('Concise local summary.');
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
			saveAttachment,
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
		saveAttachment,
		summarize,
		settingsTransport
	};
}

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
	vi.useRealTimers();
	document.body.innerHTML = '';
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

	it('saves directly to disk on Save As click, rejects duplicate clicks, and handles cancel cleanly', async () => {
		const { inbox, target, render, body, saveAttachment, attachment } = setup();
		body.mockResolvedValue(detail('first'));
		const pending = deferred<{ bytes_written: number } | null>();
		saveAttachment.mockReturnValueOnce(pending.promise).mockResolvedValueOnce(null);
		const create = vi.fn().mockReturnValue('blob:unexpected');
		vi.stubGlobal(
			'URL',
			Object.assign(class extends URL {}, {
				createObjectURL: create,
				revokeObjectURL: vi.fn()
			})
		);
		const anchorClick = vi.spyOn(HTMLAnchorElement.prototype, 'click');
		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Save As invoice.pdf"]')).not.toBeNull()
		);
		expect(saveAttachment).not.toHaveBeenCalled();
		expect(attachment).not.toHaveBeenCalled();
		const button = target.querySelector<HTMLButtonElement>('[aria-label="Save As invoice.pdf"]')!;
		button.click();
		button.click();
		flushSync();
		expect(saveAttachment).toHaveBeenCalledExactlyOnceWith({
			account_id: 'local',
			folder: 'INBOX',
			uid: 1,
			part_id: '2'
		});
		pending.resolve({ bytes_written: 3 });
		await vi.waitFor(() => expect(target.textContent).toContain('Saved'));
		expect(create).not.toHaveBeenCalled();
		expect(anchorClick).not.toHaveBeenCalled();

		// User cancelling native save dialog clears status cleanly with no error.
		target.querySelector<HTMLButtonElement>('[aria-label="Save As invoice.pdf"]')!.click();
		await vi.waitFor(() => expect(saveAttachment).toHaveBeenCalledTimes(2));
		await vi.waitFor(() => expect(target.textContent).not.toContain('Saved'));
		expect(target.textContent).not.toContain('Save failed');

		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('ignores a dismissed message save and redacts Save As failures', async () => {
		const { inbox, target, render, body, saveAttachment } = setup();
		body.mockResolvedValue(detail('first'));
		const pending = deferred<{ bytes_written: number } | null>();
		saveAttachment.mockRejectedValueOnce('private error').mockReturnValueOnce(pending.promise);
		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Save As invoice.pdf"]')).not.toBeNull()
		);
		target.querySelector<HTMLButtonElement>('[aria-label="Save As invoice.pdf"]')!.click();
		await vi.waitFor(() => expect(target.textContent).toContain('Save failed'));
		expect(target.textContent).not.toContain('private error');
		target.querySelector<HTMLButtonElement>('[aria-label="Save As invoice.pdf"]')!.click();
		inbox.selectedId = null;
		flushSync();
		pending.resolve({ bytes_written: 1 });
		await tick();
		expect(target.querySelector('[aria-label="Save As invoice.pdf"]')).toBeNull();
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('shows clear offline archive message when Save As returns Unsupported', async () => {
		const { inbox, target, render, body, saveAttachment } = setup();
		body.mockResolvedValue(detail('first'));
		saveAttachment.mockRejectedValueOnce('Unsupported');
		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Save As invoice.pdf"]')).not.toBeNull()
		);
		target.querySelector<HTMLButtonElement>('[aria-label="Save As invoice.pdf"]')!.click();
		await vi.waitFor(() =>
			expect(target.textContent).toContain(
				'Offline archive — attachment binary is not stored for on-demand fetch.'
			)
		);
		await unmount(component);
		inbox.dispose();
		target.remove();
	});

	it('previews allowlisted PDF/images/text ephemerally and revokes blob URLs on close and message switch', async () => {
		const { inbox, secondRow, target, render, body, attachment } = setup();
		const pdfDetail = detail('first');
		pdfDetail.attachments_meta = [
			...pdfDetail.attachments_meta,
			{
				part_id: '3',
				filename: 'logo.svg',
				mime_type: 'image/svg+xml',
				size_bytes: 120,
				content_id: null,
				is_inline: false
			},
			{
				part_id: '4',
				filename: 'huge.png',
				mime_type: 'image/png',
				size_bytes: MAX_PREVIEW_BYTES + 1,
				content_id: null,
				is_inline: false
			},
			{
				part_id: '5',
				filename: 'notes.txt',
				mime_type: 'text/plain; charset=utf-8',
				size_bytes: 19,
				content_id: null,
				is_inline: false
			},
			{
				part_id: '6',
				filename: 'photo.png',
				mime_type: 'image/png',
				size_bytes: 4,
				content_id: null,
				is_inline: false
			}
		];
		body.mockImplementation(async (id) => (id === 'first' ? pdfDetail : detail(id)));
		const create = vi
			.fn()
			.mockReturnValueOnce('blob:pdf-preview')
			.mockReturnValueOnce('blob:png-preview');
		const revoke = vi.fn();
		vi.stubGlobal(
			'URL',
			Object.assign(class extends URL {}, {
				createObjectURL: create,
				revokeObjectURL: revoke
			})
		);
		const textBytes = Array.from(new TextEncoder().encode('Hello <script> text'));
		const pdfBytes = [0x25, 0x50, 0x44, 0x46];
		const pngBytes = [0x89, 0x50, 0x4e, 0x47];
		attachment
			.mockResolvedValueOnce({ bytes: pdfBytes })
			.mockResolvedValueOnce({ bytes: textBytes })
			.mockResolvedValueOnce({ bytes: pngBytes });

		const component = render();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Preview invoice.pdf"]')).not.toBeNull()
		);

		// SVG and > 10 MiB attachments have disabled Preview buttons and never hit IPC.
		const svgPreviewBtn = target.querySelector<HTMLButtonElement>('[aria-label="Preview logo.svg"]')!;
		const hugePreviewBtn = target.querySelector<HTMLButtonElement>('[aria-label="Preview huge.png"]')!;
		expect(svgPreviewBtn.disabled).toBe(true);
		expect(hugePreviewBtn.disabled).toBe(true);
		svgPreviewBtn.click();
		hugePreviewBtn.click();
		expect(attachment).not.toHaveBeenCalled();

		// 1) PDF preview renders isolated blob iframe and revokes on close.
		target.querySelector<HTMLButtonElement>('[aria-label="Preview invoice.pdf"]')!.click();
		await vi.waitFor(() =>
			expect(document.body.querySelector('[data-testid="attachment-preview-dialog"]')).not.toBeNull()
		);
		const dialog = document.body.querySelector('[data-testid="attachment-preview-dialog"]')!;
		const pdfFrame = dialog.querySelector('iframe')!;
		expect(pdfFrame).not.toBeNull();
		expect(pdfFrame.getAttribute('src')).toBe('blob:pdf-preview');
		expect(pdfFrame.getAttribute('sandbox')).toBe('allow-scripts');
		expect(pdfFrame.getAttribute('referrerpolicy')).toBe('no-referrer');
		expect(pdfBytes.every((b) => b === 0)).toBe(true);

		document.body.querySelector<HTMLButtonElement>('[aria-label="Close preview"]')!.click();
		flushSync();
		expect(revoke).toHaveBeenCalledWith('blob:pdf-preview');
		const closedDialog = document.body.querySelector('[data-testid="attachment-preview-dialog"]');
		expect(closedDialog === null || closedDialog.getAttribute('data-state') === 'closed').toBe(true);
		expect(document.body.querySelector('[data-testid="attachment-preview-dialog"] iframe')).toBeNull();

		// 2) Text preview renders safe preformatted text with zeroed source bytes and no blob URL.
		target.querySelector<HTMLButtonElement>('[aria-label="Preview notes.txt"]')!.click();
		await vi.waitFor(() =>
			expect(
				document.body.querySelector('[data-testid="attachment-preview-dialog"] pre')?.textContent
			).toBe('Hello <script> text')
		);
		expect(create).toHaveBeenCalledTimes(1);
		expect(textBytes.every((b) => b === 0)).toBe(true);
		document.body.querySelector<HTMLButtonElement>('[aria-label="Close preview"]')!.click();
		flushSync();

		// 3) Raster PNG preview revokes blob URL immediately when switching messages.
		target.querySelector<HTMLButtonElement>('[aria-label="Preview photo.png"]')!.click();
		await vi.waitFor(() =>
			expect(
				document.body
					.querySelector('[data-testid="attachment-preview-dialog"] img')
					?.getAttribute('src')
			).toBe('blob:png-preview')
		);
		expect(pngBytes.every((b) => b === 0)).toBe(true);
		inbox.selectedId = secondRow.id;
		flushSync();
		await vi.waitFor(() => expect(revoke).toHaveBeenCalledWith('blob:png-preview'));

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

it('normalizes filenames, enforces preview MIME/size allowlists, and wipes ephemeral buffers', () => {
	expect(safeFilename('..\\folder/' + String.fromCodePoint(0x202e) + ' invoice?.pdf')).toBe(
		'_ invoice_.pdf'
	);
	expect(safeFilename('../../..')).toBe('attachment');

	expect(classifyPreviewMime('image/png')).toBe('image');
	expect(classifyPreviewMime('IMAGE/JPEG; name=a.jpg')).toBe('image');
	expect(classifyPreviewMime('image/webp')).toBe('image');
	expect(classifyPreviewMime('image/gif')).toBe('image');
	expect(classifyPreviewMime('image/svg+xml')).toBeNull();
	expect(classifyPreviewMime('text/html')).toBeNull();
	expect(classifyPreviewMime('application/octet-stream')).toBeNull();
	expect(classifyPreviewMime('text/csv; charset=utf-8')).toBe('text');
	expect(classifyPreviewMime('application/json')).toBe('text');
	expect(classifyPreviewMime('application/xml')).toBe('text');
	expect(classifyPreviewMime('application/pdf')).toBe('pdf');

	expect(canPreviewAttachment({ mime_type: 'image/png', size_bytes: MAX_PREVIEW_BYTES })).toBe(true);
	expect(canPreviewAttachment({ mime_type: 'image/png', size_bytes: MAX_PREVIEW_BYTES + 1 })).toBe(
		false
	);
	expect(canPreviewAttachment({ mime_type: 'image/svg+xml', size_bytes: 16 })).toBe(false);

	const revoke = vi.fn();
	vi.stubGlobal(
		'URL',
		Object.assign(class extends URL {}, {
			createObjectURL: () => 'blob:ephemeral',
			revokeObjectURL: revoke
		})
	);
	const raw = new Uint8Array([1, 2, 3, 4]);
	const handle = createEphemeralPreview(raw, {
		filename: 'img.png',
		mime_type: 'image/png',
		size_bytes: 4
	});
	expect(handle.blobUrl).toBe('blob:ephemeral');
	handle.release();
	handle.release();
	expect(revoke).toHaveBeenCalledOnce();
	expect(Array.from(raw)).toEqual([0, 0, 0, 0]);
});

it('displays recipients and navigates to the owning account inbox on click', async () => {
	const priorScope = appStore.activeScope;
	const priorAccounts = appStore.accounts;
	appStore.setScope({ kind: 'all' });

	const target = document.body.appendChild(document.createElement('div'));
	const inbox = new InboxController({
		search: async () => [],
		triage: async () => {},
		subscribe: async () => () => {}
	});
	inbox.rows = [row('first')];
	inbox.selectedId = 'first';

	const workspace = createLiveWorkspace(inbox, {
		accounts: [
			{
				account_id: 'local',
				email_display: 'mailbox@example.com',
				imap_host: 'imap.example.com',
				imap_port: 993,
				auth_method: 'AppPassword'
			}
		],
		readerTransport: {
			body: async () => ({
				...detail('first'),
				recipients: 'Alice <alice@example.org>, list@example.org'
			}),
			attachment: async () => ({ bytes: [] })
		},
		settingsTransport: {
			get: async () => ({
				ai: { enabled: false, provider: 'Ollama', endpoint: 'http://127.0.0.1:11434', model: 'llama3.2' },
				tokenizer: 'Unicode61RemoveDiacritics',
				has_api_key: false,
				model_cached: false,
				experimental_semantic_search: false
			}),
			save: async () => {},
			tokenizer: async () => {},
			setExperimentalSemanticSearch: async () => {},
			download: async () => {},
			summarize: async () => 'Summary'
		}
	});
	const onBackToList = vi.fn();
	const component = mount(WdgMessageReader, { target, props: { workspace, onBackToList } });

	try {
		await vi.waitFor(() => {
			expect(target.textContent).toContain('Alice <alice@example.org>, list@example.org');
			expect(target.textContent).toContain('(mailbox@example.com)');
		});
		const btn = target.querySelector<HTMLButtonElement>(
			'button[aria-label*="mailbox@example.com"]'
		);
		expect(btn).not.toBeNull();
		btn!.click();
		flushSync();
		expect(appStore.activeScope).toEqual({
			kind: 'folder',
			account_id: 'local',
			folder: 'INBOX'
		});
		expect(onBackToList).toHaveBeenCalledOnce();
	} finally {
		await unmount(component);
		inbox.dispose();
		target.remove();
		appStore.setScope(priorScope);
		appStore.accounts = priorAccounts;
	}
});
