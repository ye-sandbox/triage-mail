import { nativeAvailable, nativeTransport } from '$lib/inbox/transport';
import type { InboxTransport } from '$lib/inbox/types';
import type { InboxWorkspace } from '$lib/inbox/workspace.svelte';
import { countOverdueAndDueToday, type RemindersTransport } from '$lib/reminders/types';
import type { SettingsTransport } from '$lib/settings/types';
import { appStore } from '$lib/stores/app.svelte';

export function refreshDueDateBadgeCount(
	remindersTransport: RemindersTransport,
	workspace: InboxWorkspace
) {
	if (!nativeAvailable()) return;
	void remindersTransport
		.listDueDates()
		.then((items) => {
			const activeCount = countOverdueAndDueToday(items);
			workspace.dueCount = activeCount;
			appStore.overdueAndDueTodayCount = activeCount;
		})
		.catch(() => {});
}

export function syncSettingsFromTransport(
	connected: boolean,
	settingsTransport: SettingsTransport,
	workspace: InboxWorkspace
) {
	if (!connected) return;
	void settingsTransport
		.get()
		.then((settings) => {
			workspace.inbox.setExperimentalSemanticSearch(settings.experimental_semantic_search);
		})
		.catch(() => {});
}

export function syncSavedViewsFromTransport(
	connected: boolean,
	inboxTransport: InboxTransport,
	workspace: InboxWorkspace
) {
	const inbox = workspace.inbox;
	if (
		!connected ||
		!inbox.savedTransport ||
		(inboxTransport === nativeTransport && !nativeAvailable())
	) {
		return;
	}
	void inbox.savedTransport
		.list()
		.then((views) => {
			appStore.savedViews = views.map((v) => ({
				id: v.id,
				name: v.name,
				query: v.query
			}));
		})
		.catch(() => {});
}
