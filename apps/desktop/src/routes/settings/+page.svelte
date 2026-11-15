<script lang="ts">
	import { goto } from '$app/navigation';
	import WdgSettingsPage from '$lib/settings/WdgSettingsPage.svelte';
</script>

<svelte:head>
	<title>Settings · triage-mail</title>
</svelte:head>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-background text-foreground">
	<WdgSettingsPage onBackToInbox={() => void goto('/')} />
</div>
