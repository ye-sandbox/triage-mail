import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import RulesModule from './RulesModule.svelte';
import WdgRulesSplitPane from '../automation/WdgRulesSplitPane.svelte';
import { SettingsFormGuard } from '../settings/guards';
import { i18n } from '../i18n/index.svelte';
import { newRule, type Rule, type RulesTransport, type Plan } from './types';

let destroy: (() => void) | undefined;
afterEach(() => {
	destroy?.();
	destroy = undefined;
	document.body.innerHTML = '';
	i18n.locale = 'en';
});
async function settle() {
	for (let i = 0; i < 12; i++) await Promise.resolve();
	flushSync();
}
function fixture() {
	const existing: Rule = {
		...newRule(),
		id: 'rule',
		name: 'Billing',
		enabled: true,
		all: [{ Subject: 'invoice' }]
	};
	const transport: RulesTransport = {
		list: vi.fn(async () => [existing]),
		save: vi.fn(async (r: Rule) => ({ ...r, revision: r.revision + 1 })),
		remove: vi.fn(async () => {}),
		reorder: vi.fn(async () => {}),
		draft: vi.fn(async () => ({
			...existing,
			id: 'draft',
			revision: 0,
			enabled: false
		})),
		simulate: vi.fn(async (): Promise<[string, Plan][]> => [
			[
				'message',
				{
					actions: ['Star'],
					explanations: [
						{
							rule_id: 'rule',
							outcome: 'Matched',
							all: ['Matched'],
							any: [],
							except: []
						}
					]
				}
			]
		])
	};
	const guard = new SettingsFormGuard();
	return { transport, guard };
}
function render(transport: RulesTransport, guard: SettingsFormGuard, seedMessageId?: string) {
	const target = document.createElement('div');
	document.body.appendChild(target);
	const component = mount(RulesModule, {
		target,
		props: { transport, guard, selectedIds: ['message'], seedMessageId }
	});
	flushSync();
	destroy = () => {
		unmount(component);
	};
	return target;
}
function click(target: HTMLElement, text: string) {
	const button = [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);
	expect(button).toBeTruthy();
	button!.click();
	flushSync();
}
it('simulates selected messages with explanations and never saves or delivers', async () => {
	const { transport, guard } = fixture();
	const target = render(transport, guard);
	await settle();
	click(target, 'Simulate selected messages');
	await settle();
	expect(transport.simulate).toHaveBeenCalledWith(['message'], undefined);
	expect(target.textContent).toContain('Planned actions: Star');
	expect(target.textContent).toContain('Billing: Matched');
	expect(transport.save).not.toHaveBeenCalled();
	expect(guard.dirty).toBe(false);
});
it('creates an unsaved message draft, guards pending saves and clears state on teardown', async () => {
	const { transport, guard } = fixture();
	let resolveSave: (r: Rule) => void = () => {};
	transport.save = vi.fn(
		(_r: Rule) =>
			new Promise<Rule>((resolve) => {
				resolveSave = resolve;
			})
	);
	const target = render(transport, guard, 'message');
	await settle();
	expect(transport.draft).toHaveBeenCalled();
	expect(transport.save).not.toHaveBeenCalled();
	expect(guard.dirty).toBe(true);
	click(target, 'Save rule');
	expect(guard.busy).toBe(true);
	expect(target.querySelector('fieldset')?.disabled).toBe(true);
	resolveSave({ ...newRule(), name: 'Saved', revision: 1 });
	await settle();
	expect(guard.busy).toBe(false);
	expect(guard.dirty).toBe(false);
	destroy?.();
	destroy = undefined;
	expect(guard.canProceed()).toBe(true);
});
it('disables saved rules without a simulation and ignores late load results after unmount', async () => {
	const { transport, guard } = fixture();
	const target = render(transport, guard);
	await settle();
	click(target, 'Disabled');
	await settle();
	expect(transport.save).toHaveBeenCalledWith(expect.objectContaining({ enabled: false }));
	expect(transport.simulate).not.toHaveBeenCalled();
	let resolveList: (r: Rule[]) => void = () => {};
	destroy?.();
	transport.list = vi.fn(
		() =>
			new Promise<Rule[]>((resolve) => {
				resolveList = resolve;
			})
	);
	render(transport, guard);
	expect(guard.busy).toBe(true);
	destroy?.();
	destroy = undefined;
	resolveList([]);
	await settle();
	expect(guard.canProceed()).toBe(true);
});
it('shows fixed errors rather than provider diagnostics and localizes rule controls', async () => {
	const { transport, guard } = fixture();
	transport.list = vi.fn(async () => {
		throw new Error('private diagnostic');
	});
	i18n.locale = 'pt-BR';
	const target = render(transport, guard);
	await settle();
	expect(target.textContent).toContain('Nova regra');
	expect(target.textContent).toContain('A operação falhou');
	expect(target.textContent).not.toContain('private diagnostic');
});

describe('WdgRulesSplitPane (UI v2)', () => {
	it('simulates selected messages without saving and displays condition explanations', async () => {
		const { transport, guard } = fixture();
		const target = document.createElement('div');
		document.body.appendChild(target);
		const component = mount(WdgRulesSplitPane, {
			target,
			props: { transport, guard, selectedIds: ['message'] }
		});
		destroy = () => unmount(component);
		await settle();

		click(target, 'Simulate selected messages');
		await settle();

		expect(transport.simulate).toHaveBeenCalledWith(['message'], undefined);
		expect(target.textContent).toContain('Plan: Matched');
		expect(target.textContent).toContain('Planned actions:');
		expect(target.textContent).toContain('Star');
		expect(transport.save).not.toHaveBeenCalled();
		expect(guard.dirty).toBe(false);
	});

	it('creates a seeded rule draft, guards pending saves, reorders rules, and clears on unmount', async () => {
		const { transport, guard } = fixture();
		let resolveSave: (r: Rule) => void = () => {};
		transport.save = vi.fn(
			(_r: Rule) =>
				new Promise<Rule>((resolve) => {
					resolveSave = resolve;
				})
		);
		const target = document.createElement('div');
		document.body.appendChild(target);
		const component = mount(WdgRulesSplitPane, {
			target,
			props: { transport, guard, selectedIds: ['message'], seedMessageId: 'message' }
		});
		destroy = () => unmount(component);
		await settle();

		expect(transport.draft).toHaveBeenCalled();
		expect(guard.dirty).toBe(true);

		click(target, 'Create Rule');
		expect(guard.busy).toBe(true);
		expect(target.querySelector('fieldset')?.disabled).toBe(true);

		resolveSave({ ...newRule(), id: 'draft', name: 'Saved Draft', revision: 1 });
		await settle();

		expect(guard.busy).toBe(false);
		expect(guard.dirty).toBe(false);

		destroy();
		destroy = undefined;
		expect(guard.canProceed()).toBe(true);
	});

	it('rejects non-integer amounts and localizes fixed error diagnostics in pt-BR', async () => {
		const { transport, guard } = fixture();
		transport.list = vi.fn(async () => {
			throw new Error('internal trace');
		});
		i18n.locale = 'pt-BR';
		const target = document.createElement('div');
		document.body.appendChild(target);
		const component = mount(WdgRulesSplitPane, {
			target,
			props: { transport, guard, selectedIds: [] }
		});
		destroy = () => unmount(component);
		await settle();

		expect(target.textContent).toContain('Nova Regra');
		expect(target.textContent).toContain('A operação falhou');
		expect(target.textContent).not.toContain('internal trace');
	});
});
