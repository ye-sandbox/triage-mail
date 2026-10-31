<script lang="ts">
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { nativeTransport } from '$lib/inbox/transport';
	import InboxView from '$lib/inbox/InboxView.svelte';
	let { connected, manageAccounts }: { connected: boolean; manageAccounts?: () => void } = $props();
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

<InboxView {inbox} {connected} {manageAccounts} />
