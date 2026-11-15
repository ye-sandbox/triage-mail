import type { AlertsTransport, Destination } from '../alerts/types';
import { appStore } from '../stores/app.svelte';
import type { EmailMessage } from '../types';
import {
	buildInboxMessageFromDueItem,
	createDueDateFixtures,
	DEMO_ALERT_DESTINATIONS,
	DEMO_REMINDER_CONFIGS
} from './due-dates.seed';
import {
	countOverdueAndDueToday,
	type DueDateItem,
	type RemindersTransport,
	type ScheduledReminderConfig
} from './types';

export {
	buildInboxMessageFromDueItem,
	createDueDateFixtures,
	DEMO_ALERT_DESTINATIONS,
	DEMO_REMINDER_CONFIGS
};

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
