<script lang="ts">
	import { goto } from '$app/navigation';
	import DueDateDashboard from '$lib/reminders/DueDateDashboard.svelte';
</script>

<svelte:head>
	<title>Due Dates · triage-mail</title>
</svelte:head>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
	<DueDateDashboard
		open={true}
		close={() => void goto('/')}
		onSelectMessage={() => void goto('/')}
		onConfigureReminders={() => void goto('/settings')}
	/>
</div>
