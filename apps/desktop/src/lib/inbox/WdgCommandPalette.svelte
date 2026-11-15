<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Command, Dialog } from 'bits-ui';
	import {
		Archive,
		Bell,
		CalendarClock,
		Clock,
		Globe,
		Inbox,
		KeyRound,
		Mail,
		MailMinus,
		MailOpen,
		Moon,
		PanelLeft,
		RefreshCw,
		Search,
		Settings,
		Star,
		Sun,
		Tag,
		Trash2,
		Workflow
	} from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import { validTag, type TriageAction } from './triage';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace?: InboxWorkspace | null;
		restoreFocus?: () => void;
	};

	let { workspace, restoreFocus }: Props = $props();

	let searchQuery = $state('');
	let tagInput = $state('');
	let searchInputRef = $state<HTMLInputElement | null>(null);
	let tagInputRef = $state<HTMLInputElement | null>(null);

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

	$effect(() => {
		if (activeWorkspace?.tagDialogOpen) {
			tagInput = '';
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

	function submitTag(event: SubmitEvent) {
		event.preventDefault();
		const ws = activeWorkspace;
		const targetId = ws?.tagDialogTargetId;
		const trimmed = tagInput.trim();
		if (!ws || !targetId || !validTag(trimmed)) return;
		ws.tagDialogOpen = false;
		void ws.mutate(targetId, { Tag: trimmed });
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
					<Search class="h-4 w-4 shrink-0 text-secondary" />
					<Command.Input
						bind:ref={searchInputRef}
						bind:value={searchQuery}
						placeholder="Type a command or search…"
						aria-label={i18n.t.mailboxes.actionSearch}
						class="h-11 w-full bg-transparent text-xs text-foreground placeholder:text-secondary focus:outline-none"
					/>
					<kbd class="rounded border border-separator bg-field px-1.5 py-0.5 font-mono text-[10px] text-secondary">
						Esc
					</kbd>
				</div>

				<Command.List class="max-h-[380px] overflow-y-auto p-2 text-xs">
					<Command.Viewport>
						<Command.Empty class="px-3 py-6 text-center text-secondary">
							{i18n.t.mailboxes.noActions}
						</Command.Empty>

						{#if showMessageActions && selected}
						<Command.Group>
							<Command.GroupHeading class="px-2 py-1.5 font-display text-[11px] font-semibold text-secondary">
								{i18n.t.mailboxes.actions} · {selected.subject || t(appStore.locale, 'noSubject')}
							</Command.GroupHeading>
							<Command.GroupItems>
								<Command.Item
									value={`mark read ${i18n.t.mailboxes.markRead}`}
									onSelect={() => runAction('MarkRead')}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<MailOpen class="h-3.5 w-3.5 text-secondary" />
										<span>{i18n.t.mailboxes.markRead}</span>
									</span>
									<kbd class="font-mono text-[10px] text-secondary">R</kbd>
								</Command.Item>

								<Command.Item
									value={`mark unread ${i18n.t.mailboxes.markUnread}`}
									onSelect={() => runAction('MarkUnread')}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<Mail class="h-3.5 w-3.5 text-secondary" />
										<span>{i18n.t.mailboxes.markUnread}</span>
									</span>
									<kbd class="font-mono text-[10px] text-secondary">Shift U</kbd>
								</Command.Item>

								<Command.Item
									value={`archive ${i18n.t.mailboxes.archive}`}
									onSelect={() => runAction('Archive')}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<Archive class="h-3.5 w-3.5 text-secondary" />
										<span>{i18n.t.mailboxes.archive}</span>
									</span>
									<kbd class="font-mono text-[10px] text-secondary">E</kbd>
								</Command.Item>

								<Command.Item
									value={`move to trash delete ${i18n.t.mailboxes.moveTrash}`}
									onSelect={() => runAction('Trash')}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<Trash2 class="h-3.5 w-3.5 text-status-danger" />
										<span>{i18n.t.mailboxes.moveTrash}</span>
									</span>
									<kbd class="font-mono text-[10px] text-secondary">Delete</kbd>
								</Command.Item>

								<Command.Item
									value={`star toggle ${selected.is_starred ? i18n.t.mailboxes.unstar : i18n.t.mailboxes.star}`}
									onSelect={() => runAction(selected.is_starred ? 'Unstar' : 'Star')}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<Star class="h-3.5 w-3.5 text-status-warning" />
										<span>{selected.is_starred ? i18n.t.mailboxes.unstar : i18n.t.mailboxes.star}</span>
									</span>
								</Command.Item>

								<Command.Item
									value={`snooze ${i18n.t.mailboxes.snooze}`}
									onSelect={openSnoozeFromPalette}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<Clock class="h-3.5 w-3.5 text-secondary" />
										<span>{i18n.t.mailboxes.snooze}…</span>
									</span>
								</Command.Item>

								{#if selected.snooze_until_epoch_ms}
									<Command.Item
										value={`unsnooze ${i18n.t.mailboxes.unsnooze}`}
										onSelect={() => runAction('Unsnooze')}
										class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
									>
										<span class="flex items-center gap-2">
											<Bell class="h-3.5 w-3.5 text-secondary" />
											<span>{i18n.t.mailboxes.unsnooze}</span>
										</span>
									</Command.Item>
								{/if}

								<Command.Item
									value={`add local tag ${i18n.t.mailboxes.tag}`}
									onSelect={openTagFromPalette}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<Tag class="h-3.5 w-3.5 text-secondary" />
										<span>{i18n.t.mailboxes.tag}…</span>
									</span>
									<kbd class="font-mono text-[10px] text-secondary">T</kbd>
								</Command.Item>

								<Command.Item
									value={`one-click unsubscribe ${i18n.t.mailboxes.unsubscribe}`}
									disabled={!canUnsubscribe}
									onSelect={() => runAction('OneClickUnsubscribe')}
									class="flex cursor-pointer items-center justify-between rounded-md px-2.5 py-2 text-foreground outline-none data-[disabled]:cursor-not-allowed data-[disabled]:opacity-40 data-[selected]:bg-selection-soft"
								>
									<span class="flex items-center gap-2">
										<MailMinus class="h-3.5 w-3.5 text-secondary" />
										<span>{i18n.t.mailboxes.unsubscribe} (RFC 8058)</span>
									</span>
									<span class="text-[10px] text-secondary">
										{canUnsubscribe ? 'Supported' : 'Unavailable'}
									</span>
								</Command.Item>
							</Command.GroupItems>
						</Command.Group>

						<Command.Separator class="my-1.5 h-px bg-separator" />
					{/if}

					<Command.Group>
						<Command.GroupHeading class="px-2 py-1.5 font-display text-[11px] font-semibold text-secondary">
							Navigation & Preferences
						</Command.GroupHeading>
						<Command.GroupItems>
							<Command.Item
								value="go to unified inbox"
								onSelect={() => void navigateTo('/')}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<Inbox class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'navInbox')}</span>
							</Command.Item>

							<Command.Item
								value="go to due dates boletos reminders"
								onSelect={() => void navigateTo('/due-dates')}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<CalendarClock class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'navDueDates')}</span>
							</Command.Item>

							<Command.Item
								value="go to automation rules alerts sender governance"
								onSelect={() => void navigateTo('/automation')}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<Workflow class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'navAutomation')}</span>
							</Command.Item>

							<Command.Item
								value="go to onboarding accounts vault"
								onSelect={() => void navigateTo('/onboarding')}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<KeyRound class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'navOnboarding')}</span>
							</Command.Item>

							<Command.Item
								value="go to settings privacy ai backup"
								onSelect={() => void navigateTo('/settings')}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<Settings class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'navSettings')}</span>
							</Command.Item>

							<Command.Item
								value="toggle light dark theme appearance"
								onSelect={() => {
									appStore.toggleTheme();
									closePalette();
								}}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								{#if appStore.theme === 'dark'}
									<Sun class="h-3.5 w-3.5 text-secondary" />
									<span>Switch to Light Theme</span>
								{:else}
									<Moon class="h-3.5 w-3.5 text-secondary" />
									<span>Switch to Dark Theme</span>
								{/if}
							</Command.Item>

							<Command.Item
								value="switch language locale english portuguese"
								onSelect={() => {
									appStore.setLocale(appStore.locale === 'en' ? 'pt-BR' : 'en');
									closePalette();
								}}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<Globe class="h-3.5 w-3.5 text-secondary" />
								<span>
									{appStore.locale === 'en'
										? 'Switch Language to Português (BR)'
										: 'Switch Language to English'}
								</span>
							</Command.Item>

							<Command.Item
								value="toggle sidebar"
								onSelect={() => {
									appStore.toggleSidebar();
									closePalette();
								}}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<PanelLeft class="h-3.5 w-3.5 text-secondary" />
								<span>Toggle Sidebar</span>
							</Command.Item>

							<Command.Item
								value="sync mailboxes refresh"
								onSelect={() => {
									appStore.triggerSync();
									closePalette();
								}}
								class="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
							>
								<RefreshCw class="h-3.5 w-3.5 text-secondary" />
								<span>{t(appStore.locale, 'syncNow')}</span>
							</Command.Item>
						</Command.GroupItems>
					</Command.Group>
					</Command.Viewport>
				</Command.List>
			</Command.Root>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

{#if activeWorkspace}
	<Dialog.Root
		open={activeWorkspace.tagDialogOpen}
		onOpenChange={(next) => {
			if (activeWorkspace) {
				activeWorkspace.tagDialogOpen = next;
			}
			if (!next) tagInput = '';
		}}
	>
		<Dialog.Portal>
			<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
			<Dialog.Content
				onOpenAutoFocus={(event) => {
					event.preventDefault();
					tagInputRef?.focus();
				}}
				class="fixed top-1/2 left-1/2 z-50 w-[min(92vw,420px)] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
			>
				<Dialog.Title class="font-display text-sm font-semibold">
					{i18n.t.mailboxes.tag}
				</Dialog.Title>
				<Dialog.Description class="mt-1 text-xs text-secondary">
					Stored on this device. Up to 128 UTF-8 bytes, with no control characters.
				</Dialog.Description>
				<form onsubmit={submitTag} class="mt-4 space-y-4">
					<div>
						<label for="local-tag" class="block text-xs font-medium text-foreground">
							Tag name
						</label>
						<input
							id="local-tag"
							bind:this={tagInputRef}
							bind:value={tagInput}
							autocomplete="off"
							aria-describedby="tag-help"
							placeholder="e.g. Receipts, Taxes, Follow-up"
							class="mt-1.5 h-9 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
						/>
						<p id="tag-help" class="mt-1.5 text-[11px] text-secondary">
							Up to 128 UTF-8 bytes, no control characters.
						</p>
					</div>
					<div class="flex justify-end gap-2">
						<button
							type="button"
							onclick={() => {
								if (activeWorkspace) activeWorkspace.tagDialogOpen = false;
							}}
							class="min-h-9 rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground"
						>
							{i18n.t.mailboxes.close}
						</button>
						<button
							type="submit"
							disabled={!validTag(tagInput.trim())}
							class="min-h-9 rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground disabled:opacity-40"
						>
							Save tag
						</button>
					</div>
				</form>
			</Dialog.Content>
		</Dialog.Portal>
	</Dialog.Root>
{/if}
