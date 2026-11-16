export type Locale = 'en' | 'pt-BR';
export type ThemeMode = 'light' | 'dark';

export type SyncStatus =
	| 'Idle'
	| 'Syncing'
	| 'Retrying'
	| 'MutationPending'
	| 'MutationFailed'
	| 'ResyncRequired';

export type FolderId = 'INBOX' | 'Archive' | 'Trash' | (string & {});

export type MessageCategory =
	| 'Security'
	| 'Financial'
	| 'Shopping'
	| 'Newsletter'
	| 'Notifications'
	| 'Personal';

export type SearchMode = 'lexical' | 'meaning' | 'hybrid';

export type SenderStatus = 'Approved' | 'Pending' | 'Declined';

export interface AccountFolder {
	id: FolderId;
	label: string;
}

export interface MailAccount {
	account_id: string;
	display_name: string;
	address: string;
	imap_host: string;
	imap_port: number;
	tls_mode: 'IMAPS' | 'STARTTLS';
	auth_method: 'OAuth2-PKCE' | 'AppPassword-Vault' | 'LocalArchive';
	vault_locked: boolean;
	folders: AccountFolder[];
}

export interface OtpExtraction {
	code: string;
	issuer?: string;
}

export interface FebrabanBoletoExtraction {
	amount_cents: number;
	due_date_iso: string;
	digitable_line: string;
	beneficiary?: string;
	paid?: boolean;
}

export interface PixExtraction {
	payload: string;
	amount_cents?: number;
	recipient?: string;
}

export interface NfeExtraction {
	access_key: string;
	invoice_number: string;
	issuer?: string;
	amount_cents?: number;
}

export interface TrackingExtraction {
	carrier: string;
	code: string;
	status_hint?: string;
}

export interface UnsubscribeMeta {
	supports_one_click: boolean;
	endpoint_hint: string;
	unsubscribed?: boolean;
}

export interface AttachmentMeta {
	id: string;
	filename: string;
	mime_type: string;
	size_bytes: number;
	downloaded?: boolean;
}

export interface MessageExtractions {
	otp?: OtpExtraction;
	boleto?: FebrabanBoletoExtraction;
	pix?: PixExtraction;
	nfe?: NfeExtraction;
	tracking?: TrackingExtraction;
	unsubscribe?: UnsubscribeMeta;
}

export interface EmailMessage {
	id: string;
	thread_id: string;
	account_id: string;
	folder: FolderId;
	sender_name: string;
	sender_email: string;
	sender_status: SenderStatus;
	subject: string;
	snippet: string;
	received_at_iso: string;
	unread: boolean;
	starred: boolean;
	snoozed_until_iso: string | null;
	category: MessageCategory;
	tags: string[];
	extractions: MessageExtractions;
	attachments_meta: AttachmentMeta[];
	html_raw: string;
	text_fallback: string;
}

export type ScopeFilter =
	| { kind: 'all' }
	| { kind: 'inbox' }
	| { kind: 'starred' }
	| { kind: 'snoozed' }
	| { kind: 'folder'; account_id: string; folder: FolderId }
	| { kind: 'new_senders'; status: 'Pending' | 'Declined' }
	| { kind: 'category'; category: MessageCategory }
	| { kind: 'saved_view'; view_id: string };

export interface SavedView {
	id: string;
	name: string;
	query: string;
	category?: MessageCategory;
	has_boleto?: boolean;
}

export interface AutomationRule {
	id: string;
	name: string;
	enabled: boolean;
	match_sender_domain?: string;
	match_category?: MessageCategory;
	match_subject_contains?: string;
	action_tag?: string;
	action_archive?: boolean;
	action_alert_destination_id?: string;
	matches_count: number;
}

export interface AlertDestination {
	id: string;
	name: string;
	kind: 'DesktopNotification' | 'LocalWebhook' | 'MatrixBridge';
	target: string;
	enabled: boolean;
}

export interface SenderGovernanceEntry {
	sender_email: string;
	sender_name: string;
	status: SenderStatus;
	first_seen_iso: string;
	message_count: number;
}

export type DueDateStatus = 'Overdue' | 'DueToday' | 'Upcoming' | 'Dismissed';

export interface DueDateItem {
	id: string;
	message_id: string;
	account_id: string;
	sender_name: string;
	sender_email: string;
	beneficiary: string;
	subject: string;
	amount_cents: number;
	due_date_iso: string;
	status: DueDateStatus;
	digitable_line: string;
	extracted_at_iso: string;
}

export interface ReminderConfig {
	enabled: boolean;
	thresholds_days: number[];
	notify_overdue_daily: boolean;
	quiet_hours_start: string;
	quiet_hours_end: string;
	destination_id: string;
}
