<script lang="ts">
	import CategoryViews from '../categories/CategoryViews.svelte';
	import CategoryActions from '../categories/CategoryActions.svelte';
	import SavedViews from '../saved-views/SavedViews.svelte';
	import { tick, untrack } from 'svelte';
	import type { InboxController } from './controller.svelte';
	import { i18n } from '../i18n/index.svelte';
	import MailboxLifecycle from './MailboxLifecycle.svelte';
	import { Mailboxes, scopeLabel } from './mailboxes.svelte';
	import type { AccountConfig } from '../onboarding/types';
	import type { AvatarTransport } from '../onboarding/avatar-types';
	import MessageActions from './MessageActions.svelte';
	import MessageState from './MessageState.svelte';
	import type { SearchMode, MailboxScope } from './types';
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
		openSettings,
		openDueDates,
		dueCount = 0,
		createRule,
		settingsOpen = false,
		accounts = [],
		avatarTransport,
		photoRevision = 0
	}: {
		inbox: InboxController;
		connected: boolean;
		readerTransport?: ReaderTransport;
		openSettings?: () => void;
		openDueDates?: () => void;
		dueCount?: number;
		createRule?: (id: string) => void;
		settingsOpen?: boolean;
		accounts?: AccountConfig[];
		avatarTransport?: AvatarTransport;
		photoRevision?: number;
	} = $props();
	const mailboxes = new Mailboxes(untrack(() => inbox.listFolders));
	const context = $derived(
		inbox.savedView?.name ?? scopeLabel(inbox.scope, accounts, i18n.t.mailboxes, mailboxes.catalogs)
	);
	let snoozing = $state(false);
	function selectScope(scope?: MailboxScope) {
		paletteOpen = false;
		if (window.innerWidth < 768) sidebarOpen = false;
		void inbox.selectScope(scope);
	}
	let query = $state('');
	let mode = $state<SearchMode>('Lexical');
	$effect(() => {
		const view = inbox.savedView;
		if (view)
			untrack(() => {
				query = view.query;
				mode = view.mode;
			});
	});
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
	function openPalette(tag = false, snooze = false) {
		if (settingsOpen || !connected || !selected || inbox.mutating || inbox.loading) return;
		returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
		actionTarget = { id: selected.id, subject: selected.subject };
		tagging = tag;
		snoozing = snooze;
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
		if (settingsOpen || paletteOpen || !connected || !selected || inbox.mutating || inbox.loading)
			return;
		const action = shortcut(event);
		if (!action) return;
		event.preventDefault();
		if (action === 'palette' || action === 'tag') openPalette(action === 'tag');
		else void inbox.mutate(selected.id, action);
	}
	$effect(() => {
		const text = query.trim();
		const matching = mode;
		if (!connected || inbox.savedView) return;
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
<MailboxLifecycle {inbox} {mailboxes} {accounts} {connected} />
<CommandPalette
	bind:open={paletteOpen}
	bind:tagging
	bind:snoozing
	subject={actionTarget?.subject ?? ''}
	canUnsubscribe={canUnsubscribe && actionTarget?.id === selected?.id}
	{execute}
	{restoreFocus}
/>
<main
	bind:this={shell}
	class="relative flex h-full min-h-0 flex-1 overflow-hidden text-[var(--text)]"
>
	<div
		class="inbox-sidebar absolute inset-y-0 left-0 z-20 shrink-0 md:static"
		class:collapsed={!sidebarOpen}
		inert={!sidebarOpen}
		aria-hidden={!sidebarOpen}
	>
		<div class="h-full w-56">
			<Sidebar
				savedActive={!!inbox.savedView || !!inbox.category}
				status={inbox.syncStatus}
				{openSettings}
				{openDueDates}
				{dueCount}
				{accounts}
				{mailboxes}
				scope={inbox.scope}
				select={selectScope}
				{avatarTransport}
				{photoRevision}
				dismiss={() => (sidebarOpen = false)}
				><CategoryViews {inbox} {connected} /><SavedViews {inbox} {accounts} {connected} /></Sidebar
			>
		</div>
	</div>
	<div class="flex min-w-0 flex-1 flex-col">
		<SearchToolbar
			savedActive={!!inbox.savedView}
			bind:query
			{context}
			bind:mode
			{sidebarOpen}
			{busy}
			toggleSidebar={() => (sidebarOpen = !sidebarOpen)}
			refresh={() => void inbox.refresh()}
			{openSettings}
			disabled={!connected || inbox.loading}
			experimentalSemanticSearch={inbox.experimentalSemanticSearch}
		/>
		<MessageActions
			{createRule}
			{selected}
			disabled={!connected || inbox.mutating || inbox.loading}
			{openPalette}
			mutate={(action) => {
				if (selected) void inbox.mutate(selected.id, action);
			}}
		/>
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
						onclick={backToList}>‹ {context}</button
					>{/if}
				{#if selected}<MessageState
						message={selected}
						folder={inbox.folderLabels[selected.id] || selected.folder}
						tags={inbox.localTags[selected.id] ?? []}
					/>{/if}
				{#if selected}{#key selected.id}<CategoryActions
							{inbox}
							message={selected}
							disabled={!connected || settingsOpen}
						/>{/key}{/if}
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
