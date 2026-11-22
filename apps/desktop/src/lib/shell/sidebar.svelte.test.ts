import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { appStore } from '../stores/app.svelte';
import SidebarNav from './SidebarNav.svelte';

let cleanup = () => {};

beforeEach(() => {
	localStorage.clear();
	appStore.resetLocalCategories();
	appStore.collapsedAccounts = {};
	appStore.setScope({ kind: 'inbox' });
});

afterEach(() => {
	cleanup();
	cleanup = () => {};
	document.body.innerHTML = '';
	vi.restoreAllMocks();
});

function settle() {
	flushSync();
}

describe('SidebarNav account collapse and local category customization', () => {
	it('collapses and expands account options (folders and aliases) in the sidebar', () => {
		const account = appStore.accounts[0];
		expect(account).toBeTruthy();

		const target = document.createElement('div');
		document.body.appendChild(target);
		const comp = mount(SidebarNav, { target });
		cleanup = () => unmount(comp);
		settle();

		const toggleBtn = target.querySelector<HTMLButtonElement>(
			`button[aria-label*="${account.address}"]`
		);
		expect(toggleBtn).toBeTruthy();
		const accountContainer = toggleBtn!.parentElement!;
		expect(toggleBtn?.getAttribute('aria-expanded')).toBe('true');
		expect(accountContainer.textContent).toContain('INBOX');
		expect(accountContainer.textContent).toContain('Archive');

		toggleBtn?.click();
		settle();

		expect(toggleBtn?.getAttribute('aria-expanded')).toBe('false');
		expect(appStore.isAccountCollapsed(account.account_id)).toBe(true);
		expect(accountContainer.textContent).not.toContain('Archive');

		toggleBtn?.click();
		settle();

		expect(toggleBtn?.getAttribute('aria-expanded')).toBe('true');
		expect(appStore.isAccountCollapsed(account.account_id)).toBe(false);
		expect(accountContainer.textContent).toContain('Archive');
	});

	it('allows adding, renaming, removing, restoring, and resetting local categories', () => {
		const target = document.createElement('div');
		document.body.appendChild(target);
		const comp = mount(SidebarNav, { target });
		cleanup = () => unmount(comp);
		settle();

		// Open customization dialog
		const customizeBtn = target.querySelector<HTMLButtonElement>(
			'button[aria-label="Customize local categories"]'
		);
		expect(customizeBtn).toBeTruthy();
		customizeBtn?.click();
		settle();

		// Add custom category
		const addInput = document.body.querySelector<HTMLInputElement>(
			'input[aria-label="Add category"]'
		);
		expect(addInput).toBeTruthy();
		addInput!.value = 'Travel';
		addInput!.dispatchEvent(new Event('input', { bubbles: true }));
		addInput!.form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		settle();

		expect(target.textContent).toContain('Travel');

		// Rename built-in Security -> 2FA
		const renameSecBtn = document.body.querySelector<HTMLButtonElement>(
			'button[aria-label="Rename category Security"]'
		);
		expect(renameSecBtn).toBeTruthy();
		renameSecBtn?.click();
		settle();

		const renameInput = document.body.querySelector<HTMLInputElement>(
			'input[aria-label="Rename category"]'
		);
		expect(renameInput).toBeTruthy();
		renameInput!.value = '2FA';
		renameInput!.dispatchEvent(new Event('input', { bubbles: true }));
		renameInput!.form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		settle();

		expect(target.textContent).toContain('2FA');

		// Remove Newsletter
		const removeNewsletterBtn = document.body.querySelector<HTMLButtonElement>(
			'button[aria-label="Remove category Newsletter"]'
		);
		expect(removeNewsletterBtn).toBeTruthy();
		removeNewsletterBtn?.click();
		settle();

		expect(appStore.hiddenBuiltinCategories.map((c) => c.id)).toContain('Newsletter');

		// Restore defaults
		const restoreDefaultsBtn = [...document.body.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Restore defaults')
		);
		expect(restoreDefaultsBtn).toBeTruthy();
		restoreDefaultsBtn?.click();
		settle();

		expect(appStore.hiddenBuiltinCategories).toHaveLength(0);
		expect(target.textContent).toContain('Security');
		expect(target.textContent).toContain('Newsletter');
	});
});
