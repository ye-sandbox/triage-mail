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
	const { onReady } = vi.mocked(invoke).mock.calls[0][1] as { onReady: Channel<null> };
	expect(ready).not.toHaveBeenCalled();
	onReady.onmessage(null);
	expect(ready).toHaveBeenCalledTimes(1);
	resolve('account-one');
	await operation;
	onReady.onmessage(null);
	expect(ready).toHaveBeenCalledTimes(1);
});
