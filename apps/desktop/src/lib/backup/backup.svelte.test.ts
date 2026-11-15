import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeAll, expect, it, vi } from 'vitest';
import { i18n } from '../i18n/index.svelte';
import { transferGuards } from './guards';
import { SettingsFormGuard } from '../settings/guards';
import WdgBackupDefinitionsPanel from '../settings/WdgBackupDefinitionsPanel.svelte';
import type { TransferTransport } from './transport';

beforeAll(() => {
	vi.stubGlobal(
		'ResizeObserver',
		class {
			observe() {}
			unobserve() {}
			disconnect() {}
		}
	);
});

let cleanup = () => {};
afterEach(() => {
	cleanup();
	cleanup = () => {};
	document.body.innerHTML = '';
	i18n.setLocale('en');
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

function input(name: string, value: string, root: ParentNode = document) {
	const element = root.querySelector<HTMLInputElement>(`input[name="${name}"]`)!;
	element.value = value;
	element.dispatchEvent(new Event('input', { bubbles: true }));
	return element;
}

function click(text: string, root: ParentNode = document) {
	const button = [...root.querySelectorAll('button')].find(
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
	const component = mount(WdgBackupDefinitionsPanel, {
		target: document.body,
		props: { transport: api, guard }
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();

	const form = document.querySelector<HTMLFormElement>('form[aria-label="Create encrypted backup"]')!;
	const secret = input('passphrase', 'synthetic backup passphrase', form);
	input('confirmation', secret.value, form);
	form.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();

	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(secret.value).toBe('');
	expect(guard.busy).toBe(true);
	expect(api.backup).toHaveBeenCalledTimes(1);

	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
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
	const component = mount(WdgBackupDefinitionsPanel, {
		target: document.body,
		props: { transport: api }
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();

	const form = document.querySelector<HTMLFormElement>('form[aria-label="Create encrypted backup"]')!;
	const secret = input('passphrase', 'synthetic backup passphrase', form);
	input('confirmation', secret.value, form);
	form.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();

	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(secret.value).toBe('');
	expect(api.backup).not.toHaveBeenCalled();

	input('passphrase', 'synthetic another passphrase', form);
	await unmount(component);
	cleanup = () => {};
	expect(secret.value).toBe('');
});

it('requires every mapping and explicit consent before importing a preview via WdgImportPreviewDialog', async () => {
	const api = transport();
	const guard = new SettingsFormGuard();
	const importedEvents: number[] = [];
	const onImported = () => importedEvents.push(1);
	window.addEventListener('triage-definitions-imported', onImported);

	const component = mount(WdgBackupDefinitionsPanel, {
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
		window.removeEventListener('triage-definitions-imported', onImported);
		void unmount(component);
	};

	click('Choose definitions to import');
	await settle();

	const dialog = document.body.querySelector('[role="dialog"]')!;
	expect(dialog).not.toBeNull();
	const button = [...dialog.querySelectorAll('button')].find(
		(b) => b.textContent?.trim() === 'Import reviewed definitions'
	)!;
	expect(button.disabled).toBe(true);
	expect(guard.dirty).toBe(true);

	const select = dialog.querySelector('select')!;
	select.value = 'a';
	select.dispatchEvent(new Event('change', { bubbles: true }));
	dialog.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
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
	expect(importedEvents).toHaveLength(1);
});

it('restoration discloses the isolated vault unlock and does not auto-initialize', async () => {
	i18n.setLocale('pt-BR');
	const api = transport();
	const ondone = vi.fn();
	const component = mount(WdgBackupDefinitionsPanel, {
		target: document.body,
		props: { restore: true, transport: api, ondone }
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();

	const form = document.querySelector<HTMLFormElement>(
		'form[aria-label="Restaurar backup criptografado"]'
	)!;
	input('passphrase', 'synthetic backup passphrase', form);
	input('confirmation', 'synthetic backup passphrase', form);
	form.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click();
	await settle();

	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();

	expect(api.restore).toHaveBeenCalledTimes(1);
	expect(ondone).not.toHaveBeenCalled();
	expect(document.body.textContent).toContain('novo cofre local');
	click('Continuar configuração');
	expect(ondone).toHaveBeenCalledOnce();
});

it('does not import after the panel is unmounted during a preview file read', async () => {
	const api = transport();
	let resolve!: (value: Awaited<ReturnType<TransferTransport['preview']>>) => void;
	api.preview = vi.fn(
		() =>
			new Promise<Awaited<ReturnType<TransferTransport['preview']>>>((yes) => {
				resolve = yes;
			})
	);
	const component = mount(WdgBackupDefinitionsPanel, {
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

it('supports browser demo portable diff preview and conflict resolution when native is unavailable', async () => {
	const component = mount(WdgBackupDefinitionsPanel, {
		target: document.body
	});
	cleanup = () => {
		void unmount(component);
	};
	await settle();

	click('Preview Diff');
	await settle();

	const dialog = document.body.querySelector('[role="dialog"]')!;
	expect(dialog.textContent).toContain('Portable Definitions Import Preview');
	expect(dialog.textContent).toContain('Auto-tag NF-e XML Invoices');

	const overwriteBtns = [...dialog.querySelectorAll('button')].filter(
		(b) => b.textContent?.trim() === 'Overwrite'
	);
	expect(overwriteBtns.length).toBeGreaterThan(0);
	overwriteBtns[0].click();
	flushSync();

	click('Commit Import', dialog);
	await settle();
	expect(document.body.querySelector('[role="dialog"]')).toBeNull();
});

it('keeps unsaved forms independent while excluding concurrent operations', () => {
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
