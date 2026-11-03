<script lang="ts">
	import { labels as categoryLabels } from '../categories/labels';
	import { categories, type Category } from '../categories/types';
	import { i18n } from '../i18n/index.svelte';
	import { tick, untrack } from 'svelte';
	import Icon from './Icon.svelte';
	import BadgePreview from './BadgePreview.svelte';
	import TriageBadges from './TriageBadges.svelte';
	import type { BadgeLoader } from './badges';
	import type { InboxController } from './controller.svelte';
	import { ROW_HEIGHT, visibleRange, selectionTop } from './virtual';
	let { inbox, badgeLoader }: { inbox: InboxController; badgeLoader?: BadgeLoader } = $props();
	let viewport: HTMLDivElement;
	let previousRows: typeof inbox.rows = [];
	let top = $state(0);
	let height = $state(600);
	const range = $derived(visibleRange(inbox.rows.length, top, height));
	const rows = $derived(inbox.rows.slice(range.start, range.end));
	const date = new Intl.DateTimeFormat(undefined, {
		month: 'short',
		day: 'numeric'
	});

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
	function navigate(event: KeyboardEvent) {
		const keys = ['ArrowDown', 'ArrowUp', 'Home', 'End'];
		if (!keys.includes(event.key) || !inbox.rows.length) return;
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
							Math.min(inbox.rows.length - 1, current + (event.key === 'ArrowDown' ? 1 : -1))
						);
		inbox.selectedId = inbox.rows[index].id;
		top = selectionTop(index, viewport.scrollTop, height);
		viewport.scrollTop = top;
	}
</script>

<div
	class="relative flex min-h-0 flex-1 flex-col border-r border-[var(--separator)] bg-[var(--surface)]"
>
	<div
		class="flex h-12 shrink-0 items-center justify-between border-b border-[var(--separator)] px-6 text-xs text-[var(--secondary)]"
	>
		<span>{inbox.query ? 'Search results' : 'All messages'}</span><span class="tabular-nums"
			>{inbox.rows.length.toLocaleString()}{inbox.hasMore ? '+' : ''} messages</span
		>
	</div>
	<div
		bind:this={viewport}
		bind:clientHeight={height}
		onscroll={() => (top = viewport.scrollTop)}
		onkeydown={navigate}
		role="listbox"
		aria-label="Messages"
		aria-busy={inbox.loading}
		aria-activedescendant={rows.some((row) => row.id === inbox.selectedId)
			? `message-${inbox.selectedId}`
			: undefined}
		tabindex="0"
		class="min-h-0 flex-1 overflow-auto overscroll-contain [overflow-anchor:none] outline-offset-[-3px]"
	>
		<div style:height={`${range.total}px`} class="relative">
			<div class="absolute inset-x-0 top-0" style:transform={`translateY(${range.offset}px)`}>
				{#each rows as row, index (row.id)}
					<div
						id={`message-${row.id}`}
						role="option"
						tabindex="-1"
						onkeydown={navigate}
						aria-selected={inbox.selectedId === row.id}
						aria-label={`${row.is_read ? '' : 'Unread, '}${row.sender}, ${row.subject || '(No subject)'}`}
						aria-posinset={range.start + index + 1}
						aria-setsize={inbox.hasMore ? -1 : inbox.rows.length}
						class="message-row group flex cursor-default gap-3 border-b border-[var(--separator)] px-5 py-3"
						class:selected={inbox.selectedId === row.id}
						style:height={`${ROW_HEIGHT}px`}
						onclick={() => {
							inbox.selectedId = row.id;
							viewport.focus();
						}}
					>
						<span
							class="mt-2 size-1.5 shrink-0 rounded-full"
							class:bg-[var(--accent)]={!row.is_read}
						></span>
						<div class="min-w-0 flex-1">
							<div class="flex items-baseline gap-2">
								<span class="truncate text-sm" class:font-semibold={!row.is_read}
									>{row.sender || 'Unknown sender'}</span
								><time
									class="ml-auto shrink-0 text-xs tabular-nums opacity-65"
									datetime={new Date(row.received_at_epoch_ms).toISOString()}
									>{date.format(row.received_at_epoch_ms)}</time
								>
							</div>
							<p class="truncate text-sm leading-6">
								{row.subject || '(No subject)'}
							</p>
							<div class="flex items-center gap-2 text-xs opacity-65">
								<span class="truncate"
									>{inbox.folderLabels[row.id] ||
										(row.category_badge && categories.includes(row.category_badge as Category)
											? categoryLabels[i18n.locale].category[row.category_badge as Category]
											: row.category_badge) ||
										row.folder}</span
								>{#if row.has_attachments}<Icon name="paperclip" size={12} />{/if}
								{#if row.triage_badges && row.triage_badges.length > 0}
									<TriageBadges badges={row.triage_badges} />
								{:else if badgeLoader}
									<BadgePreview id={row.id} loader={badgeLoader} revision={inbox.badgeRevision} />
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
	{#if inbox.loading}<p
			class="border-t border-[var(--separator)] p-3 text-center text-xs text-[var(--secondary)]"
			role="status"
		>
			{i18n.t.mailboxes.loading}
		</p>{/if}
	{#if inbox.error}<div class="border-t border-[var(--separator)] p-4 text-sm" role="alert">
			<p>{inbox.error}</p>
			<button
				type="button"
				class="mt-2 text-[var(--accent)]"
				onclick={() => void inbox.search(inbox.query, inbox.mode)}>Try again</button
			>
		</div>{/if}
	{#if !inbox.loading && !inbox.rows.length && !inbox.error}<div
			class="pointer-events-none absolute inset-x-0 top-1/2 -translate-y-1/2 px-8 py-12 text-center"
		>
			<p class="font-medium">
				{inbox.query ? i18n.t.mailboxes.empty : i18n.t.mailboxes.noCached}
			</p>
			<p class="mt-2 text-sm text-[var(--secondary)]">
				{i18n.t.mailboxes.emptyHelp}
			</p>
		</div>{/if}
</div>
