import { invoke } from '@tauri-apps/api/core';
import type {
	InboxTransport,
	MessageRowDto,
	FolderDto
} from '$lib/inbox/types';
import type { ReaderTransport } from '$lib/reader/types';
import type { AccountConfig } from '$lib/onboarding/types';
import type { AvatarTransport } from '$lib/onboarding/avatar-types';

export const accounts: AccountConfig[] = Array.from({ length: 4 }, (_, i) => ({
	account_id: `account-${i}`,
	email_display:
		i === 1
			? 'a-very-long-synthetic-account-label-for-overflow-checks@example.org'
			: `account-${i}@example.com`,
	imap_host: 'imap.example.com',
	imap_port: 993,
	auth_method: 'AppPassword'
}));
export function createRows(): MessageRowDto[] {
	return Array.from({ length: 100_000 }, (_, i) => ({
		id: `message-${String(i).padStart(6, '0')}`,
		account_id: `account-${i % 4}`,
		folder: 'INBOX',
		uid: i + 1,
		subject: `Synthetic newsletter ${i}`,
		sender: 'sender@example.org',
		received_at_epoch_ms: 1_700_000_000_000 + i,
		is_read: i % 2 === 0,
		is_starred: i < 124,
		snooze_until_epoch_ms: null,
		has_attachments: false,
		category_badge: null,
		tags: [],
		folder_override: null
	}));
}
export const metrics = { pageCalls: 0, ipc: [] as number[] };
const native = () => '__TAURI_INTERNALS__' in window;
const scenario = () => new URLSearchParams(location.search).get('state');
let cached: MessageRowDto[] | undefined;
export const transport: InboxTransport = {
	search: async (query) => {
		metrics.pageCalls++;
		const start = performance.now();
		let result: MessageRowDto[];
		if (native()) result = await invoke('fixture_search', { query });
		else {
			cached ??= createRows();
			const scope = query.scope;
			const count = new URLSearchParams(location.search).get('accounts');
			let rows =
				count === '0'
					? []
					: count === '1'
						? cached.filter((r) => r.account_id === 'account-0')
						: cached;
			if (scope && scope !== 'GlobalAllMail') {
				const { account_id, view } = scope.AccountView;
				rows = rows.filter(
					(r) =>
						r.account_id === account_id &&
						(view === 'Starred'
							? r.is_starred
							: view === 'Snoozed'
								? false
								: typeof view === 'object'
									? view.Folder === 'INBOX'
									: true)
				);
			}
			result = rows.slice(query.offset, query.offset + query.limit);
		}
		metrics.ipc.push(performance.now() - start);
		return result;
	},
	listFolders: async (accountId) => {
		if (native()) return invoke<FolderDto[]>('fixture_folders', { accountId });
		if (scenario() === 'not-synced') return [];
		if (scenario() === 'offline') throw new Error('Synthetic offline catalog');
		if (scenario() === 'loading') return new Promise<FolderDto[]>(() => {});
		return ['INBOX', 'Archive', 'Empty', 'All Mail'].map((name) => ({
			account_id: accountId,
			folder_name: name,
			display_name: name,
			role:
				name === 'INBOX' ? 'inbox' : name === 'Archive' ? 'archive' : 'other',
			delimiter: '/',
			is_selectable: true,
			total_messages: null,
			unread_messages: null
		}));
	},

	subscribe: async () => () => {},
	triage: async () => {
		throw new Error('Read-only validation transport');
	}
};
export const avatars: AvatarTransport = {
	get: async (id) => {
		if (
			id !== 'account-0' ||
			new URLSearchParams(location.search).get('photo') !== '1'
		)
			return null;
		const canvas = document.createElement('canvas');
		canvas.width = 128;
		canvas.height = 128;
		const context = canvas.getContext('2d');
		if (!context) throw new Error('Missing fixture raster context');
		context.fillStyle = getComputedStyle(document.documentElement)
			.getPropertyValue('--accent')
			.trim();
		context.fillRect(0, 0, 128, 128);
		context.fillStyle = getComputedStyle(document.documentElement)
			.getPropertyValue('--surface')
			.trim();
		context.beginPath();
		context.arc(64, 48, 24, 0, 2 * Math.PI);
		context.fill();
		context.fillRect(24, 84, 80, 44);
		const encoded = canvas.toDataURL('image/png').split(',')[1];
		return {
			bytes: Array.from(atob(encoded), (c) => c.charCodeAt(0)),
			mime_type: 'image/png',
			width: 128,
			height: 128
		};
	},
	set: async () => {
		throw new Error('Read-only fixture');
	},
	remove: async () => {}
};
export const reader: ReaderTransport = {
	body: async (id) => ({
		id,
		subject: 'Synthetic',
		sender: 'sender@example.org',
		headers: '',
		text_plain: '',
		html_raw: null,
		triage_result: { category: 'Financial', badges: [] },
		triage_badges: [],
		attachments_meta: []
	}),
	attachment: async () => {
		throw new Error('No attachments in fixture');
	}
};
