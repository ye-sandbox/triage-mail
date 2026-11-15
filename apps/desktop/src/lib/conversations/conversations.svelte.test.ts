import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { InboxController } from '../inbox/controller.svelte';
import WdgConversationThread from '../inbox/WdgConversationThread.svelte';
import { createLiveWorkspace } from '../inbox/workspace.svelte';
import WdgConversationActions from './WdgConversationActions.svelte';
import type {
	ConversationPage,
	ConversationRow,
	ConversationTransport,
	ConversationPreview
} from './types';
import type { MessageRowDto } from '../inbox/types';

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

const message: MessageRowDto = {
	id: 'm1',
	account_id: 'a',
	folder: 'INBOX',
	uid: 1,
	subject: 'Synthetic conversation',
	sender: 'sender@example.org',
	received_at_epoch_ms: 1,
	is_read: false,
	has_attachments: false,
	tags: [],
	folder_override: null,
	category_badge: null
};
const group: ConversationRow = {
	conversation_id: 'group-a',
	representative: message,
	matching_count: 1,
	total_count: 2,
	unread_count: 2
};
const page: ConversationPage = { generation: 'epoch1', rows: [group] };
function fixture() {
	const api: ConversationTransport = {
		progress: vi.fn().mockResolvedValue({ indexed: 2, total: 2, ready: true }),
		search: vi.fn().mockResolvedValue(page),
		members: vi.fn().mockResolvedValue({
			generation: 'epoch1',
			total: 2,
			rows: [
				{ message, matching: true, pending: false },
				{ message: { ...message, id: 'm2', folder: 'Archive' }, matching: false, pending: true }
			]
		}),
		preview: vi.fn().mockResolvedValue({
			token: 'confirmation1',
			generation: 'epoch1',
			action: 'MarkRead',
			messages: [message],
			outside_count: 0
		} satisfies ConversationPreview),
		apply: vi.fn().mockResolvedValue(['m1']),
		status: vi
			.fn()
			.mockResolvedValue([{ message_id: 'm2', pending: true, failed: true, attempts: 1 }])
	};
	const inbox = new InboxController({
		conversations: api,
		search: async () => [],
		triage: vi.fn().mockResolvedValue(undefined),
		subscribe: async () => () => {}
	});
	return { api, inbox };
}
async function settle() {
	for (let i = 0; i < 10; i++) {
		await Promise.resolve();
		flushSync();
	}
}
const mounted: ReturnType<typeof mount>[] = [];
afterEach(() => {
	for (const app of mounted.splice(0)) void unmount(app);
	document.body.innerHTML = '';
});
describe('native conversation snapshots', () => {
	it('uses native grouped pages and carries the server generation into later pages', async () => {
		const { api, inbox } = fixture();
		inbox.conversations = true;
		await inbox.search('', 'Lexical');
		expect(inbox.rows).toEqual([message]);
		expect(inbox.conversationRows.m1.total_count).toBe(2);
		inbox.hasMore = true;
		await inbox.loadMore();
		expect(api.search).toHaveBeenLastCalledWith(
			expect.objectContaining({ offset: 1 }),
			expect.objectContaining({ generation: 'epoch1' })
		);
		inbox.dispose();
	});
	it('ignores a late grouped response after switching views', async () => {
		const { api, inbox } = fixture();
		let resolve!: (value: ConversationPage) => void;
		vi.mocked(api.search).mockReturnValueOnce(new Promise((done) => (resolve = done)));
		inbox.conversations = true;
		const first = inbox.search('', 'Lexical');
		inbox.conversations = false;
		await inbox.search('changed', 'Lexical');
		resolve(page);
		await first;
		expect(inbox.rows).toEqual([]);
		expect(inbox.conversationRows).toEqual({});
		inbox.dispose();
	});
	it('lists outside-view members and failed retries without loading bodies until opened', async () => {
		const { api, inbox } = fixture();
		inbox.conversations = true;
		await inbox.search('', 'Lexical');
		const detail = vi.fn().mockRejectedValue('NotFound');
		const workspace = createLiveWorkspace(inbox, {
			connected: true,
			readerTransport: { body: detail, attachment: vi.fn() }
		});
		const app = mount(WdgConversationThread, {
			target: document.body,
			props: { workspace, row: group, activeMemberId: 'm1', onSelectMember: vi.fn() }
		});
		mounted.push(app);
		await settle();
		expect(detail).not.toHaveBeenCalled();
		expect(document.body.textContent).toContain('Outside current view');
		expect(document.body.textContent).toContain('Remote change failed');
		const button = Array.from(document.querySelectorAll('button')).find((b) =>
			b.textContent?.includes('Preview conversation action')
		)!;
		button.click();
		await settle();
		expect(api.preview).toHaveBeenCalledWith(
			expect.anything(),
			expect.objectContaining({ generation: 'epoch1' }),
			'group-a',
			false,
			'MarkRead'
		);
		expect(document.body.textContent).toContain('Confirm affected messages');
		expect(api.apply).not.toHaveBeenCalled();
		inbox.conversationGeneration = 'epoch2';
		flushSync();
		await settle();
		expect(document.querySelector('[role=dialog]')).toBeNull();
		inbox.dispose();
	});
	it('submits only the confirmed native token and keeps failed apply feedback after refresh', async () => {
		const { api, inbox } = fixture();
		inbox.conversations = true;
		await inbox.search('', 'Lexical');
		let reject!: (reason: string) => void;
		vi.mocked(api.apply).mockReturnValueOnce(new Promise((_, fail) => (reject = fail)));
		mounted.push(
			mount(WdgConversationActions, {
				target: document.body,
				props: { inbox, row: group, accountLabel: 'Synthetic account' }
			})
		);
		await settle();
		Array.from(document.querySelectorAll('button'))
			.find((b) => b.textContent?.includes('Preview conversation action'))!
			.click();
		await settle();
		Array.from(document.querySelectorAll('button'))
			.find((b) => b.textContent?.includes('Apply to these messages'))!
			.click();
		await settle();
		expect(inbox.mutating).toBe(true);
		expect(api.apply).toHaveBeenCalledExactlyOnceWith('confirmation1');
		reject('InvalidRequest');
		await settle();
		expect(inbox.mutating).toBe(false);
		expect(inbox.mutationError).toContain('Conversation data changed');
		inbox.dispose();
	});
});
