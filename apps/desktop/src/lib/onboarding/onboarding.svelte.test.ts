import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { i18n } from '../i18n/index.svelte';
import type { FolderDto } from '../inbox/types';
import { AccountController } from './controller.svelte';
import { OnboardingSession } from './session.svelte';
import type { AccountConfig, AccountTransport, RecoveryTransport, VaultStatus } from './types';
import WdgAccountDiscoveryForm from './WdgAccountDiscoveryForm.svelte';
import WdgBootGate from './WdgBootGate.svelte';
import WdgConnectedAccountList from './WdgConnectedAccountList.svelte';
import WdgOnboardingPage from './WdgOnboardingPage.svelte';

vi.mock('../inbox/transport', () => ({
	nativeAvailable: () => false,
	nativeTransport: {
		search: async () => [],
		subscribe: async () => () => {},
		triage: async () => {},
		listFolders: async () => []
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

function fixture(options?: {
	native?: boolean;
	recoveryAvailable?: boolean;
	vaultStatus?: VaultStatus;
}) {
	const transport = {
		list: vi.fn<AccountTransport['list']>().mockResolvedValue([]),
		vaultStatus: vi
			.fn<NonNullable<AccountTransport['vaultStatus']>>()
			.mockResolvedValue(options?.vaultStatus ?? { exists: true, keyring_available: true }),
		unlock: vi.fn<AccountTransport['unlock']>().mockResolvedValue(),
		add: vi.fn<AccountTransport['add']>().mockResolvedValue('account-one'),
		authorize: vi.fn<AccountTransport['authorize']>().mockResolvedValue('account-one'),
		cancel: vi.fn<AccountTransport['cancel']>().mockResolvedValue(),
		testConnection: vi
			.fn<NonNullable<AccountTransport['testConnection']>>()
			.mockResolvedValue({ ok: true }),
		reauthorize: vi
			.fn<NonNullable<AccountTransport['reauthorize']>>()
			.mockResolvedValue('account-one'),
		diagnostics: vi
			.fn<NonNullable<AccountTransport['diagnostics']>>()
			.mockResolvedValue({}),
		chooseArchiveFile: vi
			.fn<NonNullable<AccountTransport['chooseArchiveFile']>>()
			.mockResolvedValue('/tmp/takeout.mbox'),
		importLocalArchive: vi
			.fn<NonNullable<AccountTransport['importLocalArchive']>>()
			.mockResolvedValue({ account_id: 'archive-one', imported_count: 2, batches_committed: 1, last_uid: 2 })
	};
	const recoveryTransport: RecoveryTransport = {
		available: vi.fn<RecoveryTransport['available']>().mockResolvedValue(
			options?.recoveryAvailable ?? false
		),
		choose: vi.fn<RecoveryTransport['choose']>().mockResolvedValue('/tmp/backup.triagebak'),
		restore: vi.fn<RecoveryTransport['restore']>().mockResolvedValue()
	};
	const controller = new AccountController(transport);
	const session = new OnboardingSession({
		accountTransport: transport,
		recoveryTransport,
		available: () => options?.native ?? true
	});
	const target = document.createElement('div');
	document.body.append(target);
	return { transport, recoveryTransport, controller, session, target };
}

async function settle() {
	for (let i = 0; i < 6; i++) {
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
	i18n.setLocale('en');
});

describe('account startup and boot gate', () => {
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

	it('routes browser mode straight to disconnected InboxSession without native IPC', async () => {
		const { transport, recoveryTransport, session, target } = fixture({ native: false });
		const onNavigate = vi.fn();
		const component = mount(WdgBootGate, {
			target,
			props: { session, onNavigate }
		});
		await settle();
		expect(transport.list).not.toHaveBeenCalled();
		expect(recoveryTransport.available).not.toHaveBeenCalled();
		expect(onNavigate).not.toHaveBeenCalled();
		expect(target.querySelector('[role="listbox"][aria-label="Messages"]')).not.toBeNull();
		await unmount(component);
	});

	it('redirects locked native boot to /onboarding, clears passphrase on submit, blocks duplicate unlocks, and unlocks to inbox', async () => {
		const { transport, session, target } = fixture({ native: true });
		transport.list
			.mockRejectedValueOnce('VaultUnavailable')
			.mockResolvedValueOnce([account]);
		const unlockPending = deferred<void>();
		transport.unlock.mockReturnValueOnce(unlockPending.promise);
		const onNavigate = vi.fn();

		const component = mount(WdgBootGate, {
			target,
			props: { session, onNavigate }
		});
		await settle();

		expect(onNavigate).toHaveBeenCalledWith('/onboarding');
		expect(target.textContent).toContain('Unlock your mail');
		// Demo switcher MUST NOT appear inside Tauri native runtime
		expect(target.querySelector('[aria-label="Onboarding lifecycle phases"]')).toBeNull();

		const passphrase = input(target, 'passphrase', 'synthetic-passphrase');
		submit(target);
		expect(passphrase.value).toBe('');
		submit(target);
		expect(transport.unlock).toHaveBeenCalledTimes(1);

		unlockPending.reject('synthetic-passphrase');
		await settle();
		expect(session.accounts.error).not.toContain('synthetic-passphrase');

		input(target, 'passphrase', 'synthetic-retry');
		submit(target);
		await settle();
		expect(session.accounts.phase).toBe('ready');
		expect(target.querySelector('[role="listbox"][aria-label="Messages"]')).not.toBeNull();
		await unmount(component);
	});

	it('presents Create local master passphrase with confirmation and keyring-unavailable copy on first run, and Unlock vault when vault exists', async () => {
		const firstRun = fixture({
			native: true,
			vaultStatus: { exists: false, keyring_available: false }
		});
		firstRun.transport.list
			.mockRejectedValueOnce('VaultUnavailable')
			.mockResolvedValueOnce([]);
		const firstComp = mount(WdgBootGate, {
			target: firstRun.target,
			props: { session: firstRun.session, onNavigate: vi.fn() }
		});
		await settle();

		expect(firstRun.transport.vaultStatus).toHaveBeenCalledTimes(1);
		expect(firstRun.target.textContent).toContain('Create local master passphrase');
		expect(firstRun.target.textContent).toContain('New Local Vault');
		expect(firstRun.target.textContent).toContain('No OS keyring (Secret Service)');
		const passInput = firstRun.target.querySelector<HTMLInputElement>('input[name="passphrase"]')!;
		const confirmInput = firstRun.target.querySelector<HTMLInputElement>('input[name="confirmation"]')!;
		expect(confirmInput).not.toBeNull();

		// Mismatched confirmation clears both DOM inputs and blocks unlock.
		input(firstRun.target, 'passphrase', 'synthetic-first');
		input(firstRun.target, 'confirmation', 'synthetic-other');
		submit(firstRun.target);
		expect(passInput.value).toBe('');
		expect(confirmInput.value).toBe('');
		expect(firstRun.transport.unlock).not.toHaveBeenCalled();
		expect(firstRun.target.textContent).toContain('Passphrase and confirmation must match');

		// Matching >= 8 chars calls unlock, clears inputs, and reaches ready state.
		input(firstRun.target, 'passphrase', 'synthetic-master');
		input(firstRun.target, 'confirmation', 'synthetic-master');
		submit(firstRun.target);
		expect(passInput.value).toBe('');
		expect(confirmInput.value).toBe('');
		expect(firstRun.transport.unlock).toHaveBeenCalledWith('synthetic-master');
		await settle();
		await unmount(firstComp);

		// Existing vault shows Unlock vault without confirmation input.
		const existing = fixture({
			native: true,
			vaultStatus: { exists: true, keyring_available: false }
		});
		existing.transport.list.mockRejectedValueOnce('VaultUnavailable');
		const existingComp = mount(WdgBootGate, {
			target: existing.target,
			props: { session: existing.session, onNavigate: vi.fn() }
		});
		await settle();
		expect(existing.target.textContent).toContain('Unlock your mail');
		expect(existing.target.textContent).toContain('Unlock vault');
		expect(existing.target.querySelector('input[name="confirmation"]')).toBeNull();
		await unmount(existingComp);
	});

	it('gates startup recovery backups (.triagebak) with consent and allows skipping', async () => {
		const { transport, recoveryTransport, session, target } = fixture({
			native: true,
			recoveryAvailable: true
		});
		const onNavigate = vi.fn();
		const component = mount(WdgBootGate, {
			target,
			props: { session, onNavigate }
		});
		await settle();

		expect(onNavigate).toHaveBeenCalledWith('/onboarding');
		expect(target.textContent).toContain('Restore from Encrypted Backup');
		expect(transport.list).not.toHaveBeenCalled();

		input(target, 'passphrase', 'synthetic-backup-key');
		input(target, 'confirmation', 'synthetic-backup-key');
		const checkbox = target.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
		checkbox.checked = true;
		checkbox.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		submit(target);
		await settle();
		expect(recoveryTransport.choose).toHaveBeenCalledWith(false);
		expect(recoveryTransport.restore).toHaveBeenCalledWith(
			'/tmp/backup.triagebak',
			'synthetic-backup-key',
			true
		);
		expect(target.querySelector<HTMLInputElement>('input[name="passphrase"]')).toBeNull();
		await unmount(component);
	});

	it('keeps existing mail mounted cleanly and exposes settings navigation after boot', async () => {
		const { transport, session, target } = fixture({ native: true });
		const pending = deferred<AccountConfig[]>();
		transport.list.mockReturnValueOnce(pending.promise);
		const onNavigate = vi.fn();

		const component = mount(WdgBootGate, {
			target,
			props: { session, onNavigate }
		});
		flushSync();
		expect(target.textContent).toContain('Opening local mail');
		expect(target.querySelector('input')).toBeNull();

		pending.resolve([account]);
		await settle();
		expect(onNavigate).not.toHaveBeenCalled();
		expect(target.textContent).toContain('Loaded Messages');
		const listbox = target.querySelector('[role="listbox"][aria-label="Messages"]');
		expect(listbox).not.toBeNull();
		const settingsButtons = [...target.querySelectorAll('button')].filter(
			(b) => b.getAttribute('aria-label') === 'Settings'
		);
		expect(settingsButtons).toHaveLength(1);
		expect(settingsButtons[0].hasAttribute('aria-haspopup')).toBe(false);

		settingsButtons[0].click();
		await settle();
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

describe('multi-account directory, folder catalog, and demo switcher gating', () => {
	it('shows the demo phase switcher only outside Tauri and hides it in native mode', async () => {
		const browser = fixture({ native: false });
		const demoComponent = mount(WdgOnboardingPage, {
			target: browser.target,
			props: { session: browser.session }
		});
		await settle();
		expect(browser.target.querySelector('[aria-label="Onboarding lifecycle phases"]')).not.toBeNull();
		await unmount(demoComponent);

		const tauri = fixture({ native: true });
		tauri.transport.list.mockResolvedValueOnce([account]);
		const nativeComponent = mount(WdgOnboardingPage, {
			target: tauri.target,
			props: { session: tauri.session, listFolders: vi.fn().mockResolvedValue([]) }
		});
		await settle();
		expect(tauri.target.querySelector('[aria-label="Onboarding lifecycle phases"]')).toBeNull();
		expect(tauri.target.textContent).toContain('reader@example.com');
		await unmount(nativeComponent);
	});

	it('renders demo Phase 3 Connected Accounts without binding_property_non_reactive warnings', async () => {
		const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
		const browser = fixture({ native: false });
		const demoComponent = mount(WdgOnboardingPage, {
			target: browser.target,
			props: { session: browser.session }
		});
		await settle();

		const phase3Btn = [...browser.target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('3. Connected Accounts')
		)!;
		phase3Btn.click();
		await settle();

		const warnings = warnSpy.mock.calls.flat().map(String).join('\n');
		expect(warnings).not.toContain('binding_property_non_reactive');

		const uploadInput = browser.target.querySelector<HTMLInputElement>(
			'input[type="file"][aria-label^="Upload avatar"]'
		)!;
		expect(uploadInput).not.toBeNull();
		const clickSpy = vi.spyOn(uploadInput, 'click');
		const chooseBtn = [...browser.target.querySelectorAll('button')].find(
			(b) => b.textContent?.includes('Replace photo') || b.textContent?.includes('Choose a photo')
		)!;
		chooseBtn.click();
		expect(clickSpy).toHaveBeenCalledTimes(1);

		await unmount(demoComponent);
	});

	it('loads and retries cmd_list_folders lazily per account on /onboarding', async () => {
		const { transport, session, target } = fixture({ native: true });
		transport.list.mockResolvedValueOnce([account]);
		const folders: FolderDto[] = [
			{
				account_id: 'account-one',
				folder_name: 'INBOX',
				display_name: 'Inbox',
				role: 'Inbox',
				delimiter: '/',
				is_selectable: true,
				total_messages: 42,
				unread_messages: 3
			},
			{
				account_id: 'account-one',
				folder_name: 'Archive',
				display_name: 'Archive',
				role: 'Archive',
				delimiter: '/',
				is_selectable: true,
				total_messages: 120,
				unread_messages: 0
			}
		];
		const listFolders = vi
			.fn<(accountId: string) => Promise<FolderDto[]>>()
			.mockRejectedValueOnce(new Error('offline'))
			.mockResolvedValueOnce(folders);

		const component = mount(WdgOnboardingPage, {
			target,
			props: { session, listFolders }
		});
		await settle();
		expect(listFolders).not.toHaveBeenCalled();

		const expandBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Folders')
		)!;
		expandBtn.click();
		await settle();
		expect(listFolders).toHaveBeenCalledWith('account-one');
		expect(target.textContent).toContain('Folder catalog unavailable');

		const retryBtn = [...target.querySelectorAll('button')].find(
			(b) => b.textContent?.trim() === 'Try again'
		)!;
		retryBtn.click();
		await settle();
		expect(target.textContent).toContain('Inbox');
		expect(target.textContent).toContain('Archive');
		expect(target.textContent).toContain('42');
		await unmount(component);
	});
});

describe('account forms and network consent', () => {
	it('sends password only on submit and immediately clears the unbound input', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		const pending = deferred<string>();
		transport.add.mockReturnValue(pending.promise);
		const component = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller }
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

	it('clears credentials on auth tab switch and teardown, uses Bridge credentials, and requests OAuth explicitly', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		const component = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller }
		});
		flushSync();
		const old = input(target, 'secret', 'synthetic-old-password');
		const authTabs = [...target.querySelectorAll<HTMLButtonElement>('[role="tablist"] button[role="tab"]')];
		authTabs.find((b) => b.textContent?.includes('Proton Mail Bridge'))!.click();
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

		authTabs.find((b) => b.textContent?.includes('Your client credentials'))!.click();
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
			.find((b) => b.textContent?.includes('Cancel authorization'))!
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
	it('switches app language via segmented selector on WdgOnboardingPage', async () => {
		const { session, transport, target } = fixture({ native: true });
		transport.list.mockResolvedValueOnce([]);
		const component = mount(WdgOnboardingPage, {
			target,
			props: { session }
		});
		await settle();

		expect(target.textContent).toContain('Connect an account');

		const ptButton = target.querySelector<HTMLButtonElement>('button[aria-label="Português"]')!;
		ptButton.click();
		flushSync();

		expect(target.textContent).toContain('Conectar uma conta');

		const enButton = target.querySelector<HTMLButtonElement>('button[aria-label="English"]')!;
		enButton.click();
		flushSync();

		expect(target.textContent).toContain('Connect an account');
		await unmount(component);
	});

	it('toggles interactive sign-in help guide and displays method tabs with step-by-step playbooks', async () => {
		const { controller, target } = fixture();
		controller.phase = 'ready';
		const component = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller }
		});
		flushSync();

		const toggleButton = target.querySelector<HTMLButtonElement>(
			'button[aria-controls="account-help-content"]'
		)!;
		expect(toggleButton.getAttribute('aria-expanded')).toBe('false');

		toggleButton.click();
		flushSync();

		expect(toggleButton.getAttribute('aria-expanded')).toBe('true');
		expect(target.textContent).toContain('Two-Factor Authentication');
		expect(target.textContent).toContain('Gmail, Google Workspace');
		expect(target.textContent).toContain('What to fill in the form below:');
		expect(target.textContent).toContain('Step-by-step: how to obtain your credentials:');
		expect(target.textContent).toContain('myaccount.google.com/apppasswords');
		// INVARIANT: When Gmail is selected by default, other provider playbooks are filtered out
		expect(target.textContent).not.toContain('appleid.apple.com');
		expect(target.textContent).not.toContain('Settings -> Privacy & Security -> Integrations');

		const tabs = [
			...target.querySelectorAll<HTMLButtonElement>('#account-help-content button[role="tab"]')
		];
		expect(tabs).toHaveLength(6);

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

		tabs[4].click();
		flushSync();
		expect(target.textContent).toContain('Cloudflare Email Routing');

		tabs[5].click();
		flushSync();
		expect(target.textContent).toContain('takeout.google.com');
		expect(target.textContent).toContain('Mozilla Thunderbird');

		await unmount(component);
	});

	it('applies provider presets to pre-fill host, port and authentication method', async () => {
		const { controller, target } = fixture();
		controller.phase = 'ready';
		const component = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller }
		});
		flushSync();

		const buttons = [...target.querySelectorAll('button')];
		const gmailPreset = buttons.find((b) => b.textContent?.trim() === 'Gmail')!;
		const gmailOAuthPreset = buttons.find((b) => b.textContent?.trim() === 'Gmail OAuth')!;
		const icloudPreset = buttons.find((b) => b.textContent?.trim() === 'iCloud')!;
		const protonPreset = buttons.find((b) => b.textContent?.trim() === 'Proton')!;

		expect(gmailPreset.getAttribute('aria-pressed')).toBe('true');
		expect(icloudPreset.getAttribute('aria-pressed')).toBe('false');

		const toggleButton = target.querySelector<HTMLButtonElement>(
			'button[aria-controls="account-help-content"]'
		)!;
		toggleButton.click();
		flushSync();
		expect(target.textContent).toContain('myaccount.google.com/apppasswords');
		expect(target.textContent).not.toContain('appleid.apple.com');

		icloudPreset.click();
		flushSync();
		expect(gmailPreset.getAttribute('aria-pressed')).toBe('false');
		expect(icloudPreset.getAttribute('aria-pressed')).toBe('true');
		expect(target.textContent).toContain('appleid.apple.com');
		expect(target.textContent).not.toContain('myaccount.google.com/apppasswords');

		gmailPreset.click();
		flushSync();
		expect(target.querySelector<HTMLInputElement>('input[name="host"]')!.value).toBe(
			'imap.gmail.com'
		);
		expect(target.querySelector<HTMLInputElement>('input[name="port"]')!.value).toBe('993');

		gmailOAuthPreset.click();
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

		protonPreset.click();
		flushSync();
		expect(target.querySelector<HTMLInputElement>('input[name="host"]')!.value).toBe('127.0.0.1');
		expect(target.querySelector<HTMLInputElement>('input[name="port"]')!.value).toBe('1143');

		await unmount(component);
	});

	it('imports Offline Archive in WdgAccountDiscoveryForm without IMAP fields and shows Offline Archive badge', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		const pending = deferred<{
			account_id: string;
			imported_count: number;
			batches_committed: number;
			last_uid: number;
		}>();
		transport.chooseArchiveFile
			.mockResolvedValueOnce('/tmp/single.eml')
			.mockResolvedValueOnce('/tmp/takeout.mbox');
		transport.importLocalArchive.mockImplementationOnce((_path, _email, _fmt, onProgress) => {
			onProgress?.({
				account_id: 'arch-1',
				state: 'Syncing',
				new_messages_count: 5,
				triage_alerts: [],
				embedding_errors_count: 0
			});
			return pending.promise;
		});

		const onSuccess = vi.fn();
		const formComponent = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller, onSuccess }
		});
		flushSync();

		const archiveTab = [...target.querySelectorAll<HTMLButtonElement>('button[role="tab"]')].find(
			(b) => b.textContent?.trim() === 'Offline Archive'
		)!;
		archiveTab.click();
		flushSync();

		expect(target.querySelector('input[name="host"]')).toBeNull();
		expect(target.querySelector('input[name="port"]')).toBeNull();
		expect(target.querySelector('[data-credential]')).toBeNull();

		const chooseBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Choose .mbox or .eml file')
		)!;
		chooseBtn.click();
		await settle();
		const radios = [...target.querySelectorAll<HTMLButtonElement>('button[role="radio"]')];
		expect(radios[1].getAttribute('aria-checked')).toBe('true');

		chooseBtn.click();
		await settle();
		expect(radios[0].getAttribute('aria-checked')).toBe('true');

		input(target, 'email', 'archive@example.com');
		submit(target);
		await settle();

		expect(transport.importLocalArchive).toHaveBeenCalledWith(
			'/tmp/takeout.mbox',
			'archive@example.com',
			'Mbox',
			expect.any(Function)
		);
		expect(target.textContent).toContain('Messages imported so far: 5');

		pending.resolve({ account_id: 'arch-1', imported_count: 5, batches_committed: 1, last_uid: 5 });
		await settle();
		expect(onSuccess).toHaveBeenCalledOnce();
		await unmount(formComponent);

		const listTarget = document.createElement('div');
		document.body.append(listTarget);
		const listComponent = mount(WdgConnectedAccountList, {
			target: listTarget,
			props: {
				accounts: [
					{
						account_id: 'arch-1',
						email_display: 'archive@example.com',
						imap_host: '',
						imap_port: 0,
						auth_method: 'LocalArchive'
					}
				],
				onAddAccount: vi.fn()
			}
		});
		flushSync();
		expect(listTarget.textContent).toContain('Offline Archive (Local Only)');
		expect(listTarget.textContent).not.toContain('Disconnected');
		await unmount(listComponent);
	});
});

describe('actionable connection diagnostics, pre-save login test, and OAuth reauthorization', () => {
	it('runs pre-save Test login without persisting credentials, clears DOM inputs immediately, and surfaces EN/PT-BR recovery hints', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		transport.testConnection
			.mockResolvedValueOnce({ ok: false, diagnostic: 'ImapDisabledHint' })
			.mockResolvedValueOnce({ ok: true });

		const component = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller }
		});
		flushSync();

		input(target, 'email', 'reader@example.com');
		input(target, 'host', 'imap.example.com');
		const secret = input(target, 'secret', 'synthetic-test-secret');

		const testBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Test login')
		)!;
		expect(testBtn).toBeDefined();
		testBtn.click();
		expect(secret.value).toBe('');
		expect(JSON.stringify(controller)).not.toContain('synthetic-test-secret');
		await settle();

		expect(transport.add).not.toHaveBeenCalled();
		expect(transport.testConnection).toHaveBeenCalledWith(
			{ ...account, account_id: '' },
			{ AppPassword: { secret: 'synthetic-test-secret' } }
		);
		expect(target.textContent).toContain('IMAP access is disabled on this mailbox');

		i18n.setLocale('pt-BR');
		flushSync();
		expect(target.textContent).toContain('Acesso IMAP desativado nesta conta');
		i18n.setLocale('en');
		flushSync();

		input(target, 'secret', 'synthetic-valid-secret');
		testBtn.click();
		expect(secret.value).toBe('');
		await settle();
		expect(target.textContent).toContain('Login verified');

		// Hidden on OAuth and Offline Archive methods.
		const archiveTab = [...target.querySelectorAll<HTMLButtonElement>('button[role="tab"]')].find(
			(b) => b.textContent?.trim() === 'Offline Archive'
		)!;
		archiveTab.click();
		flushSync();
		expect(
			[...target.querySelectorAll('button')].find((b) => b.textContent?.includes('Test login'))
		).toBeUndefined();

		await unmount(component);
	});

	it('classifies OAuth registration errors into localized guidance without echoing provider errors', async () => {
		const { controller, transport, target } = fixture();
		controller.phase = 'ready';
		transport.authorize.mockRejectedValueOnce('OAuthAccessDenied');

		const component = mount(WdgAccountDiscoveryForm, {
			target,
			props: { controller }
		});
		flushSync();

		const tabs = [...target.querySelectorAll<HTMLButtonElement>('[role="tablist"] button[role="tab"]')];
		tabs.find((b) => b.textContent?.includes('Your client credentials'))!.click();
		flushSync();

		input(target, 'email', 'oauth@example.com');
		input(target, 'host', 'imap.gmail.com');
		input(target, 'authorize', 'https://auth.example.com/authorize');
		input(target, 'token', 'https://auth.example.com/token');
		input(target, 'scopes', 'https://mail.google.com/');
		input(target, 'client', 'synthetic-client-id');
		input(target, 'secret', 'synthetic-client-secret');
		submit(target);
		await settle();

		expect(target.textContent).toContain('Provider denied OAuth authorization');
		expect(target.textContent).toContain('Test users');

		i18n.setLocale('pt-BR');
		flushSync();
		expect(target.textContent).toContain('Provedor negou a autorização OAuth');

		await unmount(component);
	});

	it('displays per-account sync diagnostics and reauthorizes OAuth accounts in place while preserving vault client creds', async () => {
		const gmailOAuth: AccountConfig = {
			account_id: 'oauth-1',
			email_display: 'gmail.user@example.com',
			imap_host: 'imap.gmail.com',
			imap_port: 993,
			auth_method: 'OAuth2ByoCredentials'
		};
		const { session, transport, target } = fixture({ native: true });
		transport.list.mockResolvedValue([gmailOAuth, account]);
		transport.diagnostics
			.mockResolvedValueOnce({ 'oauth-1': 'OAuthAccessDenied' })
			.mockResolvedValueOnce({});

		const pending = deferred<string>();
		transport.reauthorize.mockImplementationOnce((_reauth, ready) => {
			ready();
			return pending.promise;
		});

		const component = mount(WdgOnboardingPage, {
			target,
			props: { session, listFolders: vi.fn().mockResolvedValue([]) }
		});
		await settle();

		expect(target.textContent).toContain('Connection attention required');
		expect(target.textContent).toContain('Provider denied OAuth authorization');

		const reauthButtons = [...target.querySelectorAll('button')].filter((b) =>
			b.textContent?.includes('Reauthorize')
		);
		expect(reauthButtons).toHaveLength(1);
		reauthButtons[0].click();
		flushSync();

		// Host OAuth defaults should prefill Gmail endpoints & scope automatically.
		const authInput = target.querySelector<HTMLInputElement>('input[name="authorize"]')!;
		const tokenInput = target.querySelector<HTMLInputElement>('input[name="token"]')!;
		const scopesInput = target.querySelector<HTMLInputElement>('input[name="scopes"]')!;
		expect(authInput.value).toBe('https://accounts.google.com/o/oauth2/v2/auth');
		expect(tokenInput.value).toBe('https://oauth2.googleapis.com/token');
		expect(scopesInput.value).toBe('https://mail.google.com/');

		// Leaving client_id / client_secret blank submits without overriding vault client creds, and still clears any typed credential inputs.
		const clientInput = input(target, 'client', '');
		const secretInput = input(target, 'secret', 'synthetic-optional-secret');
		submit(target);
		expect(clientInput.value).toBe('');
		expect(secretInput.value).toBe('');
		expect(transport.reauthorize).toHaveBeenCalledWith(
			{
				account_id: 'oauth-1',
				authorization_endpoint: 'https://accounts.google.com/o/oauth2/v2/auth',
				token_endpoint: 'https://oauth2.googleapis.com/token',
				scopes: 'https://mail.google.com/',
				client_secret: 'synthetic-optional-secret'
			},
			expect.any(Function)
		);
		expect(target.textContent).toContain('Finish authorization in your browser');

		pending.resolve('oauth-1');
		await settle();

		expect(target.textContent).toContain('OAuth tokens refreshed');
		expect(target.textContent).not.toContain('Connection attention required');
		expect(session.accounts.accounts).toEqual([gmailOAuth, account]);

		await unmount(component);
	});
});
