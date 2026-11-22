import { tick } from 'svelte';
import { toast } from 'svelte-sonner';
import type { AccountController } from './controller.svelte';
import type { DiscoveryController } from './discovery-controller.svelte';
import { clearCredentialInputs } from './forms';
import { MAX_GOOGLE_CLIENT_JSON_BYTES, parseGoogleOAuthClientJson } from './google-oauth-json';
import {
	extractDomainOnly,
	lookupIspdbDomain,
	type IspdbPreset
} from './onboarding.fixtures';

function readFileText(file: File): Promise<string> {
	if (typeof file.text === 'function') return file.text();
	return new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onload = () => resolve(typeof reader.result === 'string' ? reader.result : '');
		reader.onerror = () => reject(reader.error ?? new Error('Failed to read file'));
		reader.readAsText(file);
	});
}

export async function importGoogleClientJsonFile(params: {
	event: Event;
	form: HTMLFormElement | undefined;
	controller: AccountController;
	discovery: DiscoveryController;
	messages: { invalid: string; webUnsupported: string; success: string };
	onImported: () => void;
}) {
	const { event, form, controller, discovery, messages, onImported } = params;
	const input = event.currentTarget as HTMLInputElement;
	const file = input.files?.[0];
	input.value = '';
	if (!file) return;
	if (file.size > MAX_GOOGLE_CLIENT_JSON_BYTES) return void toast.error(messages.invalid);
	const parsed = parseGoogleOAuthClientJson(await readFileText(file));
	if (!parsed.ok) {
		return void toast.error(
			parsed.reason === 'web_client_unsupported' ? messages.webUnsupported : messages.invalid
		);
	}
	clearCredentialInputs(form);
	controller.clearDiagnostics();
	discovery.edit();
	discovery.selectedPreset = 'Gmail OAuth';
	discovery.settings = {
		host: 'imap.gmail.com',
		port: 993,
		method: 'OAuth2ByoCredentials',
		authorize: parsed.authEndpoint,
		token: parsed.tokenEndpoint,
		scopes: parsed.scopes
	};
	onImported();
	await tick();
	// INVARIANT: Imported client_id/client_secret live only in unbound DOM inputs, never Runes state.
	const clientEl = form?.querySelector<HTMLInputElement>('input[name="client"]');
	const secretEl = form?.querySelector<HTMLInputElement>('input[name="secret"]');
	if (clientEl) clientEl.value = parsed.clientId;
	if (secretEl) secretEl.value = parsed.clientSecret;
	toast.success(messages.success);
}

export function performDiscoveryLookup(params: {
	emailValue: string;
	demoMode: boolean;
	discovery: DiscoveryController;
	invalidMsg: string;
	onDemoIspdb: (found: IspdbPreset) => void;
}) {
	const { emailValue, demoMode, discovery, invalidMsg, onDemoIspdb } = params;
	if (!demoMode) {
		void discovery.online(emailValue);
		return;
	}
	const found = lookupIspdbDomain(emailValue);
	if (!found) {
		toast.error(invalidMsg);
		return;
	}
	onDemoIspdb(found);
	discovery.edit();
	discovery.settings = {
		host: found.imap_host,
		port: found.imap_port,
		method: found.recommended_auth,
		authorize: found.auth_endpoint ?? '',
		token: found.token_endpoint ?? '',
		scopes: found.default_scopes ?? ''
	};
	toast.success(`ISPDB: ${extractDomainOnly(emailValue)}`);
}
