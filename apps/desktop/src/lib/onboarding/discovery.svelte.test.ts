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
