import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { Channel, invoke } from '@tauri-apps/api/core';
import { AccountController } from './controller.svelte';
import {
	formatBytes,
	overrideToSelect,
	quotaPercent,
	selectToOverride,
	type AccountSubscription
} from './subscriptions';
import { nativeAccountTransport } from './transport';
import type { AccountTransport } from './types';
import WdgAccountPlanBadge from './WdgAccountPlanBadge.svelte';
import WdgAccountPlanControl from './WdgAccountPlanControl.svelte';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
	Channel: class {
		onmessage = (_message: null) => {};
	}
}));

const GIB = 1024 ** 3;
const sub = (over: Partial<AccountSubscription> = {}): AccountSubscription => ({
	account_id: 'a1',
	effective_badge: 'PRO',
	badge_override: { mode: 'Auto' },
	inferred_badge: 'Pro',
	quota: { used_bytes: 5 * GIB, limit_bytes: 2048 * GIB },
	...over
});
async function settle() {
	for (let i = 0; i < 4; i++) {
		await tick();
		flushSync();
	}
}
afterEach(() => {
	document.body.replaceChildren();
	vi.mocked(invoke).mockReset();
});

it('maps override <-> select values and validates custom labels', () => {
	expect(overrideToSelect({ mode: 'Preset', value: 'Ultra' })).toBe('preset:Ultra');
	expect(selectToOverride('preset:Pro', '')).toEqual({ mode: 'Preset', value: 'Pro' });
	expect(selectToOverride('auto', '')).toEqual({ mode: 'Auto' });
	expect(selectToOverride('hidden', '')).toEqual({ mode: 'Hidden' });
	expect(selectToOverride('custom', '  ')).toBeNull();
	expect(selectToOverride('custom', 'x'.repeat(13))).toBeNull();
	expect(selectToOverride('custom', ' Work ')).toEqual({ mode: 'Custom', value: 'Work' });
	expect(selectToOverride('preset:Bogus', '')).toBeNull();
});

it('formats quota values', () => {
	expect(formatBytes(15 * GIB)).toBe('15.0 GB');
	expect(quotaPercent({ used_bytes: 5, limit_bytes: 10 })).toBe(50);
	expect(quotaPercent({ used_bytes: 5, limit_bytes: 0 })).toBe(0);
});

it('dispatches subscription IPC with camelCase arguments', async () => {
	vi.mocked(invoke).mockResolvedValue({});
	await nativeAccountTransport.subscriptions!();
	await nativeAccountTransport.setBadge!('a1', { mode: 'Hidden' });
	expect(vi.mocked(invoke).mock.calls).toEqual([
		['cmd_account_subscriptions'],
		['cmd_set_account_badge', { accountId: 'a1', overrideMode: { mode: 'Hidden' } }]
	]);
	void Channel;
});

it('controller loads subscriptions best-effort and applies badge updates', async () => {
	const updated = sub({ effective_badge: null, badge_override: { mode: 'Hidden' } });
	const transport = {
		list: vi.fn().mockResolvedValue([]),
		subscriptions: vi.fn().mockResolvedValue({ a1: sub() }),
		setBadge: vi.fn().mockResolvedValue(updated)
	} as unknown as AccountTransport;
	const controller = new AccountController(transport);
	await controller.load();
	await settle();
	expect(controller.subscriptions.a1.effective_badge).toBe('PRO');
	await controller.setBadge('a1', { mode: 'Hidden' });
	expect(controller.subscriptions.a1.effective_badge).toBeNull();

	vi.mocked(transport.setBadge!).mockRejectedValueOnce('InvalidRequest');
	await controller.setBadge('a1', { mode: 'Auto' });
	expect(controller.badgeError).toBe(true);
	vi.mocked(transport.subscriptions!).mockRejectedValueOnce('boom');
	await expect(controller.loadSubscriptions()).resolves.toBeUndefined();
});

it('renders badge pill with quota tooltip and hides when label is null', async () => {
	const host = document.body.appendChild(document.createElement('div'));
	const shown = mount(WdgAccountPlanBadge, {
		target: host,
		props: { label: 'PRO', quota: sub().quota }
	});
	await settle();
	const pill = host.querySelector('[data-testid="plan-badge"]');
	expect(pill?.textContent).toBe('PRO');
	expect(pill?.getAttribute('title')).toContain('/');
	unmount(shown);
	const hidden = mount(WdgAccountPlanBadge, { target: host, props: { label: null } });
	await settle();
	expect(host.querySelector('[data-testid="plan-badge"]')).toBeNull();
	unmount(hidden);
});

it('plan control shows quota bar and submits preset and custom overrides', async () => {
	const host = document.body.appendChild(document.createElement('div'));
	const onSetBadge = vi.fn().mockResolvedValue(undefined);
	const view = mount(WdgAccountPlanControl, {
		target: host,
		props: { subscription: sub(), onSetBadge }
	});
	await settle();
	const bar = host.querySelector('[role="progressbar"]');
	expect(bar?.getAttribute('aria-valuenow')).toBe('0');
	const select = host.querySelector('select') as HTMLSelectElement;
	select.value = 'preset:Ultra';
	select.dispatchEvent(new Event('change', { bubbles: true }));
	await settle();
	expect(onSetBadge).toHaveBeenCalledWith({ mode: 'Preset', value: 'Ultra' });

	select.value = 'custom';
	select.dispatchEvent(new Event('change', { bubbles: true }));
	await settle();
	const apply = host.querySelector('button') as HTMLButtonElement;
	expect(apply.disabled).toBe(true);
	const input = host.querySelector('input') as HTMLInputElement;
	input.value = 'Work';
	input.dispatchEvent(new Event('input', { bubbles: true }));
	await settle();
	apply.click();
	await settle();
	expect(onSetBadge).toHaveBeenLastCalledWith({ mode: 'Custom', value: 'Work' });
	unmount(view);
});
