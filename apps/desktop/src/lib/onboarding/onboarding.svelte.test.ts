import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { AccountController } from './controller.svelte';
import OnboardingView from './OnboardingView.svelte';
import DesktopApp from './DesktopApp.svelte';
import type { AccountConfig, AccountTransport } from './types';

vi.mock('../inbox/transport', () => ({
	nativeAvailable: () => false,
	nativeTransport: {
		search: async () => [],
		subscribe: async () => () => {},
		triage: async () => {}
	}
}));
const account: AccountConfig = {
	account_id: 'account-one',
	email_display: 'reader@example.com',
	imap_host: 'imap.example.com',
	imap_port: 993,
	auth_method: 'AppPassword'
};
function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}
function fixture() {
	const transport = {
		list: vi.fn<AccountTransport['list']>().mockResolvedValue([]),
		unlock: vi.fn<AccountTransport['unlock']>().mockResolvedValue(),
		add: vi.fn<AccountTransport['add']>().mockResolvedValue('account-one'),
		authorize: vi.fn<AccountTransport['authorize']>().mockResolvedValue('account-one'),
		cancel: vi.fn<AccountTransport['cancel']>().mockResolvedValue()
	};
	const controller = new AccountController(transport);
	const target = document.createElement('div');
	document.body.append(target);
	return { transport, controller, target };
}
async function settle() {
	for (let i = 0; i < 4; i++) {
		await tick();
		flushSync();
	}
}
function input(target: HTMLElement, name: string, value: string) {
	const element = target.querySelector<HTMLInputElement>(`input[name="${name}"]`)!;
	element.value = value;
	element.dispatchEvent(new Event('input', { bubbles: true }));
	return element;
}
function submit(target: HTMLElement) {
	target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
}
beforeEach(() => {
	vi.stubGlobal(
		'ResizeObserver',
		class {
			observe() {}
			disconnect() {}
			unobserve() {}
		}
	);
});
afterEach(() => {
	document.body.replaceChildren();
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

describe('account startup', () => {
	it('makes no IPC calls in a browser and ignores stale startup responses', async () => {
		const { controller, transport } = fixture();
		await controller.load(false);
		expect(controller.phase).toBe('browser');
		expect(transport.list).not.toHaveBeenCalled();
		const first = deferred<AccountConfig[]>();
		transport.list.mockReturnValueOnce(first.promise).mockResolvedValueOnce([account]);
		const old = controller.load();
		await controller.load();
		first.reject('VaultUnavailable');
		await old;
		expect(controller.phase).toBe('ready');
		expect(controller.accounts).toEqual([account]);
	});
	it('distinguishes locked vault from storage failure without echoing diagnostics', async () => {
		const { controller, transport } = fixture();
		transport.list
			.mockRejectedValueOnce('VaultUnavailable')
			.mockRejectedValueOnce('synthetic-private-diagnostic');
		await controller.load();
		expect(controller.phase).toBe('locked');
		expect(controller.error).toBe('');
		await controller.load();
		expect(controller.phase).toBe('error');
		expect(controller.error).not.toContain('synthetic-private');
	});
	it('clears passphrase on submission, prevents duplicate unlocks and permits retry', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'locked';
		const pending = deferred<void>();
		transport.unlock.mockReturnValueOnce(pending.promise);
		const component = mount(OnboardingView, {
			target,
			props: { controller, enterInbox: vi.fn() }
		});
		flushSync();
		const passphrase = input(target, 'passphrase', 'synthetic-passphrase');
		submit(target);
		expect(passphrase.value).toBe('');
		submit(target);
		expect(transport.unlock).toHaveBeenCalledTimes(1);
		pending.reject('synthetic-passphrase');
		await settle();
		expect(controller.error).not.toContain('synthetic-passphrase');
		input(target, 'passphrase', 'synthetic-retry');
		submit(target);
		await settle();
		expect(controller.phase).toBe('ready');
		await unmount(component);
	});
	it('renders onboarding only after account lookup and lets existing accounts open mail', async () => {
		const { transport, target } = fixture();
		const pending = deferred<AccountConfig[]>();
		transport.list.mockReturnValueOnce(pending.promise);
		const component = mount(DesktopApp, {
			target,
			props: { transport, available: () => true }
		});
		flushSync();
		expect(target.textContent).toContain('Opening local mail');
		expect(target.querySelector('input')).toBeNull();
		pending.resolve([account]);
		await settle();
		expect(target.textContent).toContain('Message actions');
		const button = [...target.querySelectorAll('button')].find(
			(b) => b.textContent === 'Accounts'
		)!;
		button.click();
		flushSync();
		expect(target.textContent).toContain('reader@example.com');
		expect(target.textContent).toContain('Connect an account');
		await unmount(component);
	});
	it('suppresses late updates after teardown', async () => {
		const { controller, transport } = fixture();
		const pending = deferred<AccountConfig[]>();
		transport.list.mockReturnValueOnce(pending.promise);
		const loading = controller.load();
		controller.dispose();
		pending.resolve([account]);
		await loading;
		expect(controller.accounts).toEqual([]);
	});
});

describe('account forms and network consent', () => {
	it('sends password only on submit and immediately clears the unbound input', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		const pending = deferred<string>();
		transport.add.mockReturnValue(pending.promise);
		const component = mount(OnboardingView, {
			target,
			props: { controller, enterInbox: vi.fn() }
		});
		flushSync();
		input(target, 'email', account.email_display);
		input(target, 'host', account.imap_host);
		const secret = input(target, 'secret', 'synthetic-password');
		expect(transport.add).not.toHaveBeenCalled();
		expect(transport.authorize).not.toHaveBeenCalled();
		submit(target);
		expect(transport.add).toHaveBeenCalledWith(
			{ ...account, account_id: '' },
			{ AppPassword: { secret: 'synthetic-password' } }
		);
		expect(secret.value).toBe('');
		expect(JSON.stringify(controller)).not.toContain('synthetic-password');
		submit(target);
		expect(transport.add).toHaveBeenCalledTimes(1);
		pending.reject('synthetic-password');
		await settle();
		expect(target.textContent).toContain('Account could not be saved');
		expect(target.textContent).not.toContain('synthetic-password');
		await unmount(component);
	});
	it('clears mode changes and teardown, uses Bridge credentials, and requests OAuth explicitly', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		const component = mount(OnboardingView, {
			target,
			props: { controller, enterInbox: vi.fn() }
		});
		flushSync();
		const old = input(target, 'secret', 'synthetic-old-password');
		const select = target.querySelector('select')!;
		select.value = 'ProtonBridgeLocal';
		select.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();
		expect(old.value).toBe('');
		input(target, 'email', account.email_display);
		input(target, 'secret', 'synthetic-bridge');
		submit(target);
		await settle();
		expect(transport.add).toHaveBeenCalledWith(
			expect.objectContaining({
				imap_host: '127.0.0.1',
				imap_port: 1143,
				auth_method: 'ProtonBridgeLocal'
			}),
			{ ProtonBridgeLocal: { bridge_password: 'synthetic-bridge' } }
		);
		select.value = 'OAuth2ByoCredentials';
		select.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();
		input(target, 'email', account.email_display);
		input(target, 'host', account.imap_host);
		input(target, 'authorize', 'https://auth.example.com/authorize');
		input(target, 'token', 'https://auth.example.com/token');
		input(target, 'scopes', 'imap offline_access');
		const client = input(target, 'client', 'synthetic-client');
		const secret = input(target, 'secret', 'synthetic-client-secret');
		const pending = deferred<string>();
		transport.authorize.mockImplementationOnce((_config, _authorization, ready) => {
			ready();
			return pending.promise;
		});
		expect(transport.authorize).not.toHaveBeenCalled();
		submit(target);
		expect(client.value).toBe('');
		expect(secret.value).toBe('');
		expect(transport.authorize).toHaveBeenCalledWith(
			expect.objectContaining({ auth_method: 'OAuth2ByoCredentials' }),
			expect.objectContaining({
				client_id: 'synthetic-client',
				client_secret: 'synthetic-client-secret'
			}),
			expect.any(Function)
		);
		expect(target.textContent).toContain('Finish authorization in your browser');
		[...target.querySelectorAll('button')]
			.find((b) => b.textContent === 'Cancel authorization')!
			.click();
		expect(transport.cancel).toHaveBeenCalledTimes(1);
		pending.reject('synthetic-code');
		await settle();
		input(target, 'secret', 'synthetic-teardown');
		await unmount(component);
		expect(secret.value).toBe('');
	});
	it('cancels pending OAuth on teardown and reconciles a successful commit racing cancellation', async () => {
		const { controller, transport } = fixture();
		const pending = deferred<string>();
		transport.authorize.mockImplementationOnce((_config, _authorization, ready) => {
			ready();
			return pending.promise;
		});
		const authorization = {
			authorization_endpoint: 'https://auth.example.com/authorize',
			token_endpoint: 'https://auth.example.com/token',
			scopes: 'imap',
			client_id: 'synthetic-client'
		};
		const operation = controller.add(
			{ ...account, auth_method: 'OAuth2PublicPkce' },
			{ authorization }
		);
		await controller.cancel();
		transport.list.mockResolvedValueOnce([account]);
		pending.resolve('account-one');
		await operation;
		expect(controller.accounts).toEqual([account]);
		const other = deferred<string>();
		transport.authorize.mockImplementationOnce((_config, _authorization, ready) => {
			ready();
			return other.promise;
		});
		const stopped = controller.add(
			{ ...account, auth_method: 'OAuth2PublicPkce' },
			{ authorization }
		);
		controller.dispose();
		expect(transport.cancel).toHaveBeenCalledTimes(2);
		other.reject('synthetic');
		await stopped;
		expect(controller.error).toBe('');
	});
});

it('defers cancellation until native registration and cancels a late acknowledgement after teardown', async () => {
	const { controller, transport } = fixture();
	const pending = deferred<string>();
	transport.authorize.mockReturnValueOnce(pending.promise);
	const operation = controller.add(
		{ ...account, auth_method: 'OAuth2PublicPkce' },
		{
			authorization: {
				authorization_endpoint: 'https://auth.example.com/authorize',
				token_endpoint: 'https://auth.example.com/token',
				scopes: 'imap',
				client_id: 'synthetic-client'
			}
		}
	);
	expect(controller.busy).toBe(true);
	expect(controller.authorizing).toBe(false);
	await controller.cancel();
	expect(transport.cancel).not.toHaveBeenCalled();
	controller.dispose();
	transport.authorize.mock.calls[0][2]();
	expect(transport.cancel).toHaveBeenCalledTimes(1);
	pending.reject('synthetic-cancelled');
	await operation;
	expect(controller.error).toBe('');
});
