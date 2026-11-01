<script lang="ts">
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { nativeTransport } from '$lib/inbox/transport';
	import { nativeSettingsTransport } from '$lib/settings/transport';
	import type { SettingsTransport } from '$lib/settings/types';
	import InboxView from '$lib/inbox/InboxView.svelte';
	import SettingsDialog from '$lib/settings/SettingsDialog.svelte';

	let {
		connected,
		manageAccounts,
		settingsTransport = nativeSettingsTransport
	}: {
		connected: boolean;
		manageAccounts?: () => void;
		settingsTransport?: SettingsTransport;
	} = $props();

	const inbox = new InboxController(nativeTransport);
	let settingsOpen = $state(false);

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
	{connected}
	{manageAccounts}
	openSettings={() => (settingsOpen = true)}
	{settingsOpen}
/>
<SettingsDialog
	open={settingsOpen}
	close={handleCloseSettings}
	transport={settingsTransport}
/>
