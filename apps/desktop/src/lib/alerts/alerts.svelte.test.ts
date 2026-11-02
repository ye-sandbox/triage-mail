import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import AlertsModule from './AlertsModule.svelte';
import { newDestination, type AlertsTransport, type Destination } from './types';
import { SettingsFormGuard } from '../settings/guards';
import { i18n } from '../i18n/index.svelte';
let cleanup: (() => void) | undefined;
afterEach(() => {
	cleanup?.();
	document.body.innerHTML = '';
	i18n.locale = 'en';
});
async function settle() {
	for (let i = 0; i < 20; i++) await Promise.resolve();
	flushSync();
}
function fixture() {
	const destination: Destination = {
		...newDestination(),
		id: 'destination',
		name: 'Workflow',
		revision: 1,
		kind: 'Webhook',
		enabled: true,
		public_origin: 'https://hooks.example.com',
		fields: ['Subject']
	};
	const transport: AlertsTransport = {
		list: vi.fn(async () => [destination]),
		history: vi.fn(async () => []),
		save: vi.fn(async (d) => ({ ...d, revision: d.revision + 1 })),
		remove: vi.fn(async () => {}),
		test: vi.fn(async () => {}),
		replay: vi.fn(async () => {}),
		available: vi.fn(async () => true)
	};
	const target = document.createElement('div');
	document.body.append(target);
	const guard = new SettingsFormGuard();
	const component = mount(AlertsModule, {
		target,
		props: { transport, guard, selectedIds: ['message'] }
	});
	cleanup = () => void unmount(component);
	const button = (text: string) => {
		const b = [...target.querySelectorAll('button')].find((b) => b.textContent?.includes(text));
		if (!b) throw Error(text);
		return b;
	};
	return { target, guard, transport, button, destination };
}
it('requires separate confirmation for tests and replay and sends the confirmed revision', async () => {
	const f = fixture();
	await settle();
	expect(f.transport.test).not.toHaveBeenCalled();
	f.button('Test delivery').click();
	flushSync();
	expect(f.transport.test).not.toHaveBeenCalled();
	f.button('Confirm delivery').click();
	await settle();
	expect(f.transport.test).toHaveBeenCalledWith('destination', 1, true);
	f.button('Replay selected').click();
	flushSync();
	expect(f.transport.replay).not.toHaveBeenCalled();
	f.button('Confirm delivery').click();
	await settle();
	expect(f.transport.replay).toHaveBeenCalledWith(['message'], 'destination', 1, true);
});
it('clears transient secret inputs and keeps secrets out of destination metadata', async () => {
	const f = fixture();
	await settle();
	f.button('Edit').click();
	flushSync();
	expect(f.guard.dirty).toBe(true);
	const inputs = [...f.target.querySelectorAll<HTMLInputElement>('input[type=password]')];
	inputs[0].value = 'https://hooks.example.com/private-hook';
	inputs[1].value = 'synthetic-sensitive';
	const consent = f.target.querySelector<HTMLInputElement>('input[required][type=checkbox]')!;
	consent.checked = true;
	consent.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	f.target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
	expect(inputs[0].value).toBe('');
	expect(inputs[1].value).toBe('');
	await settle();
	expect(f.transport.save).toHaveBeenCalledWith(
		f.destination,
		{
			url: 'https://hooks.example.com/private-hook',
			bearer: 'synthetic-sensitive'
		},
		true
	);
	expect(JSON.stringify(vi.mocked(f.transport.save).mock.calls[0][0])).not.toContain(
		'private-hook'
	);
	expect(f.guard.dirty).toBe(false);
});
it('renders paired Portuguese labels without sending on load', async () => {
	i18n.locale = 'pt-BR';
	const f = fixture();
	await settle();
	expect(f.target.textContent).toContain('Alertas');
	expect(f.target.textContent).toContain('Histórico de execução');
	expect(f.transport.test).not.toHaveBeenCalled();
});
