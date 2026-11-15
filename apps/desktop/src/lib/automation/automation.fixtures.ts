import type { SchedulePolicy, ScheduleState, ScheduleTransport } from '$lib/alerts/schedule';
import type { AlertsTransport, Destination, History } from '$lib/alerts/types';
import type { Category, CategoryMetadata, CategoryTransport, SenderOverride } from '$lib/categories/types';
import type { AccountConfig } from '$lib/onboarding/types';
import type { Rule, RulesTransport } from '$lib/rules/types';
import type { ReviewPreferences, ReviewTransport, SenderChoice } from '$lib/sender-review/types';
import { appStore } from '$lib/stores/app.svelte';
import { clonePlain, simulateRulesLocally, type DemoSimulationSample } from './simulator';

export interface RuleVisualMeta {
	matches_count?: number;
}

export interface HistoryVisualMeta {
	rule_name?: string;
	detail?: string;
}

export interface SenderVisualMeta {
	sender_name?: string;
	message_count?: number;
	sample_subject?: string;
}

export interface OverrideVisualMeta {
	sender_name?: string;
	reason?: CategoryMetadata['reason'];
	auto_category?: Category;
	updated_at_iso?: string;
}

export const DEMO_AUTOMATION_ACCOUNTS: AccountConfig[] = [
	{
		account_id: 'acct_work_01',
		email_display: 'work@studio.example.com',
		imap_host: 'imap.studio.example.com',
		imap_port: 993,
		auth_method: 'OAuth2PublicPkce'
	},
	{
		account_id: 'acct_personal_02',
		email_display: 'personal@vault.example.org',
		imap_host: 'mail.vault.example.org',
		imap_port: 993,
		auth_method: 'AppPassword'
	}
];

export const DEMO_SIMULATION_SAMPLES: DemoSimulationSample[] = [
	{
		id: 'msg_02_boleto',
		account_id: 'acct_personal_02',
		sender_name: 'Energia Sul Distribuidora',
		sender_email: 'billing@energia.example.org',
		subject: 'Fatura Mensal - Vencimento 20/11',
		body: 'Sua fatura de energia elétrica referente a novembro/2026 está disponível. Valor R$ 284,50.',
		attachments: [{ filename: 'fatura-nov.pdf', mime_type: 'application/pdf' }],
		amounts_cents: [28450],
		due_dates: ['2026-11-20']
	},
	{
		id: 'msg_01_otp',
		account_id: 'acct_personal_02',
		sender_name: 'Bank Auth Security',
		sender_email: 'security@bank-auth.example.com',
		subject: 'Your verification code is 492810',
		body: 'Enter one-time security code 492810 to complete your sign-in. Expires in 10 minutes.',
		attachments: [],
		amounts_cents: [],
		due_dates: []
	},
	{
		id: 'msg_03_shopping',
		account_id: 'acct_work_01',
		sender_name: 'Loja Central',
		sender_email: 'orders@loja.example.com',
		subject: 'Pedido enviado + Nota Fiscal',
		body: 'Seu pedido #8821 foi despachado via Correios (BR123456789BR) com NF-e emitida.',
		attachments: [{ filename: 'nfe-000048291.xml', mime_type: 'application/xml' }],
		amounts_cents: [64900],
		due_dates: ['2026-11-18']
	},
	{
		id: 'msg_05_promo_bill',
		account_id: 'acct_personal_02',
		sender_name: 'Promo Billing',
		sender_email: 'offers@promo.example.org',
		subject: 'Fatura promocional de assinatura anual',
		body: 'Linha digitável opcional para renovação antecipada.',
		attachments: [{ filename: 'oferta.pdf', mime_type: 'application/pdf' }],
		amounts_cents: [12000],
		due_dates: ['2026-11-28']
	}
];

export const DEMO_RULES: Rule[] = [
	{
		version: 1,
		id: 'rule_boleto_payable',
		revision: 3,
		name: 'FEBRABAN Boletos & Utility Invoices',
		enabled: true,
		position: 0,
		all: [
			{ Account: 'acct_personal_02' },
			{ AmountAtLeast: 5000 },
			{ AttachmentMime: 'application/pdf' }
		],
		any: [{ Subject: 'Fatura' }, { Body: 'linha digitável' }],
		except: [{ Domain: 'promo.example.org' }],
		actions: [{ Tag: 'Payable' }, 'Star', { Alert: 'dest_local_os' }],
		stop_processing: true
	},
	{
		version: 1,
		id: 'rule_otp_security',
		revision: 2,
		name: 'Critical Security & 2FA Alerts',
		enabled: true,
		position: 1,
		all: [{ DomainSuffix: 'bank-auth.example.com' }],
		any: [{ Subject: 'verification code' }, { Subject: '2FA' }],
		except: [],
		actions: [{ Tag: 'Security-OTP' }, { UrgentAlert: 'dest_discord_ops' }],
		stop_processing: false
	},
	{
		version: 1,
		id: 'rule_nfe_archive',
		revision: 1,
		name: 'Supplier NF-e XML Archiver',
		enabled: false,
		position: 2,
		all: [{ AttachmentName: '.xml' }],
		any: [{ Subject: 'Nota Fiscal' }, { Subject: 'NF-e' }],
		except: [],
		actions: [{ Tag: 'Fiscal-NFe' }, 'Archive'],
		stop_processing: false
	}
];

export const DEMO_RULE_META: Record<string, RuleVisualMeta> = {
	rule_boleto_payable: { matches_count: 14 },
	rule_otp_security: { matches_count: 27 },
	rule_nfe_archive: { matches_count: 9 }
};

export const DEMO_DESTINATIONS: Destination[] = [
	{
		id: 'dest_local_os',
		revision: 3,
		name: 'Native OS Notification',
		kind: 'Local',
		enabled: true,
		fields: [],
		public_origin: null,
		has_credentials: false
	},
	{
		id: 'dest_discord_ops',
		revision: 5,
		name: 'Ops Triage Channel',
		kind: 'Discord',
		enabled: true,
		fields: ['Account', 'Sender', 'Subject', 'Excerpt'],
		public_origin: 'https://discord.com',
		has_credentials: true
	},
	{
		id: 'dest_https_webhook',
		revision: 2,
		name: 'Fiscal Ledger Sink',
		kind: 'Webhook',
		enabled: true,
		fields: ['Account', 'Sender', 'Subject', 'ExtractedFacts', 'Attachments'],
		public_origin: 'https://hooks.ledger.example.org',
		has_credentials: true
	}
];

export const DEMO_SCHEDULES: Record<string, ScheduleState> = {
	dest_local_os: {
		policy: {
			destination_id: 'dest_local_os',
			destination_revision: 3,
			revision: 1,
			cadence: 'Immediate',
			zone: 'SystemLocal',
			quiet: { start: 22 * 60, end: 7 * 60 },
			urgent_bypass: true
		},
		deferred: 1,
		summarized: 0,
		expired: 0,
		overflow: 0
	},
	dest_discord_ops: {
		policy: {
			destination_id: 'dest_discord_ops',
			destination_revision: 5,
			revision: 2,
			cadence: { Hourly: { minute: 15 } },
			zone: 'SystemLocal',
			quiet: { start: 23 * 60, end: 6 * 60 + 30 },
			urgent_bypass: true
		},
		deferred: 2,
		summarized: 4,
		expired: 0,
		overflow: 0
	},
	dest_https_webhook: {
		policy: {
			destination_id: 'dest_https_webhook',
			destination_revision: 2,
			revision: 1,
			cadence: 'Immediate',
			zone: 'Utc',
			quiet: null,
			urgent_bypass: false
		},
		deferred: 0,
		summarized: 0,
		expired: 0,
		overflow: 0
	}
};

const BASE_HISTORY_MS = Date.UTC(2026, 10, 14, 17, 30, 0);

export const DEMO_HISTORY: History[] = [
	{
		event_id: 'evt_0941',
		destination_id: 'dest_local_os',
		status: 'Accepted',
		attempts: 1,
		created: BASE_HISTORY_MS,
		finished: BASE_HISTORY_MS + 420
	},
	{
		event_id: 'evt_0940',
		destination_id: 'dest_discord_ops',
		status: 'Accepted',
		attempts: 1,
		created: BASE_HISTORY_MS - 66_000,
		finished: BASE_HISTORY_MS - 65_100
	},
	{
		event_id: 'evt_0939',
		destination_id: 'dest_https_webhook',
		status: 'Failed',
		attempts: 3,
		created: BASE_HISTORY_MS - 11_700_000,
		finished: BASE_HISTORY_MS - 11_640_000
	},
	{
		event_id: 'evt_0938',
		destination_id: 'dest_discord_ops',
		status: 'Pending',
		attempts: 0,
		created: BASE_HISTORY_MS - 19_500_000,
		finished: null
	},
	{
		event_id: 'evt_0937',
		destination_id: 'dest_https_webhook',
		status: 'Canceled',
		attempts: 1,
		created: BASE_HISTORY_MS - 67_600_000,
		finished: BASE_HISTORY_MS - 67_590_000
	}
];

export const DEMO_HISTORY_META: Record<string, HistoryVisualMeta> = {
	evt_0941: {
		rule_name: 'FEBRABAN Boletos & Utility Invoices',
		detail: 'Delivered locally'
	},
	evt_0940: {
		rule_name: 'Critical Security & 2FA Alerts',
		detail: 'HTTP 204 via discord.com (UrgentAlert)'
	},
	evt_0939: {
		rule_name: 'Supplier NF-e XML Archiver',
		detail: 'TLS handshake timeout on hooks.ledger.example.org'
	},
	evt_0938: {
		rule_name: 'Critical Security & 2FA Alerts',
		detail: 'Queued by quiet-hours SchedulePolicy'
	},
	evt_0937: {
		rule_name: 'Supplier NF-e XML Archiver',
		detail: 'Superseded by destination revision rev 2'
	}
};

export const DEMO_SENDER_CHOICES: SenderChoice[] = [
	{
		account_id: 'acct_work_01',
		sender: 'orders@loja.example.com',
		revision: 1,
		state: 'Pending'
	},
	{
		account_id: 'acct_work_01',
		sender: 'contracts@vendor-cloud.example.org',
		revision: 1,
		state: 'Pending'
	},
	{
		account_id: 'acct_personal_02',
		sender: 'promo@catalog-blast.example.org',
		revision: 2,
		state: 'Declined'
	},
	{
		account_id: 'acct_personal_02',
		sender: 'events@conference-alerts.example.com',
		revision: 1,
		state: 'Declined'
	}
];

export const DEMO_SENDER_META: Record<string, SenderVisualMeta> = {
	'acct_work_01:orders@loja.example.com': {
		sender_name: 'Loja Central',
		message_count: 1,
		sample_subject: 'Pedido enviado + Nota Fiscal'
	},
	'acct_work_01:contracts@vendor-cloud.example.org': {
		sender_name: 'Vendor Cloud Billing',
		message_count: 2,
		sample_subject: 'November Workspace Seat Renewal'
	},
	'acct_personal_02:promo@catalog-blast.example.org': {
		sender_name: 'Catalog Blast',
		message_count: 3,
		sample_subject: 'Flash clearance catalog inside'
	},
	'acct_personal_02:events@conference-alerts.example.com': {
		sender_name: 'Summit Dispatch',
		message_count: 4,
		sample_subject: 'Early-bird pass closing tonight'
	}
};

export const DEMO_CATEGORY_OVERRIDES: SenderOverride[] = [
	{
		account_id: 'acct_work_01',
		sender: 'alex@studio.example.com',
		revision: 2,
		category: 'Personal'
	},
	{
		account_id: 'acct_personal_02',
		sender: 'billing@energia.example.org',
		revision: 1,
		category: 'Financial'
	},
	{
		account_id: 'acct_personal_02',
		sender: 'security@bank-auth.example.com',
		revision: 3,
		category: 'Security'
	},
	{
		account_id: 'acct_work_01',
		sender: 'digest@rustweekly.example.org',
		revision: 1,
		category: 'Newsletter'
	}
];

export const DEMO_OVERRIDE_META: Record<string, OverrideVisualMeta> = {
	'acct_work_01:alex@studio.example.com': {
		sender_name: 'Alex Rivera',
		reason: 'FinancialSignal',
		auto_category: 'Financial',
		updated_at_iso: '2026-11-13T17:12:00Z'
	},
	'acct_personal_02:billing@energia.example.org': {
		sender_name: 'Energia Sul Distribuidora',
		reason: 'FinancialSignal',
		auto_category: 'Notifications',
		updated_at_iso: '2026-11-11T10:04:00Z'
	},
	'acct_personal_02:security@bank-auth.example.com': {
		sender_name: 'Bank Auth Security',
		reason: 'SecuritySignal',
		auto_category: 'Notifications',
		updated_at_iso: '2026-11-09T14:55:00Z'
	},
	'acct_work_01:digest@rustweekly.example.org': {
		sender_name: 'Systems Engineering Weekly',
		reason: 'NewsletterSignal',
		auto_category: 'Notifications',
		updated_at_iso: '2026-11-05T08:30:00Z'
	}
};

export interface DemoAutomationBackend {
	rulesTransport: RulesTransport;
	alertsTransport: AlertsTransport;
	scheduleTransport: ScheduleTransport;
	reviewTransport: ReviewTransport;
	categoriesTransport: CategoryTransport;
	accounts: AccountConfig[];
	sampleMessages: DemoSimulationSample[];
	ruleMeta: Record<string, RuleVisualMeta>;
	historyMeta: Record<string, HistoryVisualMeta>;
	senderMeta: Record<string, SenderVisualMeta>;
	overrideMeta: Record<string, OverrideVisualMeta>;
}

export function createDemoAutomationBackend(): DemoAutomationBackend {
	let rules = clonePlain(DEMO_RULES);
	let destinations = clonePlain(DEMO_DESTINATIONS);
	const schedules: Record<string, ScheduleState> = clonePlain(DEMO_SCHEDULES);
	let history = clonePlain(DEMO_HISTORY);
	const historyMeta: Record<string, HistoryVisualMeta> = clonePlain(DEMO_HISTORY_META);
	const preferences: Record<string, ReviewPreferences> = {
		acct_work_01: {
			account_id: 'acct_work_01',
			revision: 1,
			enabled: true,
			hide_in_inbox: true,
			initializing: false
		},
		acct_personal_02: {
			account_id: 'acct_personal_02',
			revision: 1,
			enabled: true,
			hide_in_inbox: true,
			initializing: false
		}
	};
	let senderChoices = clonePlain(DEMO_SENDER_CHOICES);
	let overrides = clonePlain(DEMO_CATEGORY_OVERRIDES);

	const rulesTransport: RulesTransport = {
		list: async () => clonePlain(rules),
		save: async (rule) => {
			const idx = rules.findIndex((r) => r.id === rule.id);
			const next: Rule = {
				...clonePlain(rule),
				revision: idx >= 0 ? rules[idx].revision + 1 : 1
			};
			if (idx >= 0) {
				rules = rules.map((r) => (r.id === rule.id ? next : r));
			} else {
				rules = [...rules, next];
			}
			return clonePlain(next);
		},
		remove: async (id) => {
			rules = rules.filter((r) => r.id !== id).map((r, i) => ({ ...r, position: i }));
		},
		reorder: async (revisions) => {
			const order = new Map(revisions.map(([id], index) => [id, index]));
			rules = [...rules]
				.sort((a, b) => (order.get(a.id) ?? a.position) - (order.get(b.id) ?? b.position))
				.map((r, idx) => ({ ...r, position: idx }));
		},
		simulate: async (ids, draft) => {
			const targets = ids.length
				? DEMO_SIMULATION_SAMPLES.filter((s) => ids.includes(s.id))
				: DEMO_SIMULATION_SAMPLES;
			return simulateRulesLocally(rules, draft, targets.length ? targets : DEMO_SIMULATION_SAMPLES);
		},
		draft: async (messageId, ruleId) => {
			const sample = DEMO_SIMULATION_SAMPLES.find((s) => s.id === messageId);
			return {
				version: 1,
				id: ruleId,
				revision: 0,
				name: sample ? `Rule for ${sample.sender_email}` : 'New rule draft',
				enabled: false,
				position: rules.length,
				all: sample ? [{ Sender: sample.sender_email }] : [{ Subject: '' }],
				any: [],
				except: [],
				actions: ['Star'],
				stop_processing: false
			};
		}
	};

	const alertsTransport: AlertsTransport = {
		list: async () => clonePlain(destinations),
		save: async (destination, secret, consent) => {
			if (destination.enabled && destination.kind !== 'Local' && !consent) {
				throw new Error('Explicit consent required');
			}
			const idx = destinations.findIndex((d) => d.id === destination.id);
			const next: Destination = {
				...clonePlain(destination),
				revision: idx >= 0 ? destinations[idx].revision + 1 : 1,
				fields: destination.kind === 'Local' ? [] : [...destination.fields],
				public_origin:
					destination.kind === 'Local'
						? null
						: destination.kind === 'Telegram'
							? 'https://api.telegram.org'
							: destination.kind === 'Discord'
								? 'https://discord.com'
								: destination.public_origin,
				has_credentials:
					destination.kind !== 'Local' &&
					(Boolean(secret) || (idx >= 0 ? destinations[idx].has_credentials : false))
			};
			if (idx >= 0) {
				destinations = destinations.map((d) => (d.id === next.id ? next : d));
			} else {
				destinations = [...destinations, next];
			}
			if (!schedules[next.id]) {
				schedules[next.id] = {
					policy: {
						destination_id: next.id,
						destination_revision: next.revision,
						revision: 0,
						cadence: 'Immediate',
						zone: 'SystemLocal',
						quiet: null,
						urgent_bypass: false
					},
					deferred: 0,
					summarized: 0,
					expired: 0,
					overflow: 0
				};
			} else {
				schedules[next.id] = {
					...schedules[next.id],
					policy: {
						...schedules[next.id].policy,
						destination_revision: next.revision
					}
				};
			}
			return clonePlain(next);
		},
		remove: async (id) => {
			destinations = destinations.filter((d) => d.id !== id);
		},
		history: async () => clonePlain(history),
		test: async (id, _revision, consent) => {
			if (!consent) throw new Error('Explicit consent required');
			const dest = destinations.find((d) => d.id === id);
			const eventId = `evt_${Math.floor(1000 + Math.random() * 8999)}`;
			const now = Date.now();
			history = [
				{
					event_id: eventId,
					destination_id: id,
					status: dest?.enabled ? 'Accepted' : 'Canceled',
					attempts: dest?.enabled ? 1 : 0,
					created: now,
					finished: now
				},
				...history
			];
			historyMeta[eventId] = {
				rule_name: 'Manual Test Alert',
				detail: dest?.enabled
					? `Test alert accepted by ${dest.public_origin ?? 'local://desktop-notify'}`
					: 'Destination paused — test delivery canceled'
			};
		},
		replay: async (_ids, destinationId, _revision, consent) => {
			if (!consent) throw new Error('Explicit consent required');
			const dest = destinations.find((d) => d.id === destinationId);
			const eventId = `evt_${Math.floor(1000 + Math.random() * 8999)}`;
			const now = Date.now();
			history = [
				{
					event_id: eventId,
					destination_id: destinationId,
					status: dest?.enabled ? 'Accepted' : 'Canceled',
					attempts: 1,
					created: now,
					finished: now
				},
				...history
			];
			historyMeta[eventId] = {
				rule_name: 'Manual Replay',
				detail: `Replayed to ${dest?.public_origin ?? destinationId}`
			};
		},
		available: async () => true
	};

	const scheduleTransport: ScheduleTransport = {
		read: async (id) => {
			const dest = destinations.find((d) => d.id === id);
			const existing = schedules[id];
			if (existing) return clonePlain(existing);
			return {
				policy: {
					destination_id: id,
					destination_revision: dest?.revision ?? 1,
					revision: 0,
					cadence: 'Immediate',
					zone: 'SystemLocal',
					quiet: null,
					urgent_bypass: false
				},
				deferred: 0,
				summarized: 0,
				expired: 0,
				overflow: 0
			};
		},
		save: async (policy: SchedulePolicy, consent: boolean) => {
			if (policy.urgent_bypass && !consent) {
				throw new Error('Explicit urgent bypass consent required');
			}
			const next: SchedulePolicy = {
				...clonePlain(policy),
				revision: policy.revision + 1
			};
			schedules[policy.destination_id] = {
				...(schedules[policy.destination_id] ?? {
					deferred: 0,
					summarized: 0,
					expired: 0,
					overflow: 0
				}),
				policy: next
			};
			return clonePlain(next);
		},
		preview: async (policy) => ({
			next_epoch_ms: Date.now() + 900_000,
			timezone: policy.zone === 'Utc' ? 'UTC' : 'SystemLocal'
		})
	};

	const reviewTransport: ReviewTransport = {
		preferences: async (accountId) =>
			clonePlain(
				preferences[accountId] ?? {
					account_id: accountId,
					revision: 0,
					enabled: false,
					hide_in_inbox: false,
					initializing: false
				}
			),
		savePreferences: async (value) => {
			const next: ReviewPreferences = {
				...clonePlain(value),
				revision: value.revision + 1
			};
			preferences[value.account_id] = next;
			return clonePlain(next);
		},
		metadata: async () => null,
		list: async (accountId) =>
			clonePlain(senderChoices.filter((c) => c.account_id === accountId)),
		save: async (value) => {
			const next: SenderChoice = {
				...clonePlain(value),
				revision: value.revision + 1
			};
			const idx = senderChoices.findIndex(
				(c) => c.account_id === next.account_id && c.sender === next.sender
			);
			if (idx >= 0) {
				senderChoices = senderChoices.map((c, i) => (i === idx ? next : c));
			} else {
				senderChoices = [...senderChoices, next];
			}
			if (next.state === 'Accepted' || next.state === 'Declined' || next.state === 'Pending') {
				const mapped = next.state === 'Accepted' ? 'Approved' : next.state;
				const existsInStore = appStore.senderGovernance.some(
					(s) => s.sender_email === next.sender
				);
				if (existsInStore) {
					appStore.setSenderStatus(next.sender, mapped);
				}
			}
			return clonePlain(next);
		},
		search: async () => []
	};

	const categoriesTransport: CategoryTransport = {
		metadata: async () => {
			throw new Error('Not used in overrides list');
		},
		correct: async () => {
			throw new Error('Not used in overrides list');
		},
		list: async () => clonePlain(overrides),
		save: async (value) => {
			const next: SenderOverride = {
				...clonePlain(value),
				revision: value.revision + 1
			};
			if (next.category === null) {
				overrides = overrides.filter(
					(o) => !(o.account_id === next.account_id && o.sender === next.sender)
				);
			} else {
				const idx = overrides.findIndex(
					(o) => o.account_id === next.account_id && o.sender === next.sender
				);
				if (idx >= 0) {
					overrides = overrides.map((o, i) => (i === idx ? next : o));
				} else {
					overrides = [...overrides, next];
				}
			}
			return clonePlain(next);
		},
		search: async () => []
	};

	return {
		rulesTransport,
		alertsTransport,
		scheduleTransport,
		reviewTransport,
		categoriesTransport,
		accounts: clonePlain(DEMO_AUTOMATION_ACCOUNTS),
		sampleMessages: clonePlain(DEMO_SIMULATION_SAMPLES),
		ruleMeta: clonePlain(DEMO_RULE_META),
		historyMeta,
		senderMeta: clonePlain(DEMO_SENDER_META),
		overrideMeta: clonePlain(DEMO_OVERRIDE_META)
	};
}

export const demoAutomationBackend = createDemoAutomationBackend();
