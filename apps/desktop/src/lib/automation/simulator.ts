import type { Destination } from '$lib/alerts/types';
import type {
	Condition,
	ConditionResult,
	Field,
	Outcome,
	Plan,
	Rule,
	RuleAction
} from '$lib/rules/types';

export interface DemoSimulationSample {
	id: string;
	account_id: string;
	sender_name: string;
	sender_email: string;
	subject: string;
	body: string;
	attachments: { filename: string; mime_type: string }[];
	amounts_cents: number[];
	due_dates: string[];
}

export function clonePlain<T>(value: T): T {
	return JSON.parse(JSON.stringify(value)) as T;
}

export function extractConditionEntry(cond: Condition): { field: Field; value: string } {
	const [field] = Object.keys(cond) as [Field];
	const raw = (cond as Record<string, string | number>)[field];
	return { field, value: raw === undefined ? '' : String(raw) };
}

export function buildCondition(field: Field, value: string): Condition {
	if (field === 'AmountAtLeast') return { AmountAtLeast: Math.max(0, Math.trunc(Number(value) || 0)) };
	if (field === 'AmountAtMost') return { AmountAtMost: Math.max(0, Math.trunc(Number(value) || 0)) };
	return { [field]: value } as Condition;
}

export function conditionPlaceholder(field: Field): string {
	switch (field) {
		case 'Account':
			return 'acct_personal_02';
		case 'Sender':
			return 'billing@energia.example.org';
		case 'Domain':
		case 'DomainSuffix':
			return 'energia.example.org';
		case 'AttachmentName':
			return '.pdf or fatura';
		case 'AttachmentMime':
			return 'application/pdf';
		case 'AmountAtLeast':
		case 'AmountAtMost':
			return '5000';
		case 'DueOnOrAfter':
		case 'DueOnOrBefore':
			return '2026-11-20';
		default:
			return 'Match text…';
	}
}

export function formatRuleAction(action: RuleAction, destinations: Destination[]): string {
	if (action === 'Star' || action === 'Archive') return action;
	if ('Tag' in action) return `Tag("${action.Tag}")`;
	if ('Alert' in action) {
		const name = destinations.find((d) => d.id === action.Alert)?.name ?? action.Alert;
		return `Alert(${name})`;
	}
	const name = destinations.find((d) => d.id === action.UrgentAlert)?.name ?? action.UrgentAlert;
	return `UrgentAlert(${name})`;
}

export function validateWebhookOrigin(rawInput: string): { origin?: string; error?: string } {
	const trimmed = rawInput.trim();
	if (!trimmed) {
		return { error: 'Enter a valid public HTTPS domain (e.g. hooks.ledger.example.org).' };
	}
	if (/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(trimmed) && !trimmed.toLowerCase().startsWith('https://')) {
		return { error: 'Webhook must use HTTPS (port 443).' };
	}
	try {
		const url = new URL(trimmed.toLowerCase().startsWith('https://') ? trimmed : `https://${trimmed}`);
		if (url.protocol !== 'https:') return { error: 'Webhook must use HTTPS (port 443).' };
		if (url.port && url.port !== '443') return { error: 'Webhook only permits port 443.' };
		const host = url.hostname.toLowerCase();
		if (
			!host ||
			host === 'localhost' ||
			host.endsWith('.localhost') ||
			/^(\d{1,3}\.){3}\d{1,3}$/.test(host) ||
			host.includes(':')
		) {
			return {
				error: 'Webhook requires a public DNS hostname (IP literals and localhost are blocked).'
			};
		}
		return { origin: url.origin };
	} catch {
		return { error: 'Enter a valid public HTTPS domain (e.g. hooks.ledger.example.org).' };
	}
}

function evalCondition(cond: Condition, msg: DemoSimulationSample): ConditionResult {
	const { field, value } = extractConditionEntry(cond);
	const trimmed = value.trim();
	const lower = trimmed.toLowerCase();
	const domain = msg.sender_email.split('@')[1]?.toLowerCase() ?? '';

	switch (field) {
		case 'Account':
			return msg.account_id === trimmed ? 'Matched' : 'NotMatched';
		case 'Sender':
			return msg.sender_email.toLowerCase() === lower ? 'Matched' : 'NotMatched';
		case 'Domain':
			if (!domain) return 'MissingFact';
			return domain === lower ? 'Matched' : 'NotMatched';
		case 'DomainSuffix':
			if (!domain) return 'MissingFact';
			return domain === lower || domain.endsWith(`.${lower}`) ? 'Matched' : 'NotMatched';
		case 'Subject':
			return msg.subject.toLowerCase().includes(lower) ? 'Matched' : 'NotMatched';
		case 'Body':
			return msg.body.toLowerCase().includes(lower) ? 'Matched' : 'NotMatched';
		case 'AttachmentName':
			return msg.attachments.some((a) => a.filename.toLowerCase().includes(lower))
				? 'Matched'
				: 'NotMatched';
		case 'AttachmentMime':
			return msg.attachments.some((a) => a.mime_type.toLowerCase().includes(lower))
				? 'Matched'
				: 'NotMatched';
		case 'AmountAtLeast': {
			if (msg.amounts_cents.length === 0) return 'MissingFact';
			const threshold = Number(trimmed);
			return msg.amounts_cents.some((a) => a >= threshold) ? 'Matched' : 'NotMatched';
		}
		case 'AmountAtMost': {
			if (msg.amounts_cents.length === 0) return 'MissingFact';
			const threshold = Number(trimmed);
			return msg.amounts_cents.some((a) => a <= threshold) ? 'Matched' : 'NotMatched';
		}
		case 'DueOnOrAfter':
			if (msg.due_dates.length === 0) return 'MissingFact';
			return msg.due_dates.some((d) => d >= trimmed) ? 'Matched' : 'NotMatched';
		case 'DueOnOrBefore':
			if (msg.due_dates.length === 0) return 'MissingFact';
			return msg.due_dates.some((d) => d <= trimmed) ? 'Matched' : 'NotMatched';
	}
}

export function simulateRulesLocally(
	rules: Rule[],
	draft: Rule | undefined,
	samples: DemoSimulationSample[]
): [string, Plan][] {
	const merged = draft
		? rules.some((r) => r.id === draft.id)
			? rules.map((r) => (r.id === draft.id ? draft : r))
			: [...rules, draft]
		: [...rules];
	const ordered = [...merged].sort((a, b) =>
		a.position !== b.position ? a.position - b.position : a.id.localeCompare(b.id)
	);

	return samples.map((msg) => {
		const actions: RuleAction[] = [];
		const explanations: Plan['explanations'] = [];
		let stopped = false;

		for (const rule of ordered) {
			const all = rule.all.map((c) => evalCondition(c, msg));
			const any = rule.any.map((c) => evalCondition(c, msg));
			const except = rule.except.map((c) => evalCondition(c, msg));

			let outcome: Outcome;
			if (stopped) {
				outcome = 'Stopped';
			} else if (!rule.enabled) {
				outcome = 'Disabled';
			} else {
				const allOk = all.every((r) => r === 'Matched');
				const anyOk = any.length === 0 || any.some((r) => r === 'Matched');
				const excluded = except.some((r) => r === 'Matched');

				if (excluded && allOk && anyOk) {
					outcome = 'Excluded';
				} else if (allOk && anyOk && !excluded) {
					outcome = 'Matched';
					actions.push(...rule.actions);
					if (rule.stop_processing) stopped = true;
				} else {
					outcome = 'NotMatched';
				}
			}

			explanations.push({
				rule_id: rule.id,
				outcome,
				all,
				any,
				except
			});
		}

		return [msg.id, { actions, explanations }];
	});
}
