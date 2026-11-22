import { toast } from 'svelte-sonner';
import type { AccountController } from './controller.svelte';
import type { DiscoveryController } from './discovery-controller.svelte';
import { clearCredentialInputs } from './forms';
import {
	createDemoAccountDto,
	validateDemoDiscoveryInput,
	type DemoConnectedAccountDto
} from './onboarding.fixtures';
import { normalizeSecretFor } from './secret';
import type {
	AccountConfig,
	AccountSubmission,
	AuthMethod,
	LocalArchiveFormat,
	PasswordCredentials
} from './types';

export async function runDiscoveryTestLogin(params: {
	form: HTMLFormElement;
	method: AuthMethod;
	isBridge: boolean;
	controller: AccountController;
	onResetHint: () => void;
}) {
	const { form, method, isBridge, controller, onResetHint } = params;
	if (typeof form.reportValidity === 'function' && !form.reportValidity()) return;
	const data = new FormData(form);
	const value = (key: string) => String(data.get(key) ?? '').trim();
	const config: AccountConfig = {
		account_id: '',
		email_display: value('email'),
		imap_host: isBridge ? '127.0.0.1' : value('host'),
		imap_port: isBridge ? 1143 : Number(value('port')),
		auth_method: method
	};
	const secret = normalizeSecretFor(method, String(data.get('secret') ?? ''));
	const credentials: PasswordCredentials = isBridge
		? { ProtonBridgeLocal: { bridge_password: secret } }
		: { AppPassword: { secret } };
	const pending = controller.testConnection(config, credentials);
	for (const key of ['secret', 'client']) data.delete(key);
	clearCredentialInputs(form);
	onResetHint();
	await pending;
}

export async function submitDiscoveryAccount(params: {
	form: HTMLFormElement;
	method: AuthMethod;
	oauth: boolean;
	isBridge: boolean;
	isArchive: boolean;
	demoMode: boolean;
	demoPkceAuthorized: boolean;
	archivePath: string;
	archiveFormat: LocalArchiveFormat;
	controller: AccountController;
	discovery: DiscoveryController;
	archiveErrorMsg: string;
	archiveSuccessPrefix: string;
	onAccountAdded?: (account: DemoConnectedAccountDto) => void;
	onArchiveImported: () => void;
	onDemoReset: () => void;
	onResetHint: () => void;
	onNativeSuccess: () => void;
}) {
	const {
		form,
		method,
		oauth,
		isBridge,
		isArchive,
		demoMode,
		demoPkceAuthorized,
		archivePath,
		archiveFormat,
		controller,
		discovery,
		archiveErrorMsg,
		archiveSuccessPrefix,
		onAccountAdded,
		onArchiveImported,
		onDemoReset,
		onResetHint,
		onNativeSuccess
	} = params;

	discovery.invalidate();
	const data = new FormData(form);
	const value = (key: string) => String(data.get(key) ?? '').trim();

	if (isArchive) {
		const cleanEmail = value('email');
		clearCredentialInputs(form);
		if (!cleanEmail || !archivePath) {
			toast.error(archiveErrorMsg);
			return;
		}
		if (demoMode) {
			onAccountAdded?.(createDemoAccountDto(cleanEmail, '', 0, 'LocalArchive'));
			onArchiveImported();
			return;
		}
		const imported = await controller.importArchive(archivePath, cleanEmail, archiveFormat);
		if (imported) {
			toast.success(`${archiveSuccessPrefix} ${imported.imported_count}`);
			onArchiveImported();
			onNativeSuccess();
		}
		return;
	}

	if (demoMode) {
		const cleanEmail = value('email');
		const finalHost = isBridge ? '127.0.0.1' : value('host');
		const finalPort = isBridge ? 1143 : Number(value('port')) || 993;
		const secretLen = normalizeSecretFor(method, String(data.get('secret') ?? '')).trim().length;
		const clientLen = String(data.get('client') ?? '').trim().length;
		for (const key of ['secret', 'client']) data.delete(key);
		clearCredentialInputs(form);
		const err = validateDemoDiscoveryInput({
			email: cleanEmail,
			host: finalHost,
			authMethod: method,
			appSecretLen: method === 'AppPassword' ? secretLen : 0,
			bridgeSecretLen: method === 'ProtonBridgeLocal' ? secretLen : 0,
			pkceAuthorized: demoPkceAuthorized,
			clientLen
		});
		if (err) {
			toast.error(err);
			return;
		}
		onAccountAdded?.(createDemoAccountDto(cleanEmail, finalHost, finalPort, method));
		onDemoReset();
		return;
	}

	const config: AccountConfig = {
		account_id: '',
		email_display: value('email'),
		imap_host: isBridge ? '127.0.0.1' : value('host'),
		imap_port: isBridge ? 1143 : Number(value('port')),
		auth_method: method
	};
	const submission: AccountSubmission = oauth
		? {
				authorization: {
					authorization_endpoint: value('authorize'),
					token_endpoint: value('token'),
					scopes: value('scopes'),
					client_id: value('client'),
					...(method === 'OAuth2ByoCredentials'
						? { client_secret: String(data.get('secret') ?? '') }
						: {})
				}
			}
		: {
				credentials: isBridge
					? {
							ProtonBridgeLocal: {
								bridge_password: normalizeSecretFor(method, String(data.get('secret') ?? ''))
							}
						}
					: { AppPassword: { secret: normalizeSecretFor(method, String(data.get('secret') ?? '')) } }
			};
	const pending = controller.add(config, submission);
	for (const key of ['secret', 'client']) data.delete(key);
	clearCredentialInputs(form);
	onResetHint();
	if (await pending) onNativeSuccess();
}
