<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { TriageAction } from './triage';
	let { execute }: { execute: (action: TriageAction) => void } = $props();
	let deadline = $state('');
	let input = $state<HTMLInputElement>();
	$effect(() => {
		input?.focus();
	});
	let invalid = $state(false);
	const t = $derived(i18n.t.mailboxes);
	function submit(event: SubmitEvent) {
		event.preventDefault();
		const until = new Date(deadline).getTime();
		const now = Date.now();
		if (!Number.isFinite(until) || until <= now || until > now + 365 * 86400000) {
			invalid = true;
			return;
		}
		// INVARIANT: datetime-local is interpreted in the user's timezone; IPC always receives UTC milliseconds.
		execute({ Snooze: { until_epoch_ms: until } });
	}
</script>

<form onsubmit={submit}>
	<label class="block text-sm"
		>{t.deadline}<input
			type="datetime-local"
			bind:this={input}
			bind:value={deadline}
			required
			class="mt-2 block w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-4 py-2"
		/></label
	>
	<p class="mt-4 text-xs text-[var(--secondary)]">{t.snoozeHelp}</p>
	{#if invalid}<p role="alert" class="mt-4 text-sm">{t.invalidDeadline}</p>{/if}
	<button
		type="submit"
		class="mt-4 rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-sm text-[var(--accent)]"
		>{t.saveSnooze}</button
	>
</form>
