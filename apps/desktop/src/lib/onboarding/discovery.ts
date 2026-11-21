import { invoke } from '@tauri-apps/api/core';
import type { AuthMethod } from './types';

export interface DiscoveredProvider {
	domain: string;
	provider_name: string;
	imap_host: string | null;
	imap_port: number;
	use_ssl: boolean;
	recommended_auth: AuthMethod;
	oauth_endpoints: {
		authorization_endpoint: string;
		token_endpoint: string;
		scopes: string;
	} | null;
	help_url: string | null;
	source: 'Preset' | 'Ispdb' | 'Mx' | 'Unresolved';
}
export interface DiscoveryTransport {
	discover(email: string, online: boolean): Promise<DiscoveredProvider>;
	probe(host: string): Promise<boolean>;
	detectMx(email: string): Promise<DiscoveredProvider>;
}
export const nativeDiscoveryTransport: DiscoveryTransport = {
	discover: (email, online) => invoke('cmd_discover_email_provider', { email, online }),
	probe: (host) => invoke('cmd_probe_email_provider', { host }),
	detectMx: (email) => invoke('cmd_detect_email_provider_mx', { email })
};
