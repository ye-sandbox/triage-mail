<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { Clock, Inbox, Paperclip, Star } from 'lucide-svelte';
	import { formatCategoryLabel, formatTimestamp, t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as conversationLabels } from '../conversations/labels';
	import { categories, type Category } from '../categories/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageCategory } from '$lib/types';
	import { ROW_HEIGHT, selectionTop, visibleRange } from './virtual';
	import WdgRowBadges from './WdgRowBadges.svelte';
	import { parseSender, workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace?: InboxWorkspace;
		listElement?: HTMLElement | null;
	};

	let { workspace = workspaceRegistry.active!, listElement = $bindable(null) }: Props = $props();

	let viewport = $state<HTMLDivElement | null>(null);
	let previousRows: typeof workspace.inbox.rows = [];
	let top = $state(0);
	let height = $state(600);

	$effect(() => {
		listElement = viewport;
	});

	const inbox = $derived(workspace.inbox);
	const range = $derived(visibleRange(inbox.rows.length, top, height));
	const visibleRows = $derived(inbox.rows.slice(range.start, range.end));

	$effect(() => {
		inbox.revision;
		untrack(() => {
			top = 0;
			if (viewport) viewport.scrollTop = 0;
		});
	});

	$effect(() => {
		// INVARIANT: Fill short viewports and prefetch the next page; never mount all loaded rows.
		if (range.end >= inbox.rows.length - 20 && !inbox.loading && inbox.hasMore && !inbox.error) {
			untrack(() => void inbox.loadMore());
		}
	});

	$effect(() => {
		const next = inbox.rows;
		untrack(() => {
			const index = Math.floor(top / ROW_HEIGHT);
			const anchor = previousRows[index]?.id;
			const moved = anchor ? next.findIndex((row) => row.id === anchor) : -1;
			previousRows = next;
			// INVARIANT: Keep the first visible message anchored when sync inserts rows above it.
			if (moved >= 0 && moved !== index) {
				top += (moved - index) * ROW_HEIGHT;
				const anchoredTop = top;
				// INVARIANT: Grow the spacer before restoring a position near the old bottom.
				void tick().then(() => {
					if (viewport && top === anchoredTop) viewport.scrollTop = anchoredTop;
				});
			}
		});
	});

	function handleListKeydown(event: KeyboardEvent) {
		if (!inbox.rows.length) return;
		if (event.key === 'Enter' && inbox.selectedId) {
			event.preventDefault();
			workspace.selectMessage(inbox.selectedId, true);
			return;
		}
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		event.preventDefault();
		event.stopPropagation();
		const current = inbox.rows.findIndex((row) => row.id === inbox.selectedId);
		const index =
			event.key === 'Home'
				? 0
				: event.key === 'End'
					? inbox.rows.length - 1
					: Math.max(
							0,
							Math.min(
								inbox.rows.length - 1,
								(current === -1 ? (event.key === 'ArrowDown' ? -1 : 1) : current) +
									(event.key === 'ArrowDown' ? 1 : -1)
							)
						);
		workspace.selectMessage(inbox.rows[index].id);
		if (viewport) {
			top = selectionTop(index, viewport.scrollTop, height);
			viewport.scrollTop = top;
		}
	}

	function rowSubLabel(row: (typeof inbox.rows)[number]): string {
		const override = inbox.folderLabels[row.id];
		if (override) return override;
		if (row.category_badge && categories.includes(row.category_badge as Category)) {
			return formatCategoryLabel(appStore.locale, row.category_badge as MessageCategory);
		}
		return row.category_badge || row.folder;
	}
</script>

<div class="relative flex h-full min-h-0 flex-col bg-surface">
	<div
		bind:this={viewport}
		bind:clientHeight={height}
		onscroll={() => {
			if (viewport) top = viewport.scrollTop;
		}}
		onkeydown={handleListKeydown}
		role="listbox"
		tabindex="0"
		aria-label="Messages"
		aria-busy={inbox.loading}
		aria-activedescendant={visibleRows.some((row) => row.id === inbox.selectedId)
			? `message-${inbox.selectedId}`
			: undefined}
		class="min-h-0 flex-1 overflow-y-auto overflow-x-hidden overscroll-contain [overflow-anchor:none] focus:outline-none"
	>
		<div style:height={`${range.total}px`} class="relative">
			<div class="absolute inset-x-0 top-0" style:transform={`translateY(${range.offset}px)`}>
				{#each visibleRows as row, index (row.id)}
					{@const selected = inbox.selectedId === row.id}
					{@const sender = parseSender(row.sender)}
					{@const conv = inbox.conversations ? inbox.conversationRows[row.id] : undefined}
					{@const tags = inbox.localTags[row.id] ?? row.tags}
					<div
						id={`message-${row.id}`}
						role="option"
						tabindex="-1"
						aria-selected={selected}
						aria-label={`${row.is_read ? '' : 'Unread, '}${row.sender}, ${row.subject || t(appStore.locale, 'noSubject')}`}
						aria-posinset={range.start + index + 1}
						aria-setsize={inbox.hasMore ? -1 : inbox.rows.length}
						style:height={`${ROW_HEIGHT}px`}
						onclick={() => {
							workspace.selectMessage(row.id, typeof window !== 'undefined' && window.innerWidth < 768);
							viewport?.focus();
						}}
						onkeydown={handleListKeydown}
						class="flex cursor-pointer flex-col justify-between border-b border-separator px-3 py-2 transition-colors {selected
							? 'bg-selection-soft'
							: 'hover:bg-field'}"
					>
						<div class="flex items-center justify-between gap-2">
							<div class="flex min-w-0 items-center gap-2">
								<span
									class="h-2 w-2 shrink-0 rounded-full {!row.is_read ? 'bg-accent' : 'bg-transparent'}"
									aria-label={!row.is_read ? 'Unread' : 'Read'}
								></span>
								<span
									class="truncate text-xs {!row.is_read
										? 'font-semibold text-foreground'
										: 'font-medium text-foreground'}"
								>
									{sender.name || t(appStore.locale, 'unknownSender')}
								</span>
								{#if conv}
									<span class="font-mono text-[10px] tabular-nums text-secondary">
										({conv.matching_count}/{conv.total_count} · {conversationLabels[i18n.locale].unread}: {conv.unread_count})
									</span>
								{/if}
							</div>

							<div class="flex shrink-0 items-center gap-1">
								{#if row.snooze_until_epoch_ms}
									<Clock class="h-3 w-3 text-status-warning" aria-label="Snoozed" />
								{/if}
								<button
									type="button"
									aria-label={row.is_starred
										? t(appStore.locale, 'unstar')
										: t(appStore.locale, 'star')}
									onclick={(e) => {
										e.stopPropagation();
										void workspace.mutate(row.id, row.is_starred ? 'Unstar' : 'Star');
									}}
									class="flex h-7 w-7 items-center justify-center rounded text-secondary hover:bg-surface hover:text-foreground"
								>
									<Star
										class="h-3.5 w-3.5 {row.is_starred
											? 'fill-status-warning text-status-warning'
											: 'text-secondary'}"
									/>
								</button>
								<time
									datetime={new Date(row.received_at_epoch_ms).toISOString()}
									class="font-mono text-[11px] tabular-nums text-secondary"
								>
									{formatTimestamp(new Date(row.received_at_epoch_ms).toISOString(), appStore.locale)}
								</time>
							</div>
						</div>

						<div class="flex items-center justify-between gap-2 pl-4">
							<p
								class="truncate text-xs {!row.is_read
									? 'font-medium text-foreground'
									: 'text-secondary'}"
							>
								{row.subject || t(appStore.locale, 'noSubject')}
							</p>
							<span class="flex shrink-0 items-center gap-1 text-[11px] text-secondary">
								{#if row.has_attachments}
									<Paperclip class="h-3 w-3" />
								{/if}
								<span>{rowSubLabel(row)}</span>
							</span>
						</div>

						<div class="flex items-center gap-1.5 overflow-hidden pl-4">
							{#each tags as tag (tag)}
								<span class="shrink-0 text-[11px] font-medium text-secondary">#{tag}</span>
							{/each}
							<WdgRowBadges
								id={row.id}
								inlineBadges={row.triage_badges}
								loader={workspace.badgeLoader}
								revision={inbox.badgeRevision}
								onLoaded={(id, badges) => workspace.recordRowBadges(id, badges)}
								onCopy={(val, lbl) => void workspace.copyText(val, lbl)}
							/>
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>

	{#if inbox.loading}
		<p class="border-t border-separator px-3 py-2 text-center text-xs text-secondary" role="status">
			{i18n.t.mailboxes.loading}
		</p>
	{/if}

	{#if inbox.error}
		<div class="border-t border-separator bg-field px-4 py-3 text-xs" role="alert">
			<p class="text-status-danger">{inbox.error}</p>
			<button
				type="button"
				onclick={() => void inbox.search(inbox.query, inbox.mode)}
				class="mt-1.5 font-medium text-accent underline"
			>
				{t(appStore.locale, 'tryAgain')}
			</button>
		</div>
	{/if}

	{#if !inbox.loading && !inbox.rows.length && !inbox.error}
		<div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-3 p-6 text-center">
			<div class="flex h-10 w-10 items-center justify-center rounded-lg border border-separator bg-field text-secondary">
				<Inbox class="h-5 w-5" />
			</div>
			<div class="max-w-xs space-y-1">
				<p class="font-display text-sm font-semibold text-foreground">
					{t(appStore.locale, 'emptyScopeTitle')}
				</p>
				<p class="text-xs text-secondary">{t(appStore.locale, 'emptyScopeBody')}</p>
			</div>
			<button
				type="button"
				onclick={() => workspace.clearFilters()}
				class="pointer-events-auto min-h-11 rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] md:min-h-0"
			>
				{t(appStore.locale, 'clearFilters')}
			</button>
		</div>
	{/if}
</div>
