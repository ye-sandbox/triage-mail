<script lang="ts">
	import { goto } from '$app/navigation';
	import SettingsView from '$lib/settings/SettingsView.svelte';
</script>

<svelte:head>
	<title>Settings · triage-mail</title>
</svelte:head>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
	<SettingsView close={() => void goto('/')} />
</div>
