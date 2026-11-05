import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import BackupForm from './BackupForm.svelte';
import PortableForm from './PortableForm.svelte';
import { SettingsFormGuard } from '../settings/guards';
import type { TransferTransport } from './transport';
import { i18n } from '../i18n/index.svelte';
let cleanup = () => {};
afterEach(() => {
	cleanup();
	document.body.innerHTML = '';
	i18n.locale = 'en';
});
async function settle() {
	for (let i = 0; i < 16; i++) await Promise.resolve();
	flushSync();
}
function transport(): TransferTransport {
	return {
		choose: vi.fn(async () => '/tmp/synthetic-backup.tmb'),
		backup: vi.fn(async () => {}),
		restore: vi.fn(async () => {}),
		export: vi.fn(async () => {}),
		preview: vi.fn(async () => ({
			version: 1,
			rules: [],
			views: [],
			accounts: ['account-1'],
			destinations: []
		})),
		import: vi.fn(async () => {}),
		available: vi.fn(async () => true),
		destinations: vi.fn(async () => [])
	};
}
function input(name: string, value: string) {
	const element = document.querySelector<HTMLInputElement>(`input[name="${name}"]`)!;
	element.value = value;
	element.dispatchEvent(new Event('input', { bubbles: true }));
	return element;
}
function click(text: string) {
	const button = [...document.querySelectorAll('button')].find(
		(b) => b.textContent?.trim() === text
	);
	expect(button).toBeTruthy();
	button!.click();
}
it('clears detached passphrases on submit, blocks duplicates and resets busy after failure', async () => {
	const api = transport();
	let reject!: (reason: string) => void;
	api.backup = vi.fn(
		() =>
			new Promise<void>((_, no) => {
				reject = no;
			})
	);
	const guard = new SettingsFormGuard();
	const component = mount(BackupForm, {
		target: document.body,
		props: { transport: api, guard }
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();
	const secret = input('passphrase', 'synthetic backup passphrase');
	input('confirmation', secret.value);
	document.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();
	document
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(secret.value).toBe('');
	expect(guard.busy).toBe(true);
	expect(api.backup).toHaveBeenCalledTimes(1);
	document
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(api.backup).toHaveBeenCalledTimes(1);
	reject('synthetic private provider detail');
	await settle();
	expect(guard.busy).toBe(false);
	expect(document.body.textContent).not.toContain('synthetic private provider detail');
});
it('canceled chooser and teardown clear secrets without starting a backup', async () => {
	const api = transport();
	api.choose = vi.fn(async () => null);
	const component = mount(BackupForm, {
		target: document.body,
		props: { transport: api }
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();
	const secret = input('passphrase', 'synthetic backup passphrase');
	input('confirmation', secret.value);
	document.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();
	document.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
	await settle();
	expect(secret.value).toBe('');
	expect(api.backup).not.toHaveBeenCalled();
	input('passphrase', 'synthetic another passphrase');
	await unmount(component);
	cleanup = () => {};
	expect(secret.value).toBe('');
});
it('requires every mapping and explicit consent before importing a preview', async () => {
	const api = transport();
	const guard = new SettingsFormGuard();
	const component = mount(PortableForm, {
		target: document.body,
		props: {
			transport: api,
			guard,
			accounts: [
				{
					account_id: 'a',
					email_display: 'reader@example.com',
					imap_host: 'imap.example.com',
					imap_port: 993,
					auth_method: 'AppPassword'
				}
			]
		}
	});
	cleanup = () => {
		void unmount(component);
	};
	click('Choose definitions to import');
	await settle();
	const button = [...document.querySelectorAll('button')].find(
		(b) => b.textContent?.trim() === 'Import reviewed definitions'
	)!;
	expect(button.disabled).toBe(true);
	expect(guard.dirty).toBe(true);
	const select = document.querySelector('select')!;
	select.value = 'a';
	select.dispatchEvent(new Event('change', { bubbles: true }));
	document.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();
	expect(button.disabled).toBe(false);
	button.click();
	await settle();
	expect(api.import).toHaveBeenCalledWith(
		'/tmp/synthetic-backup.tmb',
		{ accounts: { 'account-1': 'a' }, destinations: {} },
		expect.objectContaining({ version: 1 }),
		true
	);
	expect(guard.dirty).toBe(false);
});
it('restoration discloses the isolated vault unlock and does not auto-initialize', async () => {
	i18n.locale = 'pt-BR';
	const api = transport();
	const ondone = vi.fn();
	const component = mount(BackupForm, {
		target: document.body,
		props: { restore: true, transport: api, ondone }
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();
	input('passphrase', 'synthetic backup passphrase');
	input('confirmation', 'synthetic backup passphrase');
	document.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();
	document.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
	await settle();
	expect(api.restore).toHaveBeenCalledTimes(1);
	expect(ondone).not.toHaveBeenCalled();
	expect(document.body.textContent).toContain('novo cofre local');
	click('Continuar configuração');
	expect(ondone).toHaveBeenCalledOnce();
});

it('does not import after the preview is unmounted during a file read', async () => {
	const api = transport();
	let resolve!: (value: Awaited<ReturnType<TransferTransport['preview']>>) => void;
	api.preview = vi.fn(
		() =>
			new Promise<Awaited<ReturnType<TransferTransport['preview']>>>((yes) => {
				resolve = yes;
			})
	);
	const component = mount(PortableForm, {
		target: document.body,
		props: { transport: api }
	});
	click('Choose definitions to import');
	await settle();
	await unmount(component);
	resolve({ version: 1, rules: [], views: [], accounts: [], destinations: [] });
	await settle();
	expect(document.body.textContent).toBe('');
	expect(api.import).not.toHaveBeenCalled();
});

it('keeps unsaved forms independent while excluding concurrent operations', async () => {
	const { transferGuards } = await import('./guards');
	const parent = new SettingsFormGuard();
	const guards = transferGuards(parent);
	guards.backup.markDirty();
	guards.portable.markDirty();
	guards.portable.markDirty(false);
	expect(parent.dirty).toBe(true);
	guards.backup.setBusy(true);
	expect(guards.portable.isBlocked()).toBe(true);
	guards.backup.setBusy(false);
	expect(guards.portable.isBlocked()).toBe(false);
	guards.backup.reset();
	expect(parent.canProceed()).toBe(true);
});
