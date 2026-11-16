import { toast } from 'svelte-sonner';
import { appStore } from '$lib/stores/app.svelte';
import type { MailAccount, ScopeFilter, SyncStatus } from '$lib/types';
import type { AccountConfig } from '../onboarding/types';
import { nativeReaderTransport } from '../reader/transport';
import { demoRemindersBackend } from '../reminders/due-dates.fixtures';
import { countOverdueAndDueToday } from '../reminders/types';
import { blankView } from '../saved-views/types';
import type { ReaderTransport, TriageBadge } from '../reader/types';
import { nativeSettingsTransport } from '../settings/transport';
import type { SettingsTransport } from '../settings/types';
import { BadgeLoader } from './badges';
import { InboxController } from './controller.svelte';
import { createDemoBackend, type DemoBackend } from './demo.svelte';
import { nativeAvailable, nativeTransport } from './transport';
import type { MailboxScope, MessageRowDto, SearchMode } from './types';
import type { TriageAction } from './triage';

export function parseSender(raw: string): { name: string; email: string } {
	const trimmed = raw.trim();
	if (!trimmed) return { name: '', email: '' };
	const match = /^(.*?)\s*<([^<>]+)>\s*$/.exec(trimmed);
	if (match) {
		const name = match[1].replace(/^["']|["']$/g, '').trim();
		const email = match[2].trim();
		return { name: name || email, email };
	}
	return { name: trimmed, email: trimmed.includes('@') ? trimmed : '' };
}

export function mapSyncStatus(label: string): SyncStatus {
	if (label.startsWith('Syncing')) return 'Syncing';
	if (label.startsWith('Reconnecting')) return 'Retrying';
	if (label.startsWith('Saving')) return 'MutationPending';
	if (label.startsWith('Some changes')) return 'MutationFailed';
	if (label.startsWith('Updating')) return 'ResyncRequired';
	return 'Idle';
}

export function toShellAccounts(accounts: AccountConfig[]): MailAccount[] {
	return accounts.map((acct) => ({
		account_id: acct.account_id,
		display_name: acct.email_display,
		address: acct.email_display,
		imap_host: acct.imap_host,
		imap_port: acct.imap_port,
		tls_mode: acct.imap_port === 143 ? 'STARTTLS' : 'IMAPS',
		auth_method:
			acct.auth_method === 'AppPassword'
				? 'AppPassword-Vault'
				: acct.auth_method === 'LocalArchive'
					? 'LocalArchive'
					: 'OAuth2-PKCE',
		vault_locked: false,
		folders: [
			{ id: 'INBOX', label: 'INBOX' },
			{ id: 'Archive', label: 'Archive' },
			{ id: 'Trash', label: 'Trash' }
		]
	}));
}

export interface WorkspaceOptions {
	inbox: InboxController;
	readerTransport: ReaderTransport;
	settingsTransport: SettingsTransport;
	connected: boolean;
	isDemo?: boolean;
	accounts?: AccountConfig[];
	dueCount?: number;
	createRule?: (id: string) => void;
	demo?: DemoBackend;
}

export class InboxWorkspace {
	inbox: InboxController;
	readerTransport: ReaderTransport;
	settingsTransport: SettingsTransport;
	connected = $state(false);
	isDemo = $state(false);
	accounts = $state<AccountConfig[]>([]);
	dueCount = $state(0);
	mobileReaderOpen = $state(false);
	snoozeTargetId = $state<string | null>(null);
	snoozeOpen = $state(false);
	tagDialogTargetId = $state<string | null>(null);
	tagDialogOpen = $state(false);
	selectedBadges = $state<TriageBadge[]>([]);
	badgeLoader: BadgeLoader | undefined = $state(undefined);
	listElement = $state<HTMLElement | null>(null);
	private createRuleHook: ((id: string) => void) | undefined;
	private demoBackend: DemoBackend | undefined;
	private activeAccountId: string | null = null;
	private badgeMap = $state<Record<string, TriageBadge[]>>({});

	constructor(options: WorkspaceOptions) {
		this.inbox = options.inbox;
		this.readerTransport = options.readerTransport;
		this.settingsTransport = options.settingsTransport;
		this.connected = options.connected;
		this.isDemo = Boolean(options.isDemo);
		this.accounts = options.accounts ?? [];
		this.dueCount = options.dueCount ?? 0;
		this.createRuleHook = options.createRule;
		this.demoBackend = options.demo;
		this.badgeLoader =
			options.connected || options.isDemo ? new BadgeLoader(options.readerTransport) : undefined;
	}

	get selected(): MessageRowDto | undefined {
		return this.inbox.rows.find((row) => row.id === this.inbox.selectedId);
	}

	get canUnsubscribe(): boolean {
		const rowBadges = this.selected?.triage_badges ?? [];
		const badges = this.selectedBadges.length ? this.selectedBadges : rowBadges;
		return badges.some(
			(badge) => 'UnsubscribeAvailable' in badge && badge.UnsubscribeAvailable.supports_one_click
		);
	}

	get pendingBoletosCount(): number {
		if (this.demoBackend) {
			return this.demoBackend
				.getMessages()
				.filter((m) => m.extractions.boleto && !m.extractions.boleto.paid).length;
		}
		if (this.dueCount > 0) return this.dueCount;
		let count = 0;
		for (const row of this.inbox.rows) {
			const badges = row.triage_badges ?? this.badgeMap[row.id] ?? [];
			if (badges.some((b) => 'FebrabanBoleto' in b)) count++;
		}
		return count;
	}

	recordRowBadges(id: string, badges: TriageBadge[]) {
		this.badgeMap = { ...this.badgeMap, [id]: badges };
		if (this.inbox.selectedId === id) {
			this.selectedBadges = badges;
		}
	}

	selectMessage(id: string, openMobile = false) {
		if (this.inbox.selectedId !== id) {
			this.inbox.selectedId = id;
			this.selectedBadges = this.badgeMap[id] ?? [];
		}
		if (openMobile) {
			this.mobileReaderOpen = true;
		}
	}

	closeMobileReader() {
		this.mobileReaderOpen = false;
	}

	moveSelection(delta: -1 | 1) {
		const rows = this.inbox.rows;
		if (!rows.length) return;
		const idx = rows.findIndex((r) => r.id === this.inbox.selectedId);
		const nextIdx = idx === -1 ? 0 : Math.max(0, Math.min(rows.length - 1, idx + delta));
		this.selectMessage(rows[nextIdx].id);
	}

	openSnoozeFor(id: string) {
		this.snoozeTargetId = id;
		this.snoozeOpen = true;
	}

	openTagDialogFor(id: string) {
		this.tagDialogTargetId = id;
		this.tagDialogOpen = true;
	}

	async mutate(id: string, action: TriageAction) {
		await this.inbox.mutate(id, action);
		if (this.isDemo) {
			if (this.inbox.mutationError) toast.error(this.inbox.mutationError);
			else if (action === 'OneClickUnsubscribe') toast.success('RFC 8058 Unsubscribe request sent.');
			else if (action === 'Archive') toast.success('Moved to Archive');
			else if (action === 'Trash') toast.success('Moved to Trash');
			else if (action === 'Star') toast.success('Starred message');
			else if (action === 'Unstar') toast.success('Removed star');
			else if (action === 'MarkRead') toast.success('Marked as read');
			else if (action === 'MarkUnread') toast.success('Marked as unread');
			else if (typeof action === 'object' && 'Tag' in action) {
				toast.success(`Added local tag "${action.Tag}"`);
			} else if (typeof action === 'object' && 'Snooze' in action) {
				toast.success('Message snoozed locally');
			}
		}
	}

	createRuleFor(row: MessageRowDto) {
		if (this.createRuleHook && !this.isDemo) {
			this.createRuleHook(row.id);
			return;
		}
		const { email } = parseSender(row.sender);
		const category =
			row.category_badge === 'Security' ||
			row.category_badge === 'Financial' ||
			row.category_badge === 'Shopping' ||
			row.category_badge === 'Newsletter' ||
			row.category_badge === 'Notifications' ||
			row.category_badge === 'Personal'
				? row.category_badge
				: 'Personal';
		appStore.addRuleFromMessage(email || row.sender, category);
		toast.success(`Deterministic rule created for ${email || row.sender}`);
	}

	async copyText(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			toast.success(`${label} copied to clipboard`);
		} catch {
			toast.success(`${label}: ${value}`);
		}
	}

	async applyShellScope(scope: ScopeFilter) {
		if (this.isDemo) {
			if (scope.kind === 'new_senders') {
				await this.inbox.selectReview(scope.status);
				return;
			}
			if (scope.kind === 'category') {
				await this.inbox.selectCategory(scope.category);
				return;
			}
			if (scope.kind === 'saved_view') {
				const found = appStore.savedViews.find((v) => v.id === scope.view_id);
				if (found) {
					await this.inbox.selectSavedView({
						...blankView(found.query, 'Lexical', 'GlobalAllMail'),
						id: found.id,
						name: found.name,
						revision: 1
					});
					return;
				}
			}
			if (scope.kind === 'folder') {
				this.activeAccountId = scope.account_id;
				const mapped: MailboxScope = {
					AccountView: {
						account_id: scope.account_id,
						view: scope.folder === 'INBOX' ? 'Inbox' : { Folder: scope.folder }
					}
				};
				await this.inbox.selectScope(mapped);
				return;
			}
			this.inbox.savedView = null;
			this.inbox.category = null;
			this.inbox.review = null;
			this.inbox.scope = undefined;
			await this.inbox.search(this.inbox.query, this.inbox.mode);
			return;
		}

		if (scope.kind === 'all') {
			await this.inbox.selectScope('GlobalAllMail');
			return;
		}
		if (scope.kind === 'folder') {
			this.activeAccountId = scope.account_id;
			await this.inbox.selectScope({
				AccountView: {
					account_id: scope.account_id,
					view: scope.folder === 'INBOX' ? 'Inbox' : { Folder: scope.folder }
				}
			});
			return;
		}
		if (scope.kind === 'inbox' || scope.kind === 'starred' || scope.kind === 'snoozed') {
			const accountId = this.activeAccountId ?? this.accounts[0]?.account_id;
			if (!accountId) {
				await this.inbox.selectScope('GlobalAllMail');
				return;
			}
			const view =
				scope.kind === 'inbox' ? 'Inbox' : scope.kind === 'starred' ? 'Starred' : 'Snoozed';
			await this.inbox.selectScope({ AccountView: { account_id: accountId, view } });
			return;
		}
		if (scope.kind === 'new_senders') {
			await this.inbox.selectReview(scope.status);
			return;
		}
		if (scope.kind === 'category') {
			await this.inbox.selectCategory(scope.category);
			return;
		}
		if (scope.kind === 'saved_view') {
			const list = (await this.inbox.savedTransport?.list().catch(() => [])) ?? [];
			const match = list.find((v) => v.id === scope.view_id);
			if (match) {
				await this.inbox.selectSavedView(match);
			} else {
				const fallback = appStore.savedViews.find((v) => v.id === scope.view_id);
				if (fallback) {
					await this.inbox.selectSavedView({
						...blankView(fallback.query, 'Lexical', 'GlobalAllMail'),
						id: fallback.id,
						name: fallback.name,
						revision: 1
					});
				}
			}
		}
	}

	clearFilters() {
		appStore.setScope({ kind: 'all' });
		void this.inbox.search('', this.inbox.mode);
	}

	ensureDemoDueDateMessage(messageId: string) {
		if (!this.demoBackend) return;
		const msg = demoRemindersBackend.findMessage(messageId);
		if (msg) {
			this.demoBackend.ensureMessage(msg);
		}
	}

	syncShellMetrics() {
		appStore.loadedCount = this.inbox.rows.length;
		appStore.unreadTotal = this.demoBackend
			? this.demoBackend.getMessages().filter((m) => m.folder === 'INBOX' && m.unread).length
			: this.inbox.rows.filter((r) => !r.is_read).length;
		appStore.embeddingErrors = this.inbox.totalEmbeddingErrors;
		appStore.semanticSearchEnabled = this.inbox.experimentalSemanticSearch;
		appStore.syncStatus = mapSyncStatus(this.inbox.syncStatus);
		if (this.demoBackend) {
			appStore.overdueAndDueTodayCount = countOverdueAndDueToday(demoRemindersBackend.getItems());
			for (const acct of appStore.accounts) {
				for (const folder of acct.folders) {
					const unread = this.demoBackend
						.getMessages()
						.filter(
							(m) => m.account_id === acct.account_id && m.folder === folder.id && m.unread
						).length;
					appStore.setFolderUnread(acct.account_id, folder.id, unread);
				}
			}
		}
	}

	setSearchMode(mode: SearchMode) {
		void this.inbox.search(this.inbox.query, mode);
	}
}

export function createDemoWorkspace(): InboxWorkspace {
	const demo = createDemoBackend();
	const inbox = new InboxController(demo.inboxTransport);
	const workspace = new InboxWorkspace({
		inbox,
		readerTransport: demo.readerTransport,
		settingsTransport: demo.settingsTransport,
		connected: true,
		isDemo: true,
		demo
	});
	return workspace;
}

class WorkspaceRegistry {
	current = $state<InboxWorkspace | null>(null);
	private fallbackDemo: InboxWorkspace | null = null;

	get active(): InboxWorkspace | null {
		if (this.current) return this.current;
		if (!nativeAvailable()) {
			if (!this.fallbackDemo) {
				this.fallbackDemo = createDemoWorkspace();
			}
			return this.fallbackDemo;
		}
		return null;
	}

	setActive(workspace: InboxWorkspace | null) {
		this.current = workspace;
	}
}

export const workspaceRegistry = new WorkspaceRegistry();

export function createLiveWorkspace(
	inbox = new InboxController(nativeTransport),
	options: Partial<Omit<WorkspaceOptions, 'inbox'>> = {}
): InboxWorkspace {
	return new InboxWorkspace({
		inbox,
		readerTransport: options.readerTransport ?? nativeReaderTransport,
		settingsTransport: options.settingsTransport ?? nativeSettingsTransport,
		connected: options.connected ?? true,
		isDemo: options.isDemo ?? false,
		accounts: options.accounts,
		dueCount: options.dueCount,
		createRule: options.createRule,
		demo: options.demo
	});
}
