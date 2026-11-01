<script lang="ts">
	import { untrack } from 'svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { nativeAccountTransport } from './transport';
	import type { AccountTransport } from './types';
	import { AccountController } from './controller.svelte';
	import OnboardingView from './OnboardingView.svelte';
	import InboxSession from './InboxSession.svelte';

	let {
		transport = nativeAccountTransport,
		available = nativeAvailable
	}: { transport?: AccountTransport; available?: () => boolean } = $props();

	const accounts = new AccountController(untrack(() => transport));
	let managing = $state(false);

	$effect(() => {
		void accounts.load(available());
		return () => accounts.dispose();
	});
</script>

{#if accounts.phase === 'browser'}
	<InboxSession connected={false} />
{:else if accounts.phase === 'ready' && accounts.accounts.length && !managing}
	<InboxSession
		connected={true}
		manageAccounts={() => (managing = true)}
		accountController={accounts}
	/>
{:else}
	<OnboardingView controller={accounts} enterInbox={() => (managing = false)} />
{/if}
