export type AuthMethod =
	'AppPassword' | 'ProtonBridgeLocal' | 'OAuth2PublicPkce' | 'OAuth2ByoCredentials';
export type AccountConfig = {
	account_id: string;
	email_display: string;
	imap_host: string;
	imap_port: number;
	auth_method: AuthMethod;
};
export type PasswordCredentials =
	{ AppPassword: { secret: string } } | { ProtonBridgeLocal: { bridge_password: string } };
export type Authorization = {
	authorization_endpoint: string;
	token_endpoint: string;
	scopes: string;
	client_id: string;
	client_secret?: string;
};
export interface AccountTransport {
	list(): Promise<AccountConfig[]>;
	unlock(passphrase: string): Promise<void>;
	add(config: AccountConfig, credentials: PasswordCredentials): Promise<string>;
	authorize(
		config: AccountConfig,
		authorization: Authorization,
		ready: () => void
	): Promise<string>;
	cancel(): Promise<void>;
}

export type AccountSubmission =
	{ credentials: PasswordCredentials } | { authorization: Authorization };
