<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Command, Dialog } from 'bits-ui';
	import Search from '@lucide/svelte/icons/search';
	import { t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import type { TriageAction } from './triage';
	import WdgPaletteMessageActions from './WdgPaletteMessageActions.svelte';
	import WdgPaletteNavActions from './WdgPaletteNavActions.svelte';
	import WdgTagDialog from './WdgTagDialog.svelte';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace?: InboxWorkspace | null;
		restoreFocus?: () => void;
	};

	let { workspace, restoreFocus }: Props = $props();

	let searchQuery = $state('');
	let searchInputRef = $state<HTMLInputElement | null>(null);

	const activeWorkspace = $derived(workspace ?? workspaceRegistry.active);
	const selected = $derived(activeWorkspace?.selected);
	const canUnsubscribe = $derived(activeWorkspace?.canUnsubscribe ?? false);
	const showMessageActions = $derived(
		Boolean(selected && activeWorkspace) &&
			(workspace !== undefined || page.url.pathname === '/')
	);

	$effect(() => {
		if (appStore.commandOpen) {
			searchQuery = '';
		}
	});

	function closePalette() {
		appStore.commandOpen = false;
	}

	function runAction(action: TriageAction) {
		if (!activeWorkspace || !selected) return;
		if (action === 'OneClickUnsubscribe' && !canUnsubscribe) return;
		closePalette();
		void activeWorkspace.mutate(selected.id, action);
	}

	function openTagFromPalette() {
		if (!activeWorkspace || !selected) return;
		closePalette();
		activeWorkspace.openTagDialogFor(selected.id);
	}

	function openSnoozeFromPalette() {
		if (!activeWorkspace || !selected) return;
		closePalette();
		activeWorkspace.openSnoozeFor(selected.id);
	}

	async function navigateTo(path: string) {
		closePalette();
		try {
			await goto(path);
		} catch {
			// Unit tests mount WdgCommandPalette without booting SvelteKit's client router.
		}
	}

	function filterCommand(value: string, search: string, keywords?: string[]): number {
		const query = search.trim().toLowerCase();
		if (!query) return 1;
		const haystack = [value, ...(keywords ?? [])].join(' ').toLowerCase();
		if (haystack.includes(query)) return 1;
		const tokens = query.split(/\s+/).filter(Boolean);
		if (tokens.length > 1 && tokens.every((token) => haystack.includes(token))) return 0.7;
		return 0;
	}
</script>

<Dialog.Root bind:open={appStore.commandOpen}>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			onOpenAutoFocus={(event) => {
				event.preventDefault();
				searchInputRef?.focus();
			}}
			onCloseAutoFocus={(event) => {
				if (restoreFocus) {
					event.preventDefault();
					restoreFocus();
				}
			}}
			class="fixed top-[14%] left-1/2 z-50 w-[min(92vw,560px)] -translate-x-1/2 overflow-hidden rounded-xl border border-separator bg-surface text-foreground shadow-2xl outline-none"
		>
			<Dialog.Title class="sr-only">{i18n.t.mailboxes.actions}</Dialog.Title>
			<Dialog.Description class="sr-only">
				{selected?.subject || t(appStore.locale, 'commandPalette')}
			</Dialog.Description>

			<Command.Root
				label={t(appStore.locale, 'commandPalette')}
				filter={filterCommand}
				class="flex flex-col"
			>
				<div class="flex items-center gap-2 border-b border-separator px-3">
					<Search class="h-4 w-4 shrink-0 text-secondary" aria-hidden="true" />
					<Command.Input
						bind:ref={searchInputRef}
						bind:value={searchQuery}
						placeholder="Type a command or search…"
						aria-label={i18n.t.mailboxes.actionSearch}
						class="h-11 w-full bg-transparent text-xs text-foreground placeholder:text-secondary focus:outline-none"
					/>
					<kbd class="rounded border border-separator bg-field px-2 py-1 font-mono text-[10px] text-secondary">
						Esc
					</kbd>
				</div>

				<Command.List class="max-h-[380px] overflow-y-auto p-2 text-xs">
					<Command.Viewport>
						<Command.Empty class="px-3 py-6 text-center text-secondary">
							{i18n.t.mailboxes.noActions}
						</Command.Empty>

						{#if showMessageActions && selected}
							<WdgPaletteMessageActions
								{selected}
								{canUnsubscribe}
								onAction={runAction}
								onOpenSnooze={openSnoozeFromPalette}
								onOpenTag={openTagFromPalette}
							/>

							<Command.Separator class="my-2 h-px bg-separator" />
						{/if}

						<WdgPaletteNavActions onNavigate={navigateTo} onClose={closePalette} />
					</Command.Viewport>
				</Command.List>
			</Command.Root>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

{#if activeWorkspace}
	<WdgTagDialog workspace={activeWorkspace} />
{/if}
