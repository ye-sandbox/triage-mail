<script lang="ts">
	import { tick, untrack } from 'svelte';
	import type { InboxController } from './controller.svelte';
	import type { SearchMode } from './types';
	import type { ReaderTransport } from '../reader/types';
	import Sidebar from './Sidebar.svelte';
	import SearchToolbar from './SearchToolbar.svelte';
	import MessageList from './MessageList.svelte';
	import SelectionOverview from './SelectionOverview.svelte';
	import { BadgeLoader } from './badges';
	import { nativeReaderTransport } from '../reader/transport';
	import CommandPalette from './CommandPalette.svelte';
	import { shortcut, type TriageAction } from './triage';
	import type { TriageBadge } from '../reader/types';
	let {
		inbox,
		connected,
		readerTransport,
		manageAccounts,
		openSettings
	}: {
		inbox: InboxController;
		connected: boolean;
		readerTransport?: ReaderTransport;
		manageAccounts?: () => void;
		openSettings?: () => void;
	} = $props();
	let query = $state('');
	let mode = $state<SearchMode>('Lexical');
	let sidebarOpen = $state(true);
	let paletteOpen = $state(false);
	let tagging = $state(false);
	let actionTarget = $state<{ id: string; subject: string } | null>(null);
	let returnFocus: HTMLElement | null = null;
	let focusPending = $state(false);
	let selectedBadges = $state<TriageBadge[]>([]);
	const badgeLoader = $derived(
		connected ? new BadgeLoader(readerTransport ?? nativeReaderTransport) : undefined
	);
	const busy = $derived(
		inbox.loading ||
			['Connecting…', 'Syncing mail…', 'Saving changes…', 'Updating inbox…'].includes(
				inbox.syncStatus
			)
	);
	let shell: HTMLElement;
	async function backToList() {
		inbox.selectedId = null;
		await tick();
		shell.querySelector<HTMLElement>('[role=listbox]')?.focus();
	}
	const selected = $derived(inbox.rows.find((row) => row.id === inbox.selectedId));
	const canUnsubscribe = $derived(
		selectedBadges.some(
			(badge) => 'UnsubscribeAvailable' in badge && badge.UnsubscribeAvailable.supports_one_click
		)
	);
	$effect(() => {
		const id = selected?.id;
		const loader = badgeLoader;
		const revision = inbox.badgeRevision;
		return untrack(() => {
			selectedBadges = [];
			if (id && loader) return loader.watch(id, (badges) => (selectedBadges = badges), revision);
		});
	});
	function openPalette(tag = false) {
		if (!connected || !selected || inbox.mutating || inbox.loading) return;
		returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
		actionTarget = { id: selected.id, subject: selected.subject };
		tagging = tag;
		paletteOpen = true;
	}
	function restoreFocus() {
		focusPending = true;
	}
	$effect(() => {
		if (focusPending && !paletteOpen && !inbox.mutating)
			untrack(() => {
				focusPending = false;
				// INVARIANT: Restore focus after optimistic action controls are re-enabled.
				void tick().then(() => {
					if (returnFocus?.isConnected) returnFocus.focus();
					else shell.querySelector<HTMLElement>('[role=listbox]')?.focus();
				});
			});
	});
	function execute(action: TriageAction) {
		if (actionTarget) void inbox.mutate(actionTarget.id, action);
	}
	function keyboard(event: KeyboardEvent) {
		if (paletteOpen || !connected || !selected || inbox.mutating || inbox.loading) return;
		const action = shortcut(event);
		if (!action) return;
		event.preventDefault();
		if (action === 'palette' || action === 'tag') openPalette(action === 'tag');
		else void inbox.mutate(selected.id, action);
	}
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

<svelte:window onkeydown={keyboard} />
<CommandPalette
	bind:open={paletteOpen}
	bind:tagging
	subject={actionTarget?.subject ?? ''}
	canUnsubscribe={canUnsubscribe && actionTarget?.id === selected?.id}
	{execute}
	{restoreFocus}
/>
<main bind:this={shell} class="flex h-full min-h-0 flex-1 overflow-hidden text-[var(--text)]">
	<div
		class="inbox-sidebar hidden shrink-0 md:block"
		class:collapsed={!sidebarOpen}
		inert={!sidebarOpen}
		aria-hidden={!sidebarOpen}
	>
		<div class="h-full w-56"><Sidebar status={inbox.syncStatus} /></div>
	</div>
	<div class="flex min-w-0 flex-1 flex-col">
		<SearchToolbar
			bind:query
			bind:mode
			{sidebarOpen}
			{busy}
			toggleSidebar={() => (sidebarOpen = !sidebarOpen)}
			refresh={() => void inbox.refresh()}
			disabled={!connected || inbox.loading}
			experimentalSemanticSearch={inbox.experimentalSemanticSearch}
		/>
		<div
			class="flex shrink-0 items-center justify-between gap-4 border-b border-[var(--separator)] bg-[var(--toolbar)] px-6 py-2"
		>
			<button
				type="button"
				onclick={() => openPalette()}
				disabled={!connected || !selected || inbox.mutating || inbox.loading}
				aria-haspopup="dialog"
				aria-keyshortcuts="Meta+K Control+K"
				class="rounded px-2 py-1 text-sm text-[var(--accent)] disabled:opacity-40"
				>Message actions <kbd class="ml-2 text-xs">⌘K / Ctrl+K</kbd></button
			>
			<span class="text-xs text-[var(--secondary)]"
				>{inbox.mutating ? 'Saving…' : selected ? 'Selected message' : 'Select a message'}</span
			>
			{#if openSettings}<button
					type="button"
					class="rounded px-2 py-1 text-sm text-[var(--accent)]"
					onclick={openSettings}>Settings</button
				>{/if}
			{#if manageAccounts}<button
					type="button"
					class="rounded px-2 py-1 text-sm text-[var(--accent)]"
					onclick={manageAccounts}>Accounts</button
				>{/if}
		</div>
		{#if !connected}<div
				class="border-b border-[var(--separator)] bg-[var(--surface)] px-6 py-3 text-sm text-[var(--secondary)]"
				role="status"
			>
				Open Triage Mail on your desktop to access your local mail.
			</div>{/if}
		<div class="inbox-panes flex min-h-0 flex-1" data-detail={!!selected}>
			<div
				class="inbox-list min-h-0 w-full flex-col md:flex md:w-88 lg:w-100"
				class:hidden={!!selected}
				class:flex={!selected}
			>
				<MessageList {inbox} {badgeLoader} />
			</div>
			<div
				class="inbox-detail min-w-0 flex-1 flex-col md:flex"
				class:hidden={!selected}
				class:flex={!!selected}
			>
				{#if selected}<button
						type="button"
						class="self-start px-6 py-3 text-sm text-[var(--accent)] md:hidden"
						onclick={backToList}>‹ All Mail</button
					>{/if}
				{#if selected}
					<div
						class="shrink-0 border-b border-[var(--separator)] px-8 py-2 text-xs text-[var(--secondary)]"
						aria-label="Message state"
					>
						{selected.is_read ? 'Read' : 'Unread'} · {inbox.folderLabels[selected.id] ||
							selected.folder}
						{#each inbox.localTags[selected.id] ?? [] as tag}<span class="ml-2 break-all"
								>#{tag}</span
							>{/each}
					</div>
				{/if}
				<SelectionOverview message={selected} transport={readerTransport} />
			</div>
		</div>
		{#if inbox.mutationError}<p
				role="alert"
				class="border-t border-[var(--separator)] px-6 py-3 text-sm"
			>
				{inbox.mutationError}
			</p>{/if}
		{#if inbox.mutationNotice}<p
				role="status"
				class="border-t border-[var(--separator)] px-6 py-3 text-sm"
			>
				{inbox.mutationNotice}
			</p>{/if}
		<footer
			class="flex h-8 shrink-0 items-center justify-between gap-4 border-t border-[var(--separator)] bg-[var(--toolbar)] px-6 text-[11px] text-[var(--secondary)]"
		>
			<span class="truncate">{inbox.syncStatus}</span><span class="hidden shrink-0 sm:inline"
				>On-device search · Encrypted storage</span
			>
		</footer>
	</div>
</main>
