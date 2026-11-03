<script lang="ts">
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { nativeTransport } from '$lib/inbox/transport';
	import { nativeSettingsTransport } from '$lib/settings/transport';
	import type { SettingsTransport, SettingsTab } from '$lib/settings/types';
	import type { AccountController } from './controller.svelte';
	import InboxView from '$lib/inbox/InboxView.svelte';
	import SettingsDialog from '$lib/settings/SettingsDialog.svelte';
	import DueDateDashboard from '$lib/reminders/DueDateDashboard.svelte';
	import { nativeRemindersTransport } from '$lib/reminders/transport';
	import type { RemindersTransport } from '$lib/reminders/types';

	let {
		connected,
		settingsTransport = nativeSettingsTransport,
		remindersTransport = nativeRemindersTransport,
		accountController
	}: {
		connected: boolean;
		settingsTransport?: SettingsTransport;
		remindersTransport?: RemindersTransport;
		accountController?: AccountController;
	} = $props();

	const inbox = new InboxController(nativeTransport);
	let settingsOpen = $state(false);
	let dueDateDashboardOpen = $state(false);
	let photoRevision = $state(0);
	let settingsTab = $state<SettingsTab>('general');
	let ruleSeedMessageId = $state<string | undefined>();

	$effect(() => {
		if (!connected) {
			inbox.syncStatus = 'Desktop app required';
			return;
		}
		void inbox.connect();
		void settingsTransport
			.get()
			.then((settings) => {
				inbox.setExperimentalSemanticSearch(settings.experimental_semantic_search);
			})
			.catch(() => {});
		return () => inbox.dispose();
	});

	function handleCloseSettings() {
		settingsOpen = false;
		ruleSeedMessageId = undefined;
		photoRevision++;
		void settingsTransport
			.get()
			.then((settings) => {
				inbox.setExperimentalSemanticSearch(settings.experimental_semantic_search);
			})
			.catch(() => {});
	}
</script>

<InboxView
	{inbox}
	accounts={accountController?.accounts ?? []}
	{photoRevision}
	{connected}
	openSettings={() => {
		settingsTab = 'general';
		settingsOpen = true;
	}}
	openDueDates={() => {
		dueDateDashboardOpen = true;
	}}
	createRule={(id) => {
		ruleSeedMessageId = id;
		settingsTab = 'rules';
		settingsOpen = true;
	}}
	{settingsOpen}
/>
<DueDateDashboard
	open={dueDateDashboardOpen}
	close={() => (dueDateDashboardOpen = false)}
	transport={remindersTransport}
	accounts={accountController?.accounts ?? []}
	onSelectMessage={(msgId: string) => {
		dueDateDashboardOpen = false;
		inbox.selectedId = msgId;
	}}
	onConfigureReminders={() => {
		dueDateDashboardOpen = false;
		settingsTab = 'reminders';
		settingsOpen = true;
	}}
/>
<SettingsDialog
	open={settingsOpen}
	initialTab={settingsTab}
	{ruleSeedMessageId}
	selectedMessageIds={inbox.selectedId ? [inbox.selectedId] : []}
	close={handleCloseSettings}
	transport={settingsTransport}
	{accountController}
/>
