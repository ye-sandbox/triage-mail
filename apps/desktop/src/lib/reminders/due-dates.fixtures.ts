import type { AlertsTransport, Destination } from '../alerts/types';
import { appStore } from '../stores/app.svelte';
import type { EmailMessage } from '../types';
import {
	countOverdueAndDueToday,
	type DueDateItem,
	type RemindersTransport,
	type ScheduledReminderConfig
} from './types';

function parseSenderAddress(raw: string): { name: string; email: string } {
	const trimmed = raw.trim();
	if (!trimmed) return { name: '', email: '' };
	const match = /^(.*?)\s*<([^<>]+)>\s*$/.exec(trimmed);
	if (match) {
		const name = match[1].replace(/^["']|["']$/g, '').trim();
		const email = match[2].trim();
		return { name: name || email, email };
	}
	return { name: trimmed, email: trimmed.includes('@') ? trimmed : '' };
}

function offsetIsoDate(deltaDays: number, base = new Date()): string {
	const utc = Date.UTC(base.getUTCFullYear(), base.getUTCMonth(), base.getUTCDate() + deltaDays);
	return new Date(utc).toISOString().slice(0, 10);
}

export function createDueDateFixtures(baseDate = new Date()): DueDateItem[] {
	const todayMs = Date.UTC(
		baseDate.getUTCFullYear(),
		baseDate.getUTCMonth(),
		baseDate.getUTCDate(),
		12,
		0,
		0
	);
	return [
		{
			message_id: 'msg_due_energia',
			account_id: 'acct_personal_02',
			subject: 'Fatura de Energia Elétrica — Outubro/2026',
			sender: 'Energia Utility <energia@utility.example.org>',
			received_at_epoch_ms: todayMs - 7 * 86_400_000,
			is_read: true,
			digitable_line: '34191.79001 01043.510047 91020.150008 8 99030000031240',
			amount_cents: 31240,
			due_date_iso: offsetIsoDate(-3, baseDate),
			is_dismissed: false
		},
		{
			message_id: 'msg_due_fibra',
			account_id: 'acct_work_01',
			subject: 'Fatura Fibra Empresarial — Vencimento Hoje',
			sender: 'Fibra Telecom <fibra@telecom.example.com>',
			received_at_epoch_ms: todayMs - 4 * 86_400_000,
			is_read: false,
			digitable_line: '23793.38128 60000.000003 01000.149904 1 99060000014990',
			amount_cents: 14990,
			due_date_iso: offsetIsoDate(0, baseDate),
			is_dismissed: false
		},
		{
			message_id: 'msg_02_boleto',
			account_id: 'acct_personal_02',
			subject: 'Fatura Mensal - Vencimento 20/11',
			sender: 'Energia Sul Distribuidora <billing@energia.example.org>',
			received_at_epoch_ms: todayMs - 2 * 86_400_000,
			is_read: false,
			digitable_line: '34191.79001 01043.510047 91020.150008 8 99120000028450',
			amount_cents: 28450,
			due_date_iso: offsetIsoDate(5, baseDate),
			is_dismissed: false
		},
		{
			message_id: 'msg_due_cloud',
			account_id: 'acct_work_01',
			subject: 'Invoice #IC-2026-1109 — Compute & Object Storage',
			sender: 'Infra Cloud Billing <cloud@infra.example.net>',
			received_at_epoch_ms: todayMs - 86_400_000,
			is_read: true,
			digitable_line: '03399.12345 67890.123456 78901.234567 3 99150000008900',
			amount_cents: 8900,
			due_date_iso: offsetIsoDate(9, baseDate),
			is_dismissed: false
		},
		{
			message_id: 'msg_due_seguro',
			account_id: 'acct_personal_02',
			subject: 'Parcela 04/10 — Apólice Auto #99412',
			sender: 'Seguro Auto Proteção <seguro@auto.example.org>',
			received_at_epoch_ms: todayMs - 5 * 86_400_000,
			is_read: true,
			digitable_line: '10498.76543 21098.765432 10987.654321 7 99080000042000',
			amount_cents: 42000,
			due_date_iso: offsetIsoDate(2, baseDate),
			is_dismissed: true
		}
	];
}

export const DEMO_ALERT_DESTINATIONS: Destination[] = [
	{
		id: 'dest_desktop',
		revision: 1,
		name: 'Native OS Notification',
		kind: 'Local',
		enabled: true,
		fields: ['Account', 'Sender', 'Subject', 'ExtractedFacts'],
		public_origin: null,
		has_credentials: false
	}
];

export const DEMO_REMINDER_CONFIGS: ScheduledReminderConfig[] = [
	{
		version: 1,
		id: 'rem_due_3d',
		revision: 1,
		name: 'Boletos 3 days before due date',
		enabled: true,
		trigger: {
			DueDate: {
				days_before: 3,
				target_hour: 9,
				min_amount_cents: 5000
			}
		},
		destination_ids: ['dest_desktop']
	},
	{
		version: 1,
		id: 'rem_due_day',
		revision: 1,
		name: 'Same-day due date alert',
		enabled: true,
		trigger: {
			DueDate: {
				days_before: 0,
				target_hour: 8
			}
		},
		destination_ids: ['dest_desktop']
	}
];

export function buildInboxMessageFromDueItem(item: DueDateItem): EmailMessage {
	const { name, email } = parseSenderAddress(item.sender);
	const amountLabel =
		item.amount_cents === null
			? 'Amount not specified'
			: new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' }).format(
					item.amount_cents / 100
				);
	return {
		id: item.message_id,
		thread_id: `thr_${item.message_id}`,
		account_id: item.account_id,
		folder: 'INBOX',
		sender_name: name || item.sender,
		sender_email: email || item.sender,
		sender_status: 'Approved',
		subject: item.subject,
		snippet: `${name || item.sender} — ${amountLabel} · Due ${item.due_date_iso}`,
		received_at_iso: new Date(item.received_at_epoch_ms).toISOString(),
		unread: !item.is_read,
		starred: false,
		snoozed_until_iso: null,
		category: 'Financial',
		tags: ['Boleto', 'Extracted'],
		extractions: {
			boleto: {
				amount_cents: item.amount_cents ?? 0,
				due_date_iso: item.due_date_iso,
				digitable_line: item.digitable_line,
				beneficiary: name || item.sender,
				paid: item.is_dismissed
			}
		},
		attachments_meta: [],
		html_raw: `<div><p><strong>${name || item.sender}</strong></p><p>${item.subject}</p><p>Amount: <strong>${amountLabel}</strong> · Due: <strong>${item.due_date_iso}</strong></p><p><code>${item.digitable_line}</code></p></div>`,
		text_fallback: `${name || item.sender} - ${item.subject}. ${amountLabel} due ${item.due_date_iso}. Line: ${item.digitable_line}`
	};
}

export interface DemoRemindersBackend {
	remindersTransport: RemindersTransport;
	alertsTransport: AlertsTransport;
	getItems: () => DueDateItem[];
	findMessage: (messageId: string) => EmailMessage | undefined;
}

export function createDemoRemindersBackend(): DemoRemindersBackend {
	let items = createDueDateFixtures();
	let configs: ScheduledReminderConfig[] = structuredClone(DEMO_REMINDER_CONFIGS);
	const destinations: Destination[] = structuredClone(DEMO_ALERT_DESTINATIONS);

	const remindersTransport: RemindersTransport = {
		listDueDates: async (accountId) =>
			structuredClone(accountId ? items.filter((it) => it.account_id === accountId) : items),
		dismissDueDate: async (messageId) => {
			items = items.map((it) =>
				it.message_id === messageId ? { ...it, is_dismissed: true } : it
			);
			appStore.overdueAndDueTodayCount = countOverdueAndDueToday(items);
		},
		listConfigs: async () => structuredClone(configs),
		saveConfig: async (config) => {
			const idx = configs.findIndex((c) => c.id === config.id);
			const next: ScheduledReminderConfig = {
				...structuredClone(config),
				revision: idx >= 0 ? configs[idx].revision + 1 : 1
			};
			if (idx >= 0) {
				configs = configs.map((c) => (c.id === config.id ? next : c));
			} else {
				configs = [next, ...configs];
			}
			return structuredClone(next);
		},
		deleteConfig: async (id) => {
			configs = configs.filter((c) => c.id !== id);
		}
	};

	const alertsTransport: AlertsTransport = {
		list: async () => structuredClone(destinations),
		save: async (d) => d,
		remove: async () => {},
		history: async () => [],
		test: async () => {},
		replay: async () => {},
		available: async () => true
	};

	return {
		remindersTransport,
		alertsTransport,
		getItems: () => items,
		findMessage: (messageId) => {
			const match = items.find((it) => it.message_id === messageId);
			return match ? buildInboxMessageFromDueItem(match) : undefined;
		}
	};
}

export const demoRemindersBackend = createDemoRemindersBackend();
