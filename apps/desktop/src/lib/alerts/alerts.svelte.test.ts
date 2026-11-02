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

it('submits Telegram credentials only as transient input and resets consent after edits', async () => {
	const f = fixture();
	await settle();
	f.button('Edit').click();
	flushSync();
	const kind = f.target.querySelector('select')!;
	kind.value = 'Telegram';
	kind.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	expect(f.target.textContent).toContain('BotFather');
	expect(f.target.textContent).toContain('https://api.telegram.org');
	const [token, chat] = [...f.target.querySelectorAll<HTMLInputElement>('input[type=password]')];
	token.value = '123:synthetic';
	chat.value = '-123';
	const consent = f.target.querySelector<HTMLInputElement>('input[required][type=checkbox]')!;
	consent.checked = true;
	consent.dispatchEvent(new Event('change', { bubbles: true }));
	chat.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	expect(consent.checked).toBe(false);
	consent.checked = true;
	consent.dispatchEvent(new Event('change', { bubbles: true }));
	f.target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
	expect(token.value).toBe('');
	expect(chat.value).toBe('');
	await settle();
	const [destination, secret, confirmed] = vi.mocked(f.transport.save).mock.calls[0];
	expect(destination.kind).toBe('Telegram');
	expect(destination.public_origin).toBe('https://api.telegram.org');
	expect(secret).toEqual({ token: '123:synthetic', chat_id: '-123' });
	expect(confirmed).toBe(true);
	expect(JSON.stringify(destination)).not.toContain('synthetic');
	expect(JSON.stringify(destination)).not.toContain('-123');
	expect(f.transport.test).not.toHaveBeenCalled();
});
it('clears secret inputs when switching connectors and exposes no Discord bearer input', async () => {
	const f = fixture();
	await settle();
	f.button('Edit').click();
	flushSync();
	const old = [...f.target.querySelectorAll<HTMLInputElement>('input[type=password]')];
	old[0].value = 'https://hooks.example.com/private';
	old[1].value = 'synthetic-sensitive';
	const kind = f.target.querySelector('select')!;
	kind.value = 'Discord';
	kind.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	expect(old[0].value).toBe('');
	expect(old[1].value).toBe('');
	expect(f.target.querySelectorAll('input[type=password]')).toHaveLength(1);
	expect(f.target.textContent).toContain('Integrations');
	expect(f.target.textContent).toContain('https://discord.com');
	const url = f.target.querySelector<HTMLInputElement>('input[type=password]')!;
	url.value = 'https://discord.com/api/webhooks/123/synthetic';
	f.button('Cancel').click();
	flushSync();
	expect(url.value).toBe('');
	expect(f.transport.save).not.toHaveBeenCalled();
	expect(f.transport.test).not.toHaveBeenCalled();
});
it('rejects partial replacement of Telegram credentials before IPC', async () => {
	const f = fixture();
	await settle();
	f.button('Edit').click();
	flushSync();
	const kind = f.target.querySelector('select')!;
	kind.value = 'Telegram';
	kind.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	const token = f.target.querySelector<HTMLInputElement>('input[type=password]')!;
	token.value = '123:synthetic';
	f.target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(f.transport.save).not.toHaveBeenCalled();
	const chat = [...f.target.querySelectorAll<HTMLInputElement>('input[type=password]')][1];
	expect(chat.validationMessage).toContain('both');
});
