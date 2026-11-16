import { beforeEach, expect, it, vi } from 'vitest';
import { Channel, invoke } from '@tauri-apps/api/core';
import { nativeAccountTransport } from './transport';
import type { AccountConfig } from './types';
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
	Channel: class {
		onmessage = (_message: null) => {};
	}
}));
beforeEach(() => vi.mocked(invoke).mockReset().mockResolvedValue(undefined));
it('preserves account IPC envelopes and camelCase-free cancel arguments', async () => {
	const config: AccountConfig = {
		account_id: '',
		email_display: 'reader@example.com',
		imap_host: 'imap.example.com',
		imap_port: 993,
		auth_method: 'AppPassword'
	};
	const credentials = { AppPassword: { secret: 'synthetic-password' } };
	await nativeAccountTransport.list();
	await nativeAccountTransport.unlock('synthetic-passphrase');
	await nativeAccountTransport.add(config, credentials);
	const authorization = {
		authorization_endpoint: 'https://auth.example.com/authorize',
		token_endpoint: 'https://auth.example.com/token',
		scopes: 'imap',
		client_id: 'synthetic-client'
	};
	await nativeAccountTransport.authorize(
		{ ...config, auth_method: 'OAuth2PublicPkce' },
		authorization,
		vi.fn()
	);
	await nativeAccountTransport.cancel();
	expect(vi.mocked(invoke).mock.calls).toEqual([
		['cmd_list_accounts'],
		['cmd_unlock_vault', { passphrase: 'synthetic-passphrase' }],
		['cmd_add_account', { config, credentials }],
		[
			'cmd_authorize_account',
			{
				config: { ...config, auth_method: 'OAuth2PublicPkce' },
				authorization,
				onReady: expect.anything()
			}
		],
		['cmd_cancel_authorization']
	]);
});

it('acknowledges native readiness and ignores channel messages after settlement', async () => {
	let resolve!: (id: string) => void;
	const pending = new Promise<string>((done) => {
		resolve = done;
	});
	vi.mocked(invoke).mockReturnValueOnce(pending);
	const ready = vi.fn();
	const operation = nativeAccountTransport.authorize(
		{
			account_id: '',
			email_display: 'reader@example.com',
			imap_host: 'imap.example.com',
			imap_port: 993,
			auth_method: 'OAuth2PublicPkce'
		},
		{
			authorization_endpoint: 'https://auth.example.com/authorize',
			token_endpoint: 'https://auth.example.com/token',
			scopes: 'imap',
			client_id: 'synthetic-client'
		},
		ready
	);
	const { onReady } = vi.mocked(invoke).mock.calls[0][1] as {
		onReady: Channel<null>;
	};
	expect(ready).not.toHaveBeenCalled();
	onReady.onmessage(null);
	expect(ready).toHaveBeenCalledTimes(1);
	resolve('account-one');
	await operation;
	onReady.onmessage(null);
	expect(ready).toHaveBeenCalledTimes(1);
});

it('keeps photo commands separate from account metadata and sends no file path', async () => {
	const { nativeAvatarTransport } = await import('./avatar-transport');
	await nativeAvatarTransport.get('a');
	await nativeAvatarTransport.set('a', [1, 2]);
	await nativeAvatarTransport.remove('a');
	expect(vi.mocked(invoke).mock.calls).toEqual([
		['cmd_get_account_avatar', { accountId: 'a' }],
		['cmd_set_account_avatar', { accountId: 'a', bytes: [1, 2] }],
		['cmd_remove_account_avatar', { accountId: 'a' }]
	]);
});

it('keeps discovery offline by default and probing a separate explicit command', async () => {
	const { nativeDiscoveryTransport } = await import('./discovery');
	await nativeDiscoveryTransport.discover('reader@example.com', false);
	await nativeDiscoveryTransport.discover('reader@example.com', true);
	await nativeDiscoveryTransport.probe('imap.example.com');
	expect(vi.mocked(invoke).mock.calls).toEqual([
		['cmd_discover_email_provider', { email: 'reader@example.com', online: false }],
		['cmd_discover_email_provider', { email: 'reader@example.com', online: true }],
		['cmd_probe_email_provider', { host: 'imap.example.com' }]
	]);
});

it('dispatches archive picker and streaming local archive import and stops forwarding after settlement', async () => {
	let resolveImport!: (result: {
		account_id: string;
		imported_count: number;
		batches_committed: number;
		last_uid: number;
	}) => void;
	const pending = new Promise<{
		account_id: string;
		imported_count: number;
		batches_committed: number;
		last_uid: number;
	}>((done) => {
		resolveImport = done;
	});
	vi.mocked(invoke)
		.mockResolvedValueOnce('/tmp/export.mbox')
		.mockReturnValueOnce(pending);

	const picked = await nativeAccountTransport.chooseArchiveFile!();
	expect(picked).toBe('/tmp/export.mbox');

	const onProgress = vi.fn();
	const run = nativeAccountTransport.importLocalArchive!(
		'/tmp/export.mbox',
		'archive@example.com',
		'Mbox',
		onProgress
	);

	expect(vi.mocked(invoke).mock.calls).toEqual([
		['cmd_choose_transfer_file', { save: false }],
		[
			'cmd_import_local_archive',
			{
				path: '/tmp/export.mbox',
				emailDisplay: 'archive@example.com',
				format: 'Mbox',
				onProgress: expect.anything()
			}
		]
	]);

	const { onProgress: channel } = vi.mocked(invoke).mock.calls[1][1] as {
		onProgress: Channel<{
			account_id: string;
			state: string;
			new_messages_count: number;
			triage_alerts: unknown[];
			embedding_errors_count: number;
		}>;
	};
	const evt = {
		account_id: 'arch-1',
		state: 'Syncing',
		new_messages_count: 3,
		triage_alerts: [],
		embedding_errors_count: 0
	};
	channel.onmessage(evt);
	expect(onProgress).toHaveBeenCalledTimes(1);

	resolveImport({ account_id: 'arch-1', imported_count: 3, batches_committed: 1, last_uid: 3 });
	await expect(run).resolves.toEqual({
		account_id: 'arch-1',
		imported_count: 3,
		batches_committed: 1,
		last_uid: 3
	});

	channel.onmessage(evt);
	expect(onProgress).toHaveBeenCalledTimes(1);
});

it('queries vault existence and keyring availability via cmd_vault_status', async () => {
	vi.mocked(invoke).mockResolvedValueOnce({ exists: false, keyring_available: false });
	await expect(nativeAccountTransport.vaultStatus!()).resolves.toEqual({
		exists: false,
		keyring_available: false
	});
	expect(vi.mocked(invoke).mock.calls).toEqual([['cmd_vault_status']]);
});

