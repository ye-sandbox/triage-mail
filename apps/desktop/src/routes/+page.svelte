<script lang="ts">
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { nativeAvailable, nativeTransport } from '$lib/inbox/transport';
	import InboxView from '$lib/inbox/InboxView.svelte';
	const inbox = new InboxController(nativeTransport);
	let connected = $state(false);
	$effect(() => {
		if (!nativeAvailable()) {
			inbox.syncStatus = 'Desktop app required';
			return;
		}
		connected = true;
		void inbox.connect();
		return () => inbox.dispose();
	});
</script>

<svelte:head
	><title>All Mail — Triage Mail</title><meta
		name="description"
		content="Your private, local-first unified inbox."
	/></svelte:head
>
<InboxView {inbox} {connected} />
