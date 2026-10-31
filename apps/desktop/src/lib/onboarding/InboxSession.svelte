<script lang="ts">
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { nativeTransport } from '$lib/inbox/transport';
	import InboxView from '$lib/inbox/InboxView.svelte';
	let {
		connected,
		manageAccounts,
		openSettings
	}: {
		connected: boolean;
		manageAccounts?: () => void;
		openSettings?: () => void;
	} = $props();
	const inbox = new InboxController(nativeTransport);
	$effect(() => {
		if (!connected) {
			inbox.syncStatus = 'Desktop app required';
			return;
		}
		void inbox.connect();
		return () => inbox.dispose();
	});
</script>

<InboxView {inbox} {connected} {manageAccounts} {openSettings} />
