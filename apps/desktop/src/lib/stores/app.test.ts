import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ScopeFilter } from '$lib/types';
import { appStore } from './app.svelte';

describe('AppStore', () => {
	beforeEach(() => {
		localStorage.clear();
		document.documentElement.classList.remove('dark');
		appStore.setTheme('light');
		appStore.setLocale('en');
		appStore.sidebarOpen = true;
		appStore.registerShellHooks({});
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('toggles theme and synchronizes document dark class and localStorage', () => {
		expect(appStore.theme).toBe('light');
		expect(document.documentElement.classList.contains('dark')).toBe(false);

		appStore.toggleTheme();
		expect(appStore.theme).toBe('dark');
		expect(document.documentElement.classList.contains('dark')).toBe(true);
		expect(localStorage.getItem('triage_mail_theme')).toBe('dark');

		appStore.toggleTheme();
		expect(appStore.theme).toBe('light');
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		expect(localStorage.getItem('triage_mail_theme')).toBe('light');
	});

	it('delegates locale changes to i18n and tracks unread per account folder', () => {
		appStore.setLocale('pt-BR');
		expect(appStore.locale).toBe('pt-BR');

		appStore.setFolderUnread('acct_1', 'INBOX', 12);
		expect(appStore.unreadForAccountFolder('acct_1', 'INBOX')).toBe(12);
		expect(appStore.unreadForAccountFolder('acct_1', 'Archive')).toBe(0);
	});

	it('dispatches registered scope and sync hooks and manages automation state', () => {
		const observedScopes: ScopeFilter[] = [];
		let syncCalls = 0;

		appStore.registerShellHooks({
			onScopeChange: (scope) => observedScopes.push(scope),
			onTriggerSync: () => {
				syncCalls += 1;
			}
		});

		const nextScope: ScopeFilter = { kind: 'category', category: 'Financial' };
		appStore.setScope(nextScope);
		appStore.triggerSync();

		expect(appStore.activeScope).toEqual(nextScope);
		expect(observedScopes).toEqual([nextScope]);
		expect(syncCalls).toBe(1);

		appStore.toggleSidebar();
		expect(appStore.sidebarOpen).toBe(false);

		const initialRules = appStore.rules.length;
		appStore.addRuleFromMessage('billing@vendor.example.com', 'Financial');
		expect(appStore.rules.length).toBe(initialRules + 1);
		expect(appStore.rules[0]?.match_sender_domain).toBe('vendor.example.com');
	});

	it('seeds baseline demo shell badge counts immediately without mounting InboxSession', () => {
		expect(appStore.loadedCount).toBe(5);
		expect(appStore.unreadTotal).toBe(3);
		expect(appStore.overdueAndDueTodayCount).toBe(2);
		expect(appStore.unreadForAccountFolder('acct_work_01', 'INBOX')).toBe(1);
		expect(appStore.unreadForAccountFolder('acct_personal_02', 'INBOX')).toBe(2);
	});

	it('hydrates native shell counts from transports and ignores locked-vault errors', async () => {
		const prevLoaded = appStore.loadedCount;
		const prevUnread = appStore.unreadTotal;
		const prevDue = appStore.overdueAndDueTodayCount;
		const prevAccounts = appStore.accounts;
		const prevSavedViews = appStore.savedViews;
		const prevVault = appStore.vaultUnlocked;
		const prevShellPrefs = { ...appStore.shellPreferences };

		try {
			appStore.loadedCount = 0;
			appStore.unreadTotal = 0;
			appStore.overdueAndDueTodayCount = 0;
			appStore.accounts = [];
			appStore.setShellPreferences({ close_to_tray: true, minimize_to_tray: false });

			await appStore.hydrateNativeShellCounts({
				isNative: () => true,
				accountTransport: {
					list: vi.fn().mockRejectedValueOnce('VaultUnavailable')
				}
			});
			expect(appStore.loadedCount).toBe(0);

			const todayIso = new Date().toISOString().slice(0, 10);
			await appStore.hydrateNativeShellCounts({
				isNative: () => true,
				accountTransport: {
					list: vi.fn().mockResolvedValue([
						{
							account_id: 'acct_native_1',
							email_display: 'native@example.com',
							imap_host: 'imap.example.com',
							imap_port: 993,
							auth_method: 'AppPassword'
						}
					])
				},
				inboxTransport: {
					listFolders: vi.fn().mockResolvedValue([
						{
							account_id: 'acct_native_1',
							folder_name: 'INBOX',
							display_name: 'Inbox',
							role: 'inbox',
							delimiter: '/',
							is_selectable: true,
							total_messages: 12,
							unread_messages: 4
						}
					]),
					search: vi.fn().mockResolvedValue([
						{
							id: 'm1',
							account_id: 'acct_native_1',
							folder: 'INBOX',
							uid: 1,
							subject: 'Test',
							sender: 's@example.com',
							received_at_epoch_ms: 0,
							is_read: false,
							has_attachments: false,
							tags: [],
							folder_override: null,
							category_badge: null
						},
						{
							id: 'm2',
							account_id: 'acct_native_1',
							folder: 'INBOX',
							uid: 2,
							subject: 'Test 2',
							sender: 's@example.com',
							received_at_epoch_ms: 0,
							is_read: true,
							has_attachments: false,
							tags: [],
							folder_override: null,
							category_badge: null
						}
					]),
					savedViews: {
						list: vi.fn().mockResolvedValue([
							{
								version: 1,
								id: 'sv_1',
								revision: 1,
								name: 'Invoices',
								query: 'boleto',
								mode: 'Lexical',
								scope: 'GlobalAllMail'
							}
						]),
						save: vi.fn(),
						remove: vi.fn(),
						search: vi.fn()
					}
				},
				remindersTransport: {
					listDueDates: vi.fn().mockResolvedValue([
						{
							message_id: 'due_1',
							account_id: 'acct_native_1',
							subject: 'Bill',
							sender: 'billing@example.com',
							received_at_epoch_ms: 0,
							is_read: false,
							digitable_line: '123',
							amount_cents: 1000,
							due_date_iso: todayIso,
							is_dismissed: false
						}
					])
				},
				settingsTransport: {
					shellPreferences: vi.fn().mockResolvedValue({
						close_to_tray: false,
						minimize_to_tray: true
					})
				}
			});

			expect(appStore.accounts.map((a) => a.account_id)).toEqual(['acct_native_1']);
			expect(appStore.loadedCount).toBe(2);
			expect(appStore.unreadTotal).toBe(4);
			expect(appStore.unreadForAccountFolder('acct_native_1', 'INBOX')).toBe(4);
			expect(appStore.overdueAndDueTodayCount).toBe(1);
			expect(appStore.savedViews).toEqual([{ id: 'sv_1', name: 'Invoices', query: 'boleto' }]);
			expect(appStore.shellPreferences).toEqual({
				close_to_tray: false,
				minimize_to_tray: true
			});
		} finally {
			appStore.loadedCount = prevLoaded;
			appStore.unreadTotal = prevUnread;
			appStore.overdueAndDueTodayCount = prevDue;
			appStore.accounts = prevAccounts;
			appStore.savedViews = prevSavedViews;
			appStore.vaultUnlocked = prevVault;
			appStore.setShellPreferences(prevShellPrefs);
		}
	});
});
