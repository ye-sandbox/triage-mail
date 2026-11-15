export type DueDateStatus = 'Overdue' | 'DueToday' | 'Upcoming' | 'Dismissed';

export interface DueDateItem {
	message_id: string;
	account_id: string;
	subject: string;
	sender: string;
	received_at_epoch_ms: number;
	is_read: boolean;
	digitable_line: string;
	amount_cents: number | null;
	due_date_iso: string;
	is_dismissed: boolean;
}

export type ReminderTrigger =
	| {
			DueDate: {
				days_before: number;
				target_hour: number;
				min_amount_cents?: number;
			};
	  }
	| {
			UnreadInterval: {
				interval_hours: number;
				min_amount_cents?: number;
			};
	  };

export interface ScheduledReminderConfig {
	version: number;
	id: string;
	revision: number;
	name: string;
	enabled: boolean;
	trigger: ReminderTrigger;
	destination_ids: string[];
	account_id?: string;
}

export interface RemindersTransport {
	listDueDates(accountId?: string): Promise<DueDateItem[]>;
	dismissDueDate(messageId: string): Promise<void>;
	listConfigs(): Promise<ScheduledReminderConfig[]>;
	saveConfig(config: ScheduledReminderConfig): Promise<ScheduledReminderConfig>;
	deleteConfig(id: string, revision: number): Promise<void>;
}

export function newReminderConfig(): ScheduledReminderConfig {
	return {
		version: 1,
		id: crypto.randomUUID(),
		revision: 0,
		name: '',
		enabled: true,
		trigger: {
			DueDate: {
				days_before: 2,
				target_hour: 9
			}
		},
		destination_ids: []
	};
}

export function calculateDueDateStatus(item: DueDateItem, todayIso?: string): DueDateStatus {
	if (item.is_dismissed) return 'Dismissed';
	const today = todayIso ?? new Date().toISOString().slice(0, 10);
	if (item.due_date_iso < today) return 'Overdue';
	if (item.due_date_iso === today) return 'DueToday';
	return 'Upcoming';
}

export function countOverdueAndDueToday(items: DueDateItem[], todayIso?: string): number {
	let count = 0;
	for (const item of items) {
		const status = calculateDueDateStatus(item, todayIso);
		if (status === 'Overdue' || status === 'DueToday') {
			count += 1;
		}
	}
	return count;
}
