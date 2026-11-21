import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { AccountController } from './controller.svelte';
import { MAX_ACCOUNT_ALIASES, validateAlias, type AccountAlias } from './aliases';
import type { AccountTransport } from './types';
import WdgAccountAliasesControl from './WdgAccountAliasesControl.svelte';

async function settle() {
	for (let i = 0; i < 4; i++) {
		await tick();
		flushSync();
	}
}
afterEach(() => document.body.replaceChildren());

it('validates, normalizes and rejects duplicates and overflow', () => {
	expect(validateAlias(' Billing@Example.COM ', ' Billing ', [])).toEqual({
		alias: { address: 'billing@example.com', label: 'Billing' }
	});
	expect(validateAlias('a@example.com', '', [])).toEqual({ alias: { address: 'a@example.com' } });
	for (const bad of ['', 'nope', 'Name <a@example.com>', 'a@localhost', 'a b@example.com'])
		expect(validateAlias(bad, '', [])).toEqual({ issue: 'invalid' });
	expect(validateAlias('a@example.com', 'x'.repeat(81), [])).toEqual({ issue: 'invalid' });
	expect(validateAlias('a@example.com', 'bad\nlabel', [])).toEqual({ issue: 'invalid' });
	expect(validateAlias('A@example.com', '', [{ address: 'a@example.com' }])).toEqual({
		issue: 'duplicate'
	});
	const full = Array.from({ length: MAX_ACCOUNT_ALIASES }, (_, i) => ({
		address: `a${i}@example.com`
	}));
	expect(validateAlias('z@example.com', '', full)).toEqual({ issue: 'limit' });
});

it('controller persists the full list, drops empty accounts and flags failures', async () => {
	const setAliases = vi
		.fn<NonNullable<AccountTransport['setAliases']>>()
		.mockImplementation(async (_id, list) => list);
	const controller = new AccountController({
		list: async () => [],
		aliases: async () => ({ a1: [{ address: 'x@example.com' }] }),
		setAliases
	} as unknown as AccountTransport);
	await controller.loadAliases();
	expect(controller.aliases.a1).toHaveLength(1);
	expect(await controller.setAliases('a1', [])).toBe(true);
	expect(controller.aliases.a1).toBeUndefined();
	setAliases.mockRejectedValueOnce('InvalidRequest');
	expect(await controller.setAliases('a1', [{ address: 'y@example.com' }])).toBe(false);
	expect(controller.aliasError).toBe(true);
});

it('component adds and removes aliases through onSave', async () => {
	const onSave = vi.fn<(a: AccountAlias[]) => Promise<boolean>>().mockResolvedValue(true);
	const target = document.createElement('div');
	document.body.append(target);
	const component = mount(WdgAccountAliasesControl, {
		target,
		props: { accountId: 'a1', aliases: [{ address: 'old@example.com' }], onSave }
	});
	flushSync();
	const [address] = Array.from(target.querySelectorAll('input'));
	address.value = 'bad';
	address.dispatchEvent(new Event('input', { bubbles: true }));
	target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
	await settle();
	expect(onSave).not.toHaveBeenCalled();
	expect(target.querySelector('[role="alert"]')).not.toBeNull();

	address.value = 'New@Example.com';
	address.dispatchEvent(new Event('input', { bubbles: true }));
	target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
	await settle();
	expect(onSave).toHaveBeenLastCalledWith([
		{ address: 'old@example.com' },
		{ address: 'new@example.com' }
	]);

	target.querySelector<HTMLButtonElement>('ul button')!.click();
	await settle();
	expect(onSave).toHaveBeenLastCalledWith([]);
	await unmount(component);
});
