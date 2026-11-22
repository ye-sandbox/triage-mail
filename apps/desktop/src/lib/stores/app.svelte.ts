import {
	DEFAULT_APPEARANCE,
	DENSITY_ROW_HEIGHT,
	normalizeAppearance,
	resolveEffectiveTheme,
	stepUiZoom
} from '$lib/appearance/palettes';
import type {
	AppearancePreferences,
	FontPreset,
	ListDensity,
	TextScale,
	ThemePalette,
	ThemeScheme,
	UiZoom
} from '$lib/appearance/types';
import { i18n, type SupportedLocale } from '$lib/i18n/index.svelte';
import { INBOX_FIXTURES } from '$lib/inbox/inbox.fixtures';
import { nativeAvailable, nativeTransport } from '$lib/inbox/transport';
import type { InboxTransport } from '$lib/inbox/types';
import type { AccountAliasMap } from '$lib/onboarding/aliases';
import { avatarToDataUrl } from '$lib/onboarding/avatar-controller.svelte';
import { nativeAvatarTransport } from '$lib/onboarding/avatar-transport';
import type { AvatarTransport } from '$lib/onboarding/avatar-types';
import type { AccountSubscription } from '$lib/onboarding/subscriptions';
import { nativeAccountTransport } from '$lib/onboarding/transport';
import type { AccountConfig, AccountTransport } from '$lib/onboarding/types';
import { createDueDateFixtures } from '$lib/reminders/due-dates.seed';
import { nativeRemindersTransport } from '$lib/reminders/transport';
import { countOverdueAndDueToday, type RemindersTransport } from '$lib/reminders/types';
import { nativeSettingsTransport } from '$lib/settings/transport';
import type { SettingsTransport, ShellPreferences } from '$lib/settings/types';
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
const APPEARANCE_STORAGE_KEY = 'triage_mail_appearance_v1';

export interface NativeShellHydrationDeps {
	isNative?: () => boolean;
	accountTransport?: Pick<AccountTransport, 'list' | 'subscriptions' | 'aliases'>;
	avatarTransport?: Pick<AvatarTransport, 'get'>;
	inboxTransport?: Pick<InboxTransport, 'listFolders' | 'search' | 'savedViews'>;
	remindersTransport?: Pick<RemindersTransport, 'listDueDates'>;
	settingsTransport?: Pick<SettingsTransport, 'shellPreferences'>;
}

function mapAccountToShellAccount(acct: AccountConfig): MailAccount {
	return {
		account_id: acct.account_id,
		display_name: acct.email_display,
		address: acct.email_display,
		imap_host: acct.imap_host,
		imap_port: acct.imap_port,
		tls_mode: acct.imap_port === 143 ? 'STARTTLS' : 'IMAPS',
		auth_method:
			acct.auth_method === 'LocalArchive'
				? 'LocalArchive'
				: acct.auth_method === 'AppPassword'
					? 'AppPassword-Vault'
					: 'OAuth2-PKCE',
		vault_locked: false,
		folders: [
			{ id: 'INBOX', label: 'INBOX' },
			{ id: 'Archive', label: 'Archive' },
			{ id: 'Trash', label: 'Trash' }
		]
	};
}

function buildDemoFolderUnreadCounts(): Record<string, number> {
	const counts: Record<string, number> = {};
	for (const acct of SHELL_ACCOUNT_FIXTURES) {
		for (const folder of acct.folders) {
			counts[`${acct.account_id}::${folder.id}`] = INBOX_FIXTURES.filter(
				(m) => m.account_id === acct.account_id && m.folder === folder.id && m.unread
			).length;
		}
	}
	return counts;
}

function getOsPrefersDark(): boolean {
	if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
		return window.matchMedia('(prefers-color-scheme: dark)').matches;
	}
	return false;
}

function getInitialTheme(): ThemeMode {
	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem(THEME_STORAGE_KEY);
		if (saved === 'light' || saved === 'dark') {
			return saved;
		}
	}
	return getOsPrefersDark() ? 'dark' : 'light';
}

function getInitialAppearance(): AppearancePreferences {
	const legacyTheme = getInitialTheme();
	if (typeof localStorage !== 'undefined') {
		try {
			const raw = localStorage.getItem(APPEARANCE_STORAGE_KEY);
			if (raw) {
				return normalizeAppearance(JSON.parse(raw) as Partial<AppearancePreferences>, legacyTheme);
			}
		} catch {
			// Ignore malformed persisted appearance JSON and fall back to defaults.
		}
	}
	return normalizeAppearance(null, legacyTheme);
}

class AppStore {
	osPrefersDark = $state<boolean>(getOsPrefersDark());
	appearance = $state<AppearancePreferences>(getInitialAppearance());
	theme = $derived<ThemeMode>(
		resolveEffectiveTheme(this.appearance.scheme, this.osPrefersDark)
	);
	rowHeight = $derived<number>(DENSITY_ROW_HEIGHT[this.appearance.density]);
	private schemeListenerBound = false;
	syncStatus = $state<SyncStatus>('Idle');
	embeddingErrors = $state<number>(0);
	sidebarOpen = $state<boolean>(true);
	mobileDrawerOpen = $state<boolean>(false);
	commandOpen = $state<boolean>(false);
	semanticSearchEnabled = $state<boolean>(false);
	vaultUnlocked = $state<boolean>(true);
	privacyBlockRemoteImages = $state<boolean>(true);
	shellPreferences = $state<ShellPreferences>({
		close_to_tray: true,
		minimize_to_tray: false
	});

	activeScope = $state<ScopeFilter>({ kind: 'inbox' });
	pendingSelectMessageId = $state<string | null>(null);
	pendingRuleSeedMessageId = $state<string | null>(null);
	loadedCount = $state<number>(nativeAvailable() ? 0 : INBOX_FIXTURES.length);
	unreadTotal = $state<number>(
		nativeAvailable()
			? 0
			: INBOX_FIXTURES.filter((m) => m.folder === 'INBOX' && m.unread).length
	);
	overdueAndDueTodayCount = $state<number>(
		nativeAvailable() ? 0 : countOverdueAndDueToday(createDueDateFixtures())
	);
	private folderUnreadCounts = $state<Record<string, number>>(
		nativeAvailable() ? {} : buildDemoFolderUnreadCounts()
	);
	private scopeChangeHook: ((scope: ScopeFilter) => void) | null = null;
	private syncTriggerHook: (() => void) | null = null;
	private hydrationGeneration = 0;

	accounts = $state<MailAccount[]>(
		nativeAvailable() ? [] : structuredClone(SHELL_ACCOUNT_FIXTURES)
	);
	private async loadAccountSubscriptions(
		transport: Pick<AccountTransport, "subscriptions">,
		generation: number
	): Promise<void> {
		if (!transport.subscriptions) return;
		try {
			const subscriptions = await transport.subscriptions();
			if (generation === this.hydrationGeneration) this.accountSubscriptions = subscriptions;
		} catch {
			// Best-effort: badges are decorative.
		}
	}

	private async loadAccountAliases(
		transport: Pick<AccountTransport, 'aliases'>,
		generation: number
	): Promise<void> {
		if (!transport.aliases) return;
		try {
			const aliases = await transport.aliases();
			if (generation === this.hydrationGeneration) this.accountAliases = aliases;
		} catch {
			// Best-effort: virtual alias mailboxes are optional.
		}
	}

	private async loadShellPreferences(
		transport: Pick<SettingsTransport, 'shellPreferences'>,
		generation: number
	): Promise<void> {
		if (!transport.shellPreferences) return;
		try {
			const prefs = await transport.shellPreferences();
			if (generation === this.hydrationGeneration) {
				this.setShellPreferences(prefs);
			}
		} catch {
			// Best-effort: retain default shell preferences if unavailable.
		}
	}

	accountSubscriptions = $state<Record<string, AccountSubscription>>({});
	accountAliases = $state<AccountAliasMap>({});
	accountAvatars = $state<Record<string, string>>({});

	setAccountAvatarSrc(accountId: string, src: string): void {
		if (src) {
			this.accountAvatars = { ...this.accountAvatars, [accountId]: src };
		} else if (accountId in this.accountAvatars) {
			const next = { ...this.accountAvatars };
			delete next[accountId];
			this.accountAvatars = next;
		}
	}

	private async loadAccountAvatars(
		accounts: AccountConfig[],
		transport: Pick<AvatarTransport, 'get'>,
		generation: number
	): Promise<void> {
		if (accounts.length === 0) return;
		const results = await Promise.allSettled(
			accounts.map(async (acct) => {
				const avatar = await transport.get(acct.account_id);
				return {
					accountId: acct.account_id,
					src: avatar ? avatarToDataUrl(avatar) : ''
				};
			})
		);
		if (generation !== this.hydrationGeneration) return;
		const next: Record<string, string> = {};
		for (const result of results) {
			if (result.status === 'fulfilled' && result.value.src) {
				next[result.value.accountId] = result.value.src;
			}
		}
		this.accountAvatars = next;
	}
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

	setShellPreferences(next: ShellPreferences) {
		this.shellPreferences = {
			close_to_tray: next.close_to_tray,
			minimize_to_tray: next.minimize_to_tray
		};
	}

	private persistAppearance() {
		if (typeof localStorage === 'undefined') return;
		try {
			localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify(this.appearance));
			localStorage.setItem(THEME_STORAGE_KEY, this.theme);
		} catch {
			// Storage unavailable; in-memory appearance persists.
		}
	}

	private bindOsSchemeListener() {
		if (this.schemeListenerBound || typeof window === 'undefined' || !window.matchMedia) return;
		this.schemeListenerBound = true;
		const media = window.matchMedia('(prefers-color-scheme: dark)');
		this.osPrefersDark = media.matches;
		media.addEventListener?.('change', (event) => {
			this.osPrefersDark = event.matches;
			this.syncDocumentTheme();
		});
	}

	setTheme(next: ThemeMode) {
		this.appearance = { ...this.appearance, scheme: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setScheme(next: ThemeScheme) {
		this.appearance = { ...this.appearance, scheme: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setPalette(next: ThemePalette) {
		this.appearance = { ...this.appearance, palette: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setFont(next: FontPreset) {
		this.appearance = { ...this.appearance, font: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setTextScale(next: TextScale) {
		this.appearance = { ...this.appearance, textScale: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setUiZoom(next: UiZoom) {
		this.appearance = { ...this.appearance, uiZoom: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	zoomIn() {
		this.setUiZoom(stepUiZoom(this.appearance.uiZoom, 1));
	}

	zoomOut() {
		this.setUiZoom(stepUiZoom(this.appearance.uiZoom, -1));
	}

	resetZoom() {
		this.setUiZoom(100);
	}

	setDensity(next: ListDensity) {
		this.appearance = { ...this.appearance, density: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setUnderlineLinks(next: boolean) {
		this.appearance = { ...this.appearance, underlineLinks: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	setReduceMotion(next: boolean) {
		this.appearance = { ...this.appearance, reduceMotion: next };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	resetAppearance() {
		this.appearance = { ...DEFAULT_APPEARANCE };
		this.syncDocumentTheme();
		this.persistAppearance();
	}

	syncDocumentTheme() {
		this.bindOsSchemeListener();
		if (typeof document === 'undefined') return;
		const root = document.documentElement;
		const effective = resolveEffectiveTheme(this.appearance.scheme, this.osPrefersDark);
		root.classList.toggle('dark', effective === 'dark');

		if (this.appearance.palette === 'default') delete root.dataset.palette;
		else root.dataset.palette = this.appearance.palette;

		if (this.appearance.palette === 'contrast') root.dataset.contrast = 'high';
		else delete root.dataset.contrast;

		if (this.appearance.font === 'default') delete root.dataset.font;
		else root.dataset.font = this.appearance.font;

		if (this.appearance.textScale === 100) {
			delete root.dataset.textScale;
			root.style.fontSize = '';
		} else {
			root.dataset.textScale = String(this.appearance.textScale);
			root.style.fontSize = `${this.appearance.textScale}%`;
		}

		if (this.appearance.uiZoom === 100) {
			delete root.dataset.uiZoom;
			root.style.removeProperty('--ui-zoom');
		} else {
			root.dataset.uiZoom = String(this.appearance.uiZoom);
			root.style.setProperty('--ui-zoom', String(this.appearance.uiZoom / 100));
		}

		if (this.appearance.density === 'default') delete root.dataset.density;
		else root.dataset.density = this.appearance.density;

		if (this.appearance.underlineLinks) root.dataset.underlineLinks = 'true';
		else delete root.dataset.underlineLinks;

		if (this.appearance.reduceMotion) root.dataset.reduceMotion = 'true';
		else delete root.dataset.reduceMotion;
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

	async hydrateNativeShellCounts(deps: NativeShellHydrationDeps = {}): Promise<void> {
		const isNative = deps.isNative ?? nativeAvailable;
		if (!isNative()) return;

		const generation = ++this.hydrationGeneration;
		const accountTransport = deps.accountTransport ?? nativeAccountTransport;
		const avatarTransport = deps.avatarTransport ?? nativeAvatarTransport;
		const inboxTransport = deps.inboxTransport ?? nativeTransport;
		const remindersTransport = deps.remindersTransport ?? nativeRemindersTransport;
		const settingsTransport = deps.settingsTransport ?? nativeSettingsTransport;

		let accountConfigs: AccountConfig[];
		try {
			accountConfigs = await accountTransport.list();
		} catch {
			return;
		}
		if (generation !== this.hydrationGeneration) return;

		this.vaultUnlocked = true;
		this.accounts = accountConfigs.map(mapAccountToShellAccount);
		void this.loadAccountSubscriptions(accountTransport, generation);
		void this.loadAccountAliases(accountTransport, generation);
		void this.loadAccountAvatars(accountConfigs, avatarTransport, generation);
		const shellPrefsPromise = this.loadShellPreferences(settingsTransport, generation);

		if (accountConfigs.length === 0) {
			this.loadedCount = 0;
			this.unreadTotal = 0;
			this.overdueAndDueTodayCount = 0;
			this.folderUnreadCounts = {};
			await shellPrefsPromise;
			return;
		}

		const [foldersResult, searchResult, dueDatesResult, savedViewsResult] =
			await Promise.allSettled([
				inboxTransport.listFolders
					? Promise.all(
							accountConfigs.map((acct) =>
								inboxTransport.listFolders!(acct.account_id).then((folders) => ({
									accountId: acct.account_id,
									folders
								}))
							)
						)
					: Promise.resolve([]),
				inboxTransport.search({
					query: '',
					mode: 'Lexical',
					limit: 200,
					offset: 0,
					scope: 'GlobalAllMail'
				}),
				remindersTransport.listDueDates(),
				inboxTransport.savedViews ? inboxTransport.savedViews.list() : Promise.resolve(null),
				shellPrefsPromise
			]);

		if (generation !== this.hydrationGeneration) return;

		let folderUnreadSum = 0;
		let hasFolderUnread = false;

		if (foldersResult.status === 'fulfilled') {
			for (const { accountId, folders } of foldersResult.value) {
				for (const folder of folders) {
					if (folder.unread_messages === null) continue;
					const targetFolder: FolderId | null =
						folder.role === 'inbox' || folder.folder_name.toUpperCase() === 'INBOX'
							? 'INBOX'
							: folder.role === 'archive' || folder.folder_name === 'Archive'
								? 'Archive'
								: folder.role === 'trash' || folder.folder_name === 'Trash'
									? 'Trash'
									: null;
					if (targetFolder) {
						this.setFolderUnread(accountId, targetFolder, folder.unread_messages);
						if (targetFolder === 'INBOX') {
							folderUnreadSum += folder.unread_messages;
							hasFolderUnread = true;
						}
					}
				}
			}
		}

		if (searchResult.status === 'fulfilled') {
			const rows = searchResult.value;
			this.loadedCount = rows.length;
			this.unreadTotal = hasFolderUnread
				? folderUnreadSum
				: rows.filter((r) => !r.is_read).length;
		} else if (hasFolderUnread) {
			this.unreadTotal = folderUnreadSum;
		}

		if (dueDatesResult.status === 'fulfilled') {
			this.overdueAndDueTodayCount = countOverdueAndDueToday(dueDatesResult.value);
		}

		if (savedViewsResult.status === 'fulfilled' && savedViewsResult.value) {
			this.savedViews = savedViewsResult.value.map((v) => ({
				id: v.id,
				name: v.name,
				query: v.query
			}));
		}
	}
}

export const appStore = new AppStore();
