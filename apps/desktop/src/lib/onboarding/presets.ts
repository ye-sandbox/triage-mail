import type { AuthMethod } from './types';
import {
	GOOGLE_AUTH_ENDPOINT,
	GOOGLE_IMAP_SCOPE,
	GOOGLE_TOKEN_ENDPOINT
} from './google-oauth-json';

export interface ProviderPreset {
	name: string;
	host: string;
	port: number;
	method: AuthMethod;
	authEndpoint?: string;
	tokenEndpoint?: string;
	scopes?: string;
}

export const microsoftPreset: ProviderPreset = {
	name: 'Outlook',
	host: 'outlook.office365.com',
	port: 993,
	method: 'OAuth2PublicPkce',
	authEndpoint: 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize',
	tokenEndpoint: 'https://login.microsoftonline.com/common/oauth2/v2.0/token',
	scopes: 'offline_access https://outlook.office.com/IMAP.AccessAsUser.All'
};

export const providerPresets: ProviderPreset[] = [
	{ name: 'Gmail', host: 'imap.gmail.com', port: 993, method: 'AppPassword' },
	{
		name: 'Gmail OAuth',
		host: 'imap.gmail.com',
		port: 993,
		method: 'OAuth2ByoCredentials',
		authEndpoint: GOOGLE_AUTH_ENDPOINT,
		tokenEndpoint: GOOGLE_TOKEN_ENDPOINT,
		scopes: GOOGLE_IMAP_SCOPE
	},
	microsoftPreset,
	{
		name: 'Google Workspace',
		host: 'imap.gmail.com',
		port: 993,
		method: 'OAuth2ByoCredentials',
		authEndpoint: GOOGLE_AUTH_ENDPOINT,
		tokenEndpoint: GOOGLE_TOKEN_ENDPOINT,
		scopes: GOOGLE_IMAP_SCOPE
	},
	{ name: 'iCloud', host: 'imap.mail.me.com', port: 993, method: 'AppPassword' },
	{ name: 'Yahoo', host: 'imap.mail.yahoo.com', port: 993, method: 'AppPassword' },
	{ name: 'Fastmail', host: 'imap.fastmail.com', port: 993, method: 'AppPassword' },
	{ name: 'Zoho', host: 'imap.zoho.com', port: 993, method: 'AppPassword' },
	{ name: 'UOL', host: 'imap.uol.com.br', port: 993, method: 'AppPassword' },
	{ name: 'BOL', host: 'imap.bol.com.br', port: 993, method: 'AppPassword' },
	{ name: 'Terra', host: 'imap.terra.com.br', port: 993, method: 'AppPassword' },
	{ name: 'Proton', host: '127.0.0.1', port: 1143, method: 'ProtonBridgeLocal' },
	{ name: 'Other IMAP', host: '', port: 993, method: 'AppPassword' }
];

export function oauthDefaultsForHost(host: string): {
	authEndpoint: string;
	tokenEndpoint: string;
	scopes: string;
} | null {
	const normalized = host.trim().toLowerCase();
	if (normalized === 'imap.gmail.com' || normalized === 'imap.googlemail.com') {
		return {
			authEndpoint: GOOGLE_AUTH_ENDPOINT,
			tokenEndpoint: GOOGLE_TOKEN_ENDPOINT,
			scopes: GOOGLE_IMAP_SCOPE
		};
	}
	if (normalized === 'outlook.office365.com') {
		return {
			authEndpoint: microsoftPreset.authEndpoint ?? '',
			tokenEndpoint: microsoftPreset.tokenEndpoint ?? '',
			scopes: microsoftPreset.scopes ?? ''
		};
	}
	return null;
}
