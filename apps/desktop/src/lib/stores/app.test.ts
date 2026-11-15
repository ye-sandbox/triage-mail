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
});
