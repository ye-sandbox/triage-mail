<script lang="ts">
	import { goto } from '$app/navigation';
	import { nativeAvailable } from '$lib/inbox/transport';
	import OnboardingView from '$lib/onboarding/OnboardingView.svelte';
	import { AccountController } from '$lib/onboarding/controller.svelte';
	import { nativeAccountTransport } from '$lib/onboarding/transport';

	const controller = new AccountController(nativeAccountTransport);

	$effect(() => {
		void controller.load(nativeAvailable());
		return () => {
			controller.dispose();
		};
	});
</script>

<svelte:head>
	<title>Onboarding · triage-mail</title>
</svelte:head>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
	<OnboardingView {controller} enterInbox={() => void goto('/')} />
</div>
