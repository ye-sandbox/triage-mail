<script lang="ts">
	import { untrack } from 'svelte';
	import type { InboxController } from './controller.svelte';
	import type { SearchMode } from './types';
	import Sidebar from './Sidebar.svelte';
	import SearchToolbar from './SearchToolbar.svelte';
	import MessageList from './MessageList.svelte';
	import SelectionOverview from './SelectionOverview.svelte';
	let { inbox, connected }: { inbox: InboxController; connected: boolean } = $props();
	let query = $state('');
	let mode = $state<SearchMode>('Lexical');
	let sidebarOpen = $state(true);
	const selected = $derived(inbox.rows.find((row) => row.id === inbox.selectedId));
	$effect(() => {
		const text = query.trim();
		const matching = mode;
		if (!connected) return;
		if (
			!text &&
			matching === 'Lexical' &&
			!untrack(() => inbox.query) &&
			untrack(() => inbox.mode) === 'Lexical'
		)
			return;
		const timer = setTimeout(() => void inbox.search(text, matching), 200);
		return () => clearTimeout(timer);
	});
</script>

<main class="flex h-dvh min-h-0 overflow-hidden text-[var(--text)]">
	{#if sidebarOpen}<div class="hidden w-56 shrink-0 md:block">
			<Sidebar status={inbox.syncStatus} />
		</div>{/if}
	<div class="flex min-w-0 flex-1 flex-col">
		<SearchToolbar
			bind:query
			bind:mode
			toggleSidebar={() => (sidebarOpen = !sidebarOpen)}
			refresh={() => void inbox.refresh()}
			disabled={!connected || inbox.loading}
		/>
		{#if !connected}<div
				class="border-b border-[var(--separator)] bg-[var(--surface)] px-6 py-3 text-sm text-[var(--secondary)]"
				role="status"
			>
				Open Triage Mail on your desktop to access your local mail.
			</div>{/if}
		<div class="flex min-h-0 flex-1">
			<div
				class="min-h-0 w-full flex-col md:flex md:w-88 lg:w-100"
				class:hidden={!!selected}
				class:flex={!selected}
			>
				<MessageList {inbox} />
			</div>
			<div class="min-w-0 flex-1 flex-col md:flex" class:hidden={!selected} class:flex={!!selected}>
				{#if selected}<button
						type="button"
						class="self-start px-6 py-3 text-sm text-[var(--accent)] md:hidden"
						onclick={() => (inbox.selectedId = null)}>‹ All Mail</button
					>{/if}
				<SelectionOverview message={selected} />
			</div>
		</div>
		<footer
			class="flex h-8 shrink-0 items-center justify-between gap-4 border-t border-[var(--separator)] bg-[var(--toolbar)] px-6 text-[11px] text-[var(--secondary)]"
		>
			<span class="truncate">{inbox.syncStatus}</span><span class="hidden shrink-0 sm:inline"
				>On-device search · Encrypted storage</span
			>
		</footer>
	</div>
</main>
