import { invoke } from '@tauri-apps/api/core';
import type { DueDateItem, RemindersTransport, ScheduledReminderConfig } from './types';

export const nativeRemindersTransport: RemindersTransport = {
	listDueDates: (accountId) =>
		invoke<DueDateItem[]>('cmd_list_due_date_items', {
			accountId: accountId ?? null
		}),
	dismissDueDate: (messageId) => invoke<void>('cmd_dismiss_due_date_item', { messageId }),
	listConfigs: () => invoke<ScheduledReminderConfig[]>('cmd_list_reminder_configs'),
	saveConfig: (config) => invoke<ScheduledReminderConfig>('cmd_save_reminder_config', { config }),
	deleteConfig: (id, revision) => invoke<void>('cmd_delete_reminder_config', { id, revision })
};
