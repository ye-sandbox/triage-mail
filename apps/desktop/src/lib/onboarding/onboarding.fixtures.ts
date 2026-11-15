import type { AuthMethod } from './types';

export type OnboardingPhase = 'vault-locked' | 'discovery' | 'directory';

export type DemoAccountHealth = 'Healthy' | 'Syncing' | 'Idle' | 'Retrying';

export interface DemoFolderDto {
	id: string;
	name: string;
	role: 'inbox' | 'sent' | 'archive' | 'trash' | 'custom';
	total_messages: number;
	unread_messages: number;
	uid_validity: number;
}

export interface DemoConnectedAccountDto {
	account_id: string;
	email_display: string;
	imap_host: string;
	imap_port: number;
	tls_mode: 'IMAPS' | 'STARTTLS' | 'LoopbackPinned';
	auth_method: AuthMethod;
	health: DemoAccountHealth;
	last_sync_iso: string;
	avatar_url: string | null;
	folders: DemoFolderDto[];
}

export interface IspdbPreset {
	domain: string;
	provider_name: string;
	imap_host: string;
	imap_port: number;
	recommended_auth: AuthMethod;
	auth_endpoint?: string;
	token_endpoint?: string;
	default_scopes?: string;
	note: string;
}

export interface TlsProbeResult {
	host: string;
	port: number;
	verified: boolean;
	protocol: string;
	cipher: string;
	spki_sha256: string;
	subject: string;
	loopback_pinned: boolean;
}

export const ONBOARDING_ACCOUNTS_FIXTURES: DemoConnectedAccountDto[] = [
	{
		account_id: 'acct-01',
		email_display: 'alex.work@example.com',
		imap_host: 'imap.fastmail.example.com',
		imap_port: 993,
		tls_mode: 'IMAPS',
		auth_method: 'AppPassword',
		health: 'Healthy',
		last_sync_iso: '2026-11-14T15:12:00-08:00',
		avatar_url: null,
		folders: [
			{
				id: 'INBOX',
				name: 'INBOX',
				role: 'inbox',
				total_messages: 1420,
				unread_messages: 18,
				uid_validity: 170041201
			},
			{
				id: 'Sent',
				name: 'Sent',
				role: 'sent',
				total_messages: 612,
				unread_messages: 0,
				uid_validity: 170041202
			},
			{
				id: 'Archive',
				name: 'Archive',
				role: 'archive',
				total_messages: 4890,
				unread_messages: 0,
				uid_validity: 170041203
			},
			{
				id: 'Trash',
				name: 'Trash',
				role: 'trash',
				total_messages: 84,
				unread_messages: 0,
				uid_validity: 170041204
			}
		]
	},
	{
		account_id: 'acct-02',
		email_display: 'billing@studio.example.org',
		imap_host: 'outlook.office365.example.com',
		imap_port: 993,
		tls_mode: 'IMAPS',
		auth_method: 'OAuth2PublicPkce',
		health: 'Syncing',
		last_sync_iso: '2026-11-14T15:14:40-08:00',
		avatar_url: null,
		folders: [
			{
				id: 'INBOX',
				name: 'INBOX',
				role: 'inbox',
				total_messages: 642,
				unread_messages: 7,
				uid_validity: 291003110
			},
			{
				id: 'Archive',
				name: 'Archive',
				role: 'archive',
				total_messages: 2110,
				unread_messages: 0,
				uid_validity: 291003111
			},
			{
				id: 'Trash',
				name: 'Trash',
				role: 'trash',
				total_messages: 38,
				unread_messages: 0,
				uid_validity: 291003112
			}
		]
	},
	{
		account_id: 'acct-03',
		email_display: 'vault@privacy.example.net',
		imap_host: '127.0.0.1',
		imap_port: 1143,
		tls_mode: 'LoopbackPinned',
		auth_method: 'ProtonBridgeLocal',
		health: 'Idle',
		last_sync_iso: '2026-11-14T15:08:15-08:00',
		avatar_url: null,
		folders: [
			{
				id: 'INBOX',
				name: 'INBOX',
				role: 'inbox',
				total_messages: 318,
				unread_messages: 4,
				uid_validity: 440192801
			},
			{
				id: 'Archive',
				name: 'Archive',
				role: 'archive',
				total_messages: 940,
				unread_messages: 0,
				uid_validity: 440192802
			},
			{
				id: 'Trash',
				name: 'Trash',
				role: 'trash',
				total_messages: 12,
				unread_messages: 0,
				uid_validity: 440192803
			}
		]
	},
	{
		account_id: 'acct-04',
		email_display: 'alerts@ops.example.org',
		imap_host: 'imap.custom.example.org',
		imap_port: 993,
		tls_mode: 'IMAPS',
		auth_method: 'OAuth2ByoCredentials',
		health: 'Retrying',
		last_sync_iso: '2026-11-14T15:02:05-08:00',
		avatar_url: null,
		folders: [
			{
				id: 'INBOX',
				name: 'INBOX',
				role: 'inbox',
				total_messages: 2890,
				unread_messages: 41,
				uid_validity: 551029301
			},
			{
				id: 'Archive',
				name: 'Archive',
				role: 'archive',
				total_messages: 5120,
				unread_messages: 0,
				uid_validity: 551029302
			},
			{
				id: 'Trash',
				name: 'Trash',
				role: 'trash',
				total_messages: 95,
				unread_messages: 0,
				uid_validity: 551029303
			}
		]
	}
];

export const ISPDB_PRESETS: Record<string, IspdbPreset> = {
	'example.com': {
		domain: 'example.com',
		provider_name: 'Fastmail Synthetic Preset',
		imap_host: 'imap.fastmail.example.com',
		imap_port: 993,
		recommended_auth: 'AppPassword',
		note: 'ISPDB domain record matched IMAPS on port 993 with App Password.'
	},
	'studio.example.org': {
		domain: 'studio.example.org',
		provider_name: 'Office 365 Synthetic Preset',
		imap_host: 'outlook.office365.example.com',
		imap_port: 993,
		recommended_auth: 'OAuth2PublicPkce',
		auth_endpoint: 'https://login.microsoftonline.example.org/common/oauth2/v2.0/authorize',
		token_endpoint: 'https://login.microsoftonline.example.org/common/oauth2/v2.0/token',
		default_scopes: 'https://outlook.office.com/IMAP.AccessAsUser.All offline_access',
		note: 'ISPDB domain record matched OAuth2 PKCE with loopback redirect.'
	},
	'privacy.example.net': {
		domain: 'privacy.example.net',
		provider_name: 'Proton Bridge Loopback',
		imap_host: '127.0.0.1',
		imap_port: 1143,
		recommended_auth: 'ProtonBridgeLocal',
		note: 'Local loopback Bridge detected; self-signed TLS pinning locked to 127.0.0.1:1143.'
	},
	'ops.example.org': {
		domain: 'ops.example.org',
		provider_name: 'Custom Ops Identity Provider',
		imap_host: 'imap.custom.example.org',
		imap_port: 993,
		recommended_auth: 'OAuth2ByoCredentials',
		auth_endpoint: 'https://auth.ops.example.org/oauth/authorize',
		token_endpoint: 'https://auth.ops.example.org/oauth/token',
		default_scopes: 'imap.read offline_access',
		note: 'ISPDB domain record matched custom OAuth tenant (BYO Client ID).'
	}
};

export function extractDomainOnly(emailInput: string): string {
	const trimmed = emailInput.trim().toLowerCase();
	const atIndex = trimmed.lastIndexOf('@');
	if (atIndex !== -1) {
		return trimmed.slice(atIndex + 1);
	}
	return trimmed;
}

export function lookupIspdbDomain(emailInput: string): IspdbPreset | null {
	const domain = extractDomainOnly(emailInput);
	if (!domain || !domain.includes('.')) return null;
	if (ISPDB_PRESETS[domain]) {
		return ISPDB_PRESETS[domain];
	}
	return {
		domain,
		provider_name: `Domain MX / SRV (${domain})`,
		imap_host: `imap.${domain}`,
		imap_port: 993,
		recommended_auth: 'AppPassword',
		note: `Resolved domain-only SRV fallback for ${domain} (port 993 IMAPS).`
	};
}

export function probeTlsEndpoint(host: string, port: number): TlsProbeResult {
	const cleanHost = host.trim() || '127.0.0.1';
	const isLoopback = cleanHost === '127.0.0.1' || cleanHost === '::1' || cleanHost === 'localhost';
	return {
		host: cleanHost,
		port,
		verified: true,
		protocol: isLoopback ? 'TLSv1.3 (Pinned Loopback)' : 'TLSv1.3 (WebPKI)',
		cipher: 'TLS_AES_256_GCM_SHA384',
		spki_sha256: isLoopback
			? 'sha256/bridge-loopback-7f000001-pinned-cert'
			: `sha256/${cleanHost.replace(/[^a-z0-9]/gi, '').slice(0, 12)}-webpki-verified`,
		subject: `CN=${cleanHost}`,
		loopback_pinned: isLoopback
	};
}

export function formatPkceTimer(seconds: number): string {
	const m = String(Math.floor(seconds / 60)).padStart(2, '0');
	const s = String(seconds % 60).padStart(2, '0');
	return `${m}:${s}`;
}

export function createDemoAccountDto(
	email: string,
	host: string,
	port: number,
	authMethod: AuthMethod
): DemoConnectedAccountDto {
	const id = `acct-${String(Math.floor(10 + Math.random() * 89))}`;
	return {
		account_id: id,
		email_display: email,
		imap_host: host,
		imap_port: port,
		tls_mode: authMethod === 'ProtonBridgeLocal' ? 'LoopbackPinned' : 'IMAPS',
		auth_method: authMethod,
		health: 'Healthy',
		last_sync_iso: new Date().toISOString(),
		avatar_url: null,
		folders: [
			{
				id: 'INBOX',
				name: 'INBOX',
				role: 'inbox',
				total_messages: 128,
				unread_messages: 4,
				uid_validity: 61001
			},
			{
				id: 'Archive',
				name: 'Archive',
				role: 'archive',
				total_messages: 340,
				unread_messages: 0,
				uid_validity: 61002
			},
			{
				id: 'Trash',
				name: 'Trash',
				role: 'trash',
				total_messages: 9,
				unread_messages: 0,
				uid_validity: 61003
			}
		]
	};
}

export function validateDemoDiscoveryInput(opts: {
	email: string;
	host: string;
	authMethod: AuthMethod;
	appSecretLen: number;
	bridgeSecretLen: number;
	pkceAuthorized: boolean;
	clientLen: number;
}): string | null {
	if (!opts.email || !opts.host) return 'Email address and IMAP host are required.';
	if (opts.authMethod === 'AppPassword' && opts.appSecretLen === 0) return 'Enter an App Password.';
	if (opts.authMethod === 'ProtonBridgeLocal' && opts.bridgeSecretLen === 0)
		return 'Enter the Bridge IMAP password.';
	if (opts.authMethod === 'OAuth2PublicPkce' && !opts.pkceAuthorized)
		return 'Complete 127.0.0.1 PKCE authorization first.';
	if (opts.authMethod === 'OAuth2ByoCredentials' && opts.clientLen === 0)
		return 'Provide your OAuth client_id.';
	return null;
}
