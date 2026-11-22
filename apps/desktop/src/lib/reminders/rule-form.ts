import type { ScheduledReminderConfig } from './types';

export interface ReminderFormInput {
	initial: ScheduledReminderConfig;
	name: string;
	enabled: boolean;
	triggerKind: 'DueDate' | 'UnreadInterval';
	daysBefore: number;
	targetHour: number;
	intervalHours: number;
	minAmountStr: string;
	selectedAccount: string;
	selectedDestinations: string[];
}

export interface ReminderValidationMessages {
	invalidRuleName: string;
	noDestinations: string;
	invalidDaysRange: string;
	invalidHourRange: string;
	invalidIntervalRange: string;
}

export function parseMinAmountCents(val: string): number | undefined {
	const clean = val.replace(',', '.').trim();
	if (!clean) return undefined;
	const parsed = Number.parseFloat(clean);
	if (Number.isNaN(parsed) || parsed <= 0) return undefined;
	return Math.round(parsed * 100);
}

export function buildValidatedReminderConfig(
	input: ReminderFormInput,
	messages: ReminderValidationMessages
): { ok: true; value: ScheduledReminderConfig } | { ok: false; error: string } {
	const trimmedName = input.name.trim();
	if (!trimmedName || trimmedName.length > 120) {
		return { ok: false, error: messages.invalidRuleName };
	}
	if (input.selectedDestinations.length === 0) {
		return { ok: false, error: messages.noDestinations };
	}
	if (input.triggerKind === 'DueDate') {
		const days = Number(input.daysBefore);
		const hour = Number(input.targetHour);
		if (!Number.isInteger(days) || days < 0 || days > 365) {
			return { ok: false, error: messages.invalidDaysRange };
		}
		if (!Number.isInteger(hour) || hour < 0 || hour > 23) {
			return { ok: false, error: messages.invalidHourRange };
		}
	} else {
		const hours = Number(input.intervalHours);
		if (!Number.isInteger(hours) || hours < 1 || hours > 8760) {
			return { ok: false, error: messages.invalidIntervalRange };
		}
	}

	const minAmt = parseMinAmountCents(input.minAmountStr);
	const value: ScheduledReminderConfig = {
		...input.initial,
		name: trimmedName,
		enabled: input.enabled,
		account_id: input.selectedAccount === 'All' ? undefined : input.selectedAccount,
		destination_ids: [...input.selectedDestinations],
		trigger:
			input.triggerKind === 'DueDate'
				? {
						DueDate: {
							days_before: Number(input.daysBefore),
							target_hour: Number(input.targetHour),
							...(minAmt !== undefined ? { min_amount_cents: minAmt } : {})
						}
					}
				: {
						UnreadInterval: {
							interval_hours: Number(input.intervalHours),
							...(minAmt !== undefined ? { min_amount_cents: minAmt } : {})
						}
					}
	};
	return { ok: true, value };
}
