import { i18n, type SupportedLocale } from '$lib/i18n/index.svelte';
import { nativeAvailable } from '$lib/inbox/transport';
import type {
	AlertDestination,
	AutomationRule,
	FolderId,
	MailAccount,
	MessageCategory,
	SavedView,
	ScopeFilter,
	SenderGovernanceEntry,
	SenderStatus,
	SyncStatus,
	ThemeMode
} from '$lib/types';
import {
	SHELL_ACCOUNT_FIXTURES,
	SHELL_ALERT_DESTINATION_FIXTURES,
	SHELL_RULE_FIXTURES,
	SHELL_SAVED_VIEW_FIXTURES,
	SHELL_SENDER_GOVERNANCE_FIXTURES
} from './app.fixtures';

const THEME_STORAGE_KEY = 'triage_mail_theme';

function getInitialTheme(): ThemeMode {
	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem(THEME_STORAGE_KEY);
		if (saved === 'light' || saved === 'dark') {
			return saved;
		}
	}
	if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
		if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
			return 'dark';
		}
	}
	return 'light';
}

class AppStore {
	theme = $state<ThemeMode>(getInitialTheme());
	syncStatus = $state<SyncStatus>('Idle');
	embeddingErrors = $state<number>(0);
	sidebarOpen = $state<boolean>(true);
	mobileDrawerOpen = $state<boolean>(false);
	commandOpen = $state<boolean>(false);
	semanticSearchEnabled = $state<boolean>(false);
	vaultUnlocked = $state<boolean>(true);
	privacyBlockRemoteImages = $state<boolean>(true);

	activeScope = $state<ScopeFilter>({ kind: 'inbox' });
	pendingSelectMessageId = $state<string | null>(null);
	pendingRuleSeedMessageId = $state<string | null>(null);
	loadedCount = $state<number>(0);
	unreadTotal = $state<number>(0);
	overdueAndDueTodayCount = $state<number>(0);
	private folderUnreadCounts = $state<Record<string, number>>({});
	private scopeChangeHook: ((scope: ScopeFilter) => void) | null = null;
	private syncTriggerHook: (() => void) | null = null;

	accounts = $state<MailAccount[]>(
		nativeAvailable() ? [] : structuredClone(SHELL_ACCOUNT_FIXTURES)
	);
	savedViews = $state<SavedView[]>(
		nativeAvailable() ? [] : structuredClone(SHELL_SAVED_VIEW_FIXTURES)
	);
	rules = $state<AutomationRule[]>(
		nativeAvailable() ? [] : structuredClone(SHELL_RULE_FIXTURES)
	);
	alertDestinations = $state<AlertDestination[]>(
		nativeAvailable() ? [] : structuredClone(SHELL_ALERT_DESTINATION_FIXTURES)
	);
	senderGovernance = $state<SenderGovernanceEntry[]>(
		nativeAvailable() ? [] : structuredClone(SHELL_SENDER_GOVERNANCE_FIXTURES)
	);

	get locale(): SupportedLocale {
		return i18n.locale;
	}

	setTheme(next: ThemeMode) {
		this.theme = next;
		if (typeof document !== 'undefined') {
			document.documentElement.classList.toggle('dark', next === 'dark');
		}
		if (typeof localStorage !== 'undefined') {
			try {
				localStorage.setItem(THEME_STORAGE_KEY, next);
			} catch {
				// Storage unavailable; in-memory theme persists.
			}
		}
	}

	syncDocumentTheme() {
		if (typeof document !== 'undefined') {
			document.documentElement.classList.toggle('dark', this.theme === 'dark');
		}
	}

	toggleTheme() {
		this.setTheme(this.theme === 'dark' ? 'light' : 'dark');
	}

	setLocale(next: SupportedLocale) {
		i18n.setLocale(next);
	}

	toggleSidebar() {
		this.sidebarOpen = !this.sidebarOpen;
	}

	setScope(scope: ScopeFilter) {
		this.activeScope = scope;
		this.scopeChangeHook?.(scope);
	}

	requestSelectMessage(messageId: string) {
		this.pendingSelectMessageId = messageId;
	}

	consumePendingSelectMessage(): string | null {
		const id = this.pendingSelectMessageId;
		this.pendingSelectMessageId = null;
		return id;
	}

	requestCreateRule(messageId: string) {
		this.pendingRuleSeedMessageId = messageId;
	}

	consumePendingRuleSeedMessage(): string | null {
		const id = this.pendingRuleSeedMessageId;
		this.pendingRuleSeedMessageId = null;
		return id;
	}

	registerShellHooks(hooks: {
		onScopeChange?: (scope: ScopeFilter) => void;
		onTriggerSync?: () => void;
	}) {
		this.scopeChangeHook = hooks.onScopeChange ?? null;
		this.syncTriggerHook = hooks.onTriggerSync ?? null;
	}

	setFolderUnread(accountId: string, folderId: FolderId, count: number) {
		this.folderUnreadCounts = {
			...this.folderUnreadCounts,
			[`${accountId}::${folderId}`]: count
		};
	}

	unreadForAccountFolder(accountId: string, folderId: FolderId): number {
		return this.folderUnreadCounts[`${accountId}::${folderId}`] ?? 0;
	}

	triggerSync() {
		if (this.syncTriggerHook) {
			this.syncTriggerHook();
			return;
		}
		if (nativeAvailable() || this.syncStatus === 'Syncing') return;
		this.syncStatus = 'Syncing';
		setTimeout(() => {
			this.syncStatus = 'Idle';
		}, 900);
	}

	addAccount(account: MailAccount) {
		this.accounts = [...this.accounts, account];
	}

	toggleRule(ruleId: string) {
		this.rules = this.rules.map((r) => (r.id === ruleId ? { ...r, enabled: !r.enabled } : r));
	}

	addRule(rule: Omit<AutomationRule, 'id' | 'matches_count'>) {
		const id = `rule_${Date.now()}`;
		this.rules = [{ ...rule, id, matches_count: 0 }, ...this.rules];
	}

	toggleAlertDestination(destId: string) {
		this.alertDestinations = this.alertDestinations.map((d) =>
			d.id === destId ? { ...d, enabled: !d.enabled } : d
		);
	}

	setSenderStatus(senderEmail: string, status: SenderStatus) {
		this.senderGovernance = this.senderGovernance.map((entry) =>
			entry.sender_email === senderEmail ? { ...entry, status } : entry
		);
	}

	addRuleFromMessage(senderEmail: string, category: MessageCategory) {
		const domain = senderEmail.split('@')[1] ?? senderEmail;
		this.addRule({
			name: `Auto-triage ${domain}`,
			enabled: true,
			match_sender_domain: domain,
			match_category: category,
			action_tag: category,
			action_archive: false
		});
	}
}

export const appStore = new AppStore();
