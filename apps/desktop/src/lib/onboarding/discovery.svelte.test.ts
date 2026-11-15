import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import WdgAccountDiscoveryForm from './WdgAccountDiscoveryForm.svelte';
import { AccountController } from './controller.svelte';
import { DiscoveryController } from './discovery-controller.svelte';
import { providerPresets } from './presets';
import type { DiscoveredProvider, DiscoveryTransport } from './discovery';

const result: DiscoveredProvider = {
	domain: 'example.com',
	provider_name: 'Example',
	imap_host: 'imap.example.com',
	imap_port: 993,
	use_ssl: true,
	recommended_auth: 'AppPassword',
	oauth_endpoints: null,
	help_url: null,
	source: 'Ispdb'
};
function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}
function transport() {
	return {
		discover: vi.fn<DiscoveryTransport['discover']>().mockResolvedValue(result),
		probe: vi.fn<DiscoveryTransport['probe']>().mockResolvedValue(true)
	};
}
afterEach(() => {
	document.body.innerHTML = '';
	vi.useRealTimers();
});

it('debounces blur, keeps online lookup explicit and never probes on discovery', async () => {
	vi.useFakeTimers();
	const api = transport();
	const discovery = new DiscoveryController(api);
	discovery.blur('reader@example.com');
	discovery.blur('reader@example.org');
	await vi.advanceTimersByTimeAsync(200);
	expect(api.discover.mock.calls).toEqual([['reader@example.org', false]]);
	expect(api.probe).not.toHaveBeenCalled();
	await discovery.online('reader@example.com');
	expect(api.discover).toHaveBeenLastCalledWith('reader@example.com', true);
	discovery.dispose();
});

it('invalidates requests after address changes, manual edits, preset selection and teardown', async () => {
	for (const invalidate of [
		(d: DiscoveryController) => d.input(),
		(d: DiscoveryController) => d.protectCredentials(),
		(d: DiscoveryController) => {
			d.settings.host = 'manual.example.org';
			d.edit();
		},
		(d: DiscoveryController) => d.preset(providerPresets[2]),
		(d: DiscoveryController) => d.dispose()
	]) {
		const api = transport();
		const pending = deferred<DiscoveredProvider>();
		api.discover.mockReturnValueOnce(pending.promise);
		const discovery = new DiscoveryController(api);
		const lookup = discovery.lookup('reader@example.com', false);
		invalidate(discovery);
		const host = discovery.settings.host;
		pending.resolve(result);
		await lookup;
		expect(discovery.settings.host).toBe(host);
		expect(discovery.provider).toBeNull();
		expect(discovery.busy).toBe(false);
		discovery.dispose();
	}
});

it('preserves manual configuration on blur and ignores late probe feedback', async () => {
	vi.useFakeTimers();
	const api = transport();
	const discovery = new DiscoveryController(api);
	discovery.edit();
	discovery.settings.host = 'manual.example.org';
	discovery.blur('reader@example.com');
	await vi.advanceTimersByTimeAsync(200);
	expect(api.discover).not.toHaveBeenCalled();
	const pending = deferred<boolean>();
	api.probe.mockReturnValueOnce(pending.promise);
	const probe = discovery.probe();
	expect(api.probe).toHaveBeenCalledWith('manual.example.org');
	discovery.input();
	pending.resolve(true);
	await probe;
	expect(discovery.status).toBe('idle');
	discovery.dispose();
});

it('renders email first, requests online lookup only by click and confirms the displayed host', async () => {
	const api = transport();
	const target = document.createElement('div');
	document.body.append(target);
	const controller = new AccountController({
		list: async () => [],
		unlock: async () => {},
		add: async () => 'a',
		authorize: async () => 'a',
		cancel: async () => {}
	});
	const component = mount(WdgAccountDiscoveryForm, {
		target,
		props: { controller, discoveryTransport: api }
	});
	flushSync();
	const input = target.querySelector<HTMLInputElement>('input[name="email"]')!;
	expect(target.querySelector('input')).toBe(input);
	input.value = 'reader@example.com';
	input.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	expect(api.discover).not.toHaveBeenCalled();
	[...target.querySelectorAll('button')]
		.find((b) => b.textContent?.includes('Look up settings online'))!
		.click();
	await tick();
	await tick();
	flushSync();
	expect(api.discover).toHaveBeenCalledWith('reader@example.com', true);
	expect(api.probe).not.toHaveBeenCalled();
	[...target.querySelectorAll('button')]
		.find((b) => b.textContent?.includes('Confirm and test imap.example.com:993'))!
		.click();
	await tick();
	await tick();
	flushSync();
	expect(api.probe).toHaveBeenCalledWith('imap.example.com');
	expect(target.textContent).toContain('IMAP TLS connection confirmed');
	await unmount(component);
});

it('preserves email across Outlook PKCE setup and clears credentials when switching methods', async () => {
	const target = document.createElement('div');
	document.body.append(target);
	const authorize = vi.fn().mockResolvedValue('a');
	const controller = new AccountController({
		list: async () => [],
		unlock: async () => {},
		add: async () => 'a',
		authorize,
		cancel: async () => {}
	});
	const component = mount(WdgAccountDiscoveryForm, {
		target,
		props: { controller, discoveryTransport: transport() }
	});
	flushSync();
	const email = target.querySelector<HTMLInputElement>('input[name="email"]')!;
	email.value = 'reader@example.com';
	const oldPassword = target.querySelector<HTMLInputElement>('input[name="secret"]')!;
	oldPassword.value = 'synthetic-app-password';
	[...target.querySelectorAll('button')]
		.find((b) => b.textContent?.trim() === 'Outlook')!
		.click();
	flushSync();
	expect(oldPassword.value).toBe('');
	expect(email.value).toBe('reader@example.com');
	expect(authorize).not.toHaveBeenCalled();
	const client = target.querySelector<HTMLInputElement>('input[name="client"]')!;
	client.value = 'synthetic-client';
	target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
	expect(client.value).toBe('');
	expect(authorize).toHaveBeenCalledWith(
		expect.objectContaining({ auth_method: 'OAuth2PublicPkce' }),
		expect.objectContaining({ client_id: 'synthetic-client' }),
		expect.any(Function)
	);
	await tick();
	await tick();
	await unmount(component);
});

it('surfaces unresolved lookup feedback and preserves manual host input while editing', async () => {
	const api = transport();
	api.discover.mockResolvedValue({ ...result, imap_host: null, source: 'Unresolved' });
	const target = document.createElement('div');
	document.body.append(target);
	const controller = new AccountController({
		list: async () => [],
		unlock: async () => {},
		add: async () => 'a',
		authorize: async () => 'a',
		cancel: async () => {}
	});
	const component = mount(WdgAccountDiscoveryForm, {
		target,
		props: { controller, discoveryTransport: api }
	});
	flushSync();
	target.querySelector<HTMLInputElement>('input[name="email"]')!.value = 'reader@example.com';
	[...target.querySelectorAll('button')]
		.find((b) => b.textContent?.includes('Look up settings online'))!
		.click();
	await tick();
	await tick();
	flushSync();
	expect(target.textContent).toContain('No matching settings found');
	const host = target.querySelector<HTMLInputElement>('input[name="host"]')!;
	expect(host.value).toBe('');
	host.value = 'manual.example.org';
	host.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
	expect(host.value).toBe('manual.example.org');
	await unmount(component);
});

it('auto-fills Google OAuth endpoints when switching from default Gmail to BYO OAuth tab', async () => {
	const target = document.createElement('div');
	document.body.append(target);
	const authorize = vi.fn().mockResolvedValue('a');
	const controller = new AccountController({
		list: async () => [],
		unlock: async () => {},
		add: async () => 'a',
		authorize,
		cancel: async () => {}
	});
	const component = mount(WdgAccountDiscoveryForm, {
		target,
		props: { controller, discoveryTransport: transport() }
	});
	flushSync();
	const byoTab = [...target.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find((b) =>
		b.textContent?.includes('Your client credentials')
	)!;
	byoTab.click();
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
	await unmount(component);
});

it('imports Google Cloud Desktop JSON directly into transient credential inputs', async () => {
	const target = document.createElement('div');
	document.body.append(target);
	const authorize = vi.fn().mockResolvedValue('acct-google');
	const controller = new AccountController({
		list: async () => [],
		unlock: async () => {},
		add: async () => 'acct-google',
		authorize,
		cancel: async () => {}
	});
	const component = mount(WdgAccountDiscoveryForm, {
		target,
		props: { controller, discoveryTransport: transport() }
	});
	flushSync();
	const workspaceBtn = [...target.querySelectorAll<HTMLButtonElement>('button')].find(
		(b) => b.textContent?.trim() === 'Google Workspace'
	)!;
	workspaceBtn.click();
	flushSync();
	const fileInput = target.querySelector<HTMLInputElement>('input[type="file"]')!;
	const jsonPayload = JSON.stringify({
		installed: {
			client_id: 'synthetic-imported-client-id',
			client_secret: 'synthetic-imported-client-secret',
			auth_uri: 'https://accounts.google.com/o/oauth2/auth',
			token_uri: 'https://oauth2.googleapis.com/token'
		}
	});
	const file = new File([jsonPayload], 'client_secret_synthetic.json', {
		type: 'application/json'
	});
	Object.defineProperty(fileInput, 'files', { value: [file], configurable: true });
	fileInput.dispatchEvent(new Event('change', { bubbles: true }));
	const clientEl = target.querySelector<HTMLInputElement>('input[name="client"]')!;
	const secretEl = target.querySelector<HTMLInputElement>('input[name="secret"]')!;
	await vi.waitFor(() => {
		expect(clientEl.value).toBe('synthetic-imported-client-id');
		expect(secretEl.value).toBe('synthetic-imported-client-secret');
	});
	const email = target.querySelector<HTMLInputElement>('input[name="email"]')!;
	email.value = 'reader@example.com';
	target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
	expect(clientEl.value).toBe('');
	expect(secretEl.value).toBe('');
	expect(authorize).toHaveBeenCalledWith(
		expect.objectContaining({
			imap_host: 'imap.gmail.com',
			imap_port: 993,
			auth_method: 'OAuth2ByoCredentials'
		}),
		expect.objectContaining({
			authorization_endpoint: 'https://accounts.google.com/o/oauth2/auth',
			token_endpoint: 'https://oauth2.googleapis.com/token',
			scopes: 'https://mail.google.com/',
			client_id: 'synthetic-imported-client-id',
			client_secret: 'synthetic-imported-client-secret'
		}),
		expect.any(Function)
	);
	await unmount(component);
});
