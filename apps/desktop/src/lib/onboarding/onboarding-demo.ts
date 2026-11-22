import { toast } from 'svelte-sonner';
import { appStore } from '../stores/app.svelte';
import type { DemoConnectedAccountDto } from './onboarding.fixtures';

export function buildShellAccountFromDemo(account: DemoConnectedAccountDto) {
	return {
		account_id: account.account_id,
		display_name: account.email_display.split('@')[0] ?? account.email_display,
		address: account.email_display,
		imap_host: account.imap_host,
		imap_port: account.imap_port,
		tls_mode: account.tls_mode === 'STARTTLS' ? ('STARTTLS' as const) : ('IMAPS' as const),
		auth_method:
			account.auth_method === 'LocalArchive'
				? ('LocalArchive' as const)
				: account.auth_method === 'AppPassword'
					? ('AppPassword-Vault' as const)
					: ('OAuth2-PKCE' as const),
		vault_locked: false,
		folders: [
			{ id: 'INBOX', label: 'INBOX' },
			{ id: 'Archive', label: 'Archive' },
			{ id: 'Trash', label: 'Trash' }
		]
	};
}

export function notifyDemoUnlock(method: 'passphrase' | 'keyring' | 'backup', detail?: string) {
	appStore.vaultUnlocked = true;
	if (method === 'passphrase') {
		toast.success('SQLCipher vault unlocked via Argon2id + AES-GCM');
		return 'discovery' as const;
	}
	if (method === 'keyring') {
		toast.success('OS Secret Service / Keychain unlocked local vault');
		return 'directory' as const;
	}
	toast.success(`Restored vault metadata from ${detail ?? 'backup'}`);
	return 'directory' as const;
}
