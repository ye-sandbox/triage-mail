<script lang="ts">
	import { tick, untrack } from 'svelte';
	import Inbox from '@lucide/svelte/icons/inbox';
	import { t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import { selectionTop, visibleRange } from './virtual';
	import WdgMessageListRow from './WdgMessageListRow.svelte';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

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
	const rowHeight = $derived(appStore.rowHeight);
	const range = $derived(visibleRange(inbox.rows.length, top, height, rowHeight));
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
		const currentRowHeight = rowHeight;
		untrack(() => {
			const index = Math.floor(top / currentRowHeight);
			const anchor = previousRows[index]?.id;
			const moved = anchor ? next.findIndex((row) => row.id === anchor) : -1;
			previousRows = next;
			// INVARIANT: Keep the first visible message anchored when sync inserts rows above it.
			if (moved >= 0 && moved !== index) {
				top += (moved - index) * currentRowHeight;
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
			top = selectionTop(index, viewport.scrollTop, height, rowHeight);
			viewport.scrollTop = top;
		}
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
					<WdgMessageListRow
						{row}
						posinset={range.start + index + 1}
						setsize={inbox.hasMore ? -1 : inbox.rows.length}
						{workspace}
						onSelect={() => {
							workspace.selectMessage(row.id, typeof window !== 'undefined' && window.innerWidth < 768);
							viewport?.focus();
						}}
						onKeydown={handleListKeydown}
					/>
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
				class="mt-2 font-medium text-accent underline"
			>
				{t(appStore.locale, 'tryAgain')}
			</button>
		</div>
	{/if}

	{#if !inbox.loading && !inbox.rows.length && !inbox.error}
		<div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-3 p-6 text-center">
			<div class="flex h-10 w-10 items-center justify-center rounded-lg border border-separator bg-field text-secondary">
				<Inbox class="h-5 w-5" aria-hidden="true" />
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
