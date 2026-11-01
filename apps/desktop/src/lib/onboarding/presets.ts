import type { AuthMethod } from './types';

export interface ProviderPreset {
	name: string;
	host: string;
	port: number;
	method: AuthMethod;
	authEndpoint?: string;
	tokenEndpoint?: string;
	scopes?: string;
}

export const providerPresets: ProviderPreset[] = [
	{ name: 'Gmail', host: 'imap.gmail.com', port: 993, method: 'AppPassword' },
	{
		name: 'Gmail OAuth',
		host: 'imap.gmail.com',
		port: 993,
		method: 'OAuth2ByoCredentials',
		authEndpoint: 'https://accounts.google.com/o/oauth2/v2/auth',
		tokenEndpoint: 'https://oauth2.googleapis.com/token',
		scopes: 'https://mail.google.com/'
	},
	{
		name: 'Outlook',
		host: 'outlook.office365.com',
		port: 993,
		method: 'OAuth2PublicPkce',
		authEndpoint: 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize',
		tokenEndpoint: 'https://login.microsoftonline.com/common/oauth2/v2.0/token',
		scopes: 'offline_access https://outlook.office.com/IMAP.AccessAsUser.All'
	},
	{ name: 'iCloud', host: 'imap.mail.me.com', port: 993, method: 'AppPassword' },
	{ name: 'Yahoo', host: 'imap.mail.yahoo.com', port: 993, method: 'AppPassword' },
	{ name: 'Fastmail', host: 'imap.fastmail.com', port: 993, method: 'AppPassword' },
	{ name: 'Proton', host: '127.0.0.1', port: 1143, method: 'ProtonBridgeLocal' }
];
