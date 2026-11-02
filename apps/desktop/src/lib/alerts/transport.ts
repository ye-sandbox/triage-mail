import { invoke } from '@tauri-apps/api/core';
import type { AlertsTransport, Destination, History } from './types';
export const nativeAlertsTransport: AlertsTransport = {
	list: () => invoke<Destination[]>('cmd_list_alert_destinations'),
	save: (destination, secret, consent) =>
		invoke<Destination>('cmd_save_alert_destination', {
			destination,
			secret,
			consent
		}),
	remove: (id, revision) => invoke<void>('cmd_delete_alert_destination', { id, revision }),
	history: () => invoke<History[]>('cmd_alert_history'),
	test: (id, revision, consent) => invoke<void>('cmd_test_alert', { id, revision, consent }),
	replay: (ids, destination, revision, consent) =>
		invoke<void>('cmd_replay_alert', { ids, destination, revision, consent }),
	available: () => invoke<boolean>('cmd_local_alert_available')
};
