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
	it('opens account management through settings while keeping existing mail mounted', async () => {
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
		expect(target.textContent).toContain('Loaded Messages');
		const listbox = target.querySelector('[role="listbox"][aria-label="Messages"]');
		expect(listbox).not.toBeNull();
		expect([...target.querySelectorAll('button')].some((b) => b.textContent === 'Accounts')).toBe(false);
		expect(
			[...target.querySelectorAll('button')].filter((b) => b.textContent?.trim() === 'Settings')
		).toHaveLength(1);
		target.querySelector<HTMLButtonElement>('button[aria-label="Settings"]')!.click();
		await settle();
		target.querySelector<HTMLButtonElement>('#tab-accounts')!.click();
		flushSync();
		const dialog = target.querySelector('[role="dialog"]')!;
		expect(dialog.textContent).toContain('reader@example.com');
		expect(dialog.textContent).toContain('Add account');
		expect(target.querySelector('[role="listbox"][aria-label="Messages"]')).toBe(listbox);
		dialog.querySelector<HTMLButtonElement>('button[aria-label="Close settings"]')!.click();
		flushSync();
		expect(target.querySelector('[role="dialog"]')).toBeNull();
		expect(target.querySelector('[role="listbox"][aria-label="Messages"]')).toBe(listbox);
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

describe('onboarding help guide and language selection', () => {
	it('switches app language via Apple-style segmented selector', async () => {
		const { controller, target } = fixture();
		controller.phase = 'ready';
		const component = mount(OnboardingView, {
			target,
			props: { controller, enterInbox: vi.fn() }
		});
		flushSync();

		expect(target.textContent).toContain('Connect an account');
		expect(target.textContent).toContain('Your accounts');

		const ptButton = [...target.querySelectorAll('button')].find(
			(b) => b.getAttribute('aria-label') === 'Português'
		);
		expect(ptButton).toBeDefined();
		ptButton!.click();
		flushSync();

		expect(target.textContent).toContain('Conectar uma conta');
		expect(target.textContent).toContain('Suas contas');

		const enButton = [...target.querySelectorAll('button')].find(
			(b) => b.getAttribute('aria-label') === 'English'
		);
		expect(enButton).toBeDefined();
		enButton!.click();
		flushSync();

		expect(target.textContent).toContain('Connect an account');
		await unmount(component);
	});

	it('toggles interactive sign-in help guide and displays method tabs with step-by-step playbooks', async () => {
		const { controller, target } = fixture();
		controller.phase = 'ready';
		const component = mount(OnboardingView, {
			target,
			props: { controller, enterInbox: vi.fn() }
		});
		flushSync();

		const toggleButton = target.querySelector<HTMLButtonElement>(
			'button[aria-controls="account-help-content"]'
		);
		expect(toggleButton).toBeDefined();
		expect(toggleButton!.getAttribute('aria-expanded')).toBe('false');

		toggleButton!.click();
		flushSync();

		expect(toggleButton!.getAttribute('aria-expanded')).toBe('true');
		expect(target.textContent).toContain('Two-Factor Authentication');
		expect(target.textContent).toContain('Gmail, Google Workspace');
		expect(target.textContent).toContain('What to fill in the form below:');
		expect(target.textContent).toContain('Step-by-step: how to obtain your credentials:');
		expect(target.textContent).toContain('myaccount.google.com/apppasswords');
		// INVARIANT: When Gmail is selected by default, other provider playbooks are filtered out
		expect(target.textContent).not.toContain('appleid.apple.com');
		expect(target.textContent).not.toContain('Settings -> Privacy & Security -> Integrations');

		const tabs = [...target.querySelectorAll<HTMLButtonElement>('button[role="tab"]')];
		expect(tabs).toHaveLength(4);

		tabs[1].click();
		flushSync();
		expect(target.textContent).toContain('portal.azure.com');
		expect(target.textContent).toContain('Application (client) ID');

		tabs[2].click();
		flushSync();
		expect(target.textContent).toContain('console.cloud.google.com');
		expect(target.textContent).toContain('Desktop app');

		tabs[3].click();
		flushSync();
		expect(target.textContent).toContain('Mailbox details / Configuration');

		await unmount(component);
	});

	it('applies provider presets to pre-fill host, port and authentication method', async () => {
		const { controller, target } = fixture();
		controller.phase = 'ready';
		const component = mount(OnboardingView, {
			target,
			props: { controller, enterInbox: vi.fn() }
		});
		flushSync();

		const buttons = [...target.querySelectorAll('button')];
		const gmailPreset = buttons.find((b) => b.textContent?.trim() === 'Gmail');
		const gmailOAuthPreset = buttons.find((b) => b.textContent?.trim() === 'Gmail OAuth');
		const icloudPreset = buttons.find((b) => b.textContent?.trim() === 'iCloud');
		const protonPreset = buttons.find((b) => b.textContent?.trim() === 'Proton');
		expect(gmailPreset).toBeDefined();
		expect(gmailOAuthPreset).toBeDefined();
		expect(icloudPreset).toBeDefined();
		expect(protonPreset).toBeDefined();

		// INVARIANT: Gmail is selected by default on initial render
		expect(gmailPreset!.getAttribute('aria-pressed')).toBe('true');
		expect(icloudPreset!.getAttribute('aria-pressed')).toBe('false');

		// Toggle help guide open and check that switching presets switches the displayed playbook
		const toggleButton = target.querySelector<HTMLButtonElement>(
			'button[aria-controls="account-help-content"]'
		)!;
		toggleButton.click();
		flushSync();
		expect(target.textContent).toContain('myaccount.google.com/apppasswords');
		expect(target.textContent).not.toContain('appleid.apple.com');

		icloudPreset!.click();
		flushSync();
		expect(gmailPreset!.getAttribute('aria-pressed')).toBe('false');
		expect(icloudPreset!.getAttribute('aria-pressed')).toBe('true');
		expect(target.textContent).toContain('appleid.apple.com');
		expect(target.textContent).not.toContain('myaccount.google.com/apppasswords');

		gmailPreset!.click();
		flushSync();
		const hostInput = target.querySelector<HTMLInputElement>('input[name="host"]')!;
		const portInput = target.querySelector<HTMLInputElement>('input[name="port"]')!;
		expect(hostInput.value).toBe('imap.gmail.com');
		expect(portInput.value).toBe('993');

		gmailOAuthPreset!.click();
		flushSync();
		expect(target.querySelector<HTMLInputElement>('input[name="authorize"]')!.value).toBe(
			'https://accounts.google.com/o/oauth2/v2/auth'
		);
		expect(target.querySelector<HTMLInputElement>('input[name="token"]')!.value).toBe(
			'https://oauth2.googleapis.com/token'
		);
		expect(target.querySelector<HTMLInputElement>('input[name="scopes"]')!.value).toBe(
			'https://mail.google.com/'
		);

		protonPreset!.click();
		flushSync();
		const bridgeHost = target.querySelector<HTMLInputElement>('input[name="host"]')!;
		const bridgePort = target.querySelector<HTMLInputElement>('input[name="port"]')!;
		expect(bridgeHost.value).toBe('127.0.0.1');
		expect(bridgePort.value).toBe('1143');

		await unmount(component);
	});
});
