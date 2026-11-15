import type {
	AlertDestination,
	AutomationRule,
	MailAccount,
	SavedView,
	SenderGovernanceEntry
} from '$lib/types';

export const SHELL_ACCOUNT_FIXTURES: MailAccount[] = [
	{
		account_id: 'acct_work_01',
		display_name: 'Studio Work',
		address: 'work@studio.example.com',
		imap_host: 'imap.studio.example.com',
		imap_port: 993,
		tls_mode: 'IMAPS',
		auth_method: 'OAuth2-PKCE',
		vault_locked: false,
		folders: [
			{ id: 'INBOX', label: 'INBOX' },
			{ id: 'Archive', label: 'Archive' },
			{ id: 'Trash', label: 'Trash' }
		]
	},
	{
		account_id: 'acct_personal_02',
		display_name: 'Personal Vault',
		address: 'personal@vault.example.org',
		imap_host: 'mail.vault.example.org',
		imap_port: 993,
		tls_mode: 'IMAPS',
		auth_method: 'AppPassword-Vault',
		vault_locked: false,
		folders: [
			{ id: 'INBOX', label: 'INBOX' },
			{ id: 'Archive', label: 'Archive' },
			{ id: 'Trash', label: 'Trash' }
		]
	}
];

export const SHELL_SAVED_VIEW_FIXTURES: SavedView[] = [
	{
		id: 'view_boletos_open',
		name: 'Open Boletos & Bills',
		query: '',
		category: 'Financial',
		has_boleto: true
	},
	{
		id: 'view_security_codes',
		name: '2FA & Auth Codes',
		query: '',
		category: 'Security'
	}
];

export const SHELL_RULE_FIXTURES: AutomationRule[] = [
	{
		id: 'rule_boleto_tag',
		name: 'Tag FEBRABAN Boletos as Payable',
		enabled: true,
		match_category: 'Financial',
		match_subject_contains: 'Fatura',
		action_tag: 'Payable',
		action_alert_destination_id: 'dest_desktop',
		matches_count: 14
	}
];

export const SHELL_ALERT_DESTINATION_FIXTURES: AlertDestination[] = [
	{
		id: 'dest_desktop',
		name: 'Native OS Notification',
		kind: 'DesktopNotification',
		target: 'local://system-notify',
		enabled: true
	}
];

export const SHELL_SENDER_GOVERNANCE_FIXTURES: SenderGovernanceEntry[] = [
	{
		sender_email: 'orders@loja.example.com',
		sender_name: 'Loja Central',
		status: 'Pending',
		first_seen_iso: '2026-11-14T11:15:00Z',
		message_count: 1
	}
];
