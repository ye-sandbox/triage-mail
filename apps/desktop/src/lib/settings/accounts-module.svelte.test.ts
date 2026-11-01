import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AccountsModule from './modules/AccountsModule.svelte';
import { AccountController } from '../onboarding/controller.svelte';
import type { AccountConfig, AccountTransport } from '../onboarding/types';

function createMockAccountTransport(accountsList: AccountConfig[] = []): AccountTransport {
	return {
		list: vi.fn(async () => [...accountsList]),
		add: vi.fn(async () => 'acc-new'),
		unlock: vi.fn(async () => {}),
		authorize: vi.fn(async () => 'acc-oauth'),
		cancel: vi.fn(async () => {})
	};
}

describe('AccountsModule', () => {
	let cleanupFns: Array<() => void> = [];

	afterEach(() => {
		for (const cleanup of cleanupFns) {
			cleanup();
		}
		cleanupFns = [];
		document.body.innerHTML = '';
	});

	it('renders connected accounts list and + Add account button', async () => {
		const sampleAccounts: AccountConfig[] = [
			{
				account_id: 'acc-1',
				email_display: 'test@example.com',
				imap_host: 'imap.example.com',
				imap_port: 993,
				auth_method: 'AppPassword'
			}
		];
		const transport = createMockAccountTransport(sampleAccounts);
		const controller = new AccountController(transport);
		await controller.load();

		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(AccountsModule, {
			target,
			props: {
				controller
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		expect(target.textContent).toContain('test@example.com');
		expect(target.textContent).toContain('imap.example.com:993');

		const addBtn = target.querySelector<HTMLButtonElement>('button');
		expect(addBtn?.textContent?.trim()).toContain('+ Add account');

		// Click + Add account
		addBtn?.click();
		flushSync();

		// Should render AccountForm and back button
		expect(target.querySelector('form')).not.toBeNull();
		const backBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Back to accounts')
		);
		expect(backBtn).toBeDefined();

		// Click back to accounts
		backBtn?.click();
		flushSync();

		expect(target.textContent).toContain('test@example.com');
		expect(target.querySelector('form')).toBeNull();
	});

	it('renders account form directly when no accounts are connected', async () => {
		const transport = createMockAccountTransport([]);
		const controller = new AccountController(transport);
		await controller.load();

		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(AccountsModule, {
			target,
			props: {
				controller
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		expect(target.querySelector('form')).not.toBeNull();
	});
});
