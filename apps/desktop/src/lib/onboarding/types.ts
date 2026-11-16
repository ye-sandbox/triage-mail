export type AuthMethod =
	| 'AppPassword'
	| 'ProtonBridgeLocal'
	| 'OAuth2PublicPkce'
	| 'OAuth2ByoCredentials'
	| 'LocalArchive';
export type LocalArchiveFormat = 'Mbox' | 'Eml';
export type LocalArchiveImportResult = {
	account_id: string;
	imported_count: number;
	batches_committed: number;
	last_uid: number;
};
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
export type VaultStatus = {
	exists: boolean;
	keyring_available: boolean;
};

export interface AccountTransport {
	list(): Promise<AccountConfig[]>;
	vaultStatus?(): Promise<VaultStatus>;
	unlock(passphrase: string): Promise<void>;
	add(config: AccountConfig, credentials: PasswordCredentials): Promise<string>;
	authorize(
		config: AccountConfig,
		authorization: Authorization,
		ready: () => void
	): Promise<string>;
	cancel(): Promise<void>;
	chooseArchiveFile?(): Promise<string | null>;
	importLocalArchive?(
		path: string,
		emailDisplay: string,
		format: LocalArchiveFormat,
		onProgress?: (event: import('../inbox/types').SyncProgressEvent) => void
	): Promise<LocalArchiveImportResult>;
}

export type AccountSubmission =
	{ credentials: PasswordCredentials } | { authorization: Authorization };

export type FolderCatalogTransport = {
	listFolders: (accountId: string) => Promise<import('../inbox/types').FolderDto[]>;
};

export type RecoveryTransport = {
	available: () => Promise<boolean>;
	choose: (save: boolean) => Promise<string | null>;
	restore: (path: string, passphrase: string, consent: boolean) => Promise<void>;
};
