<script lang="ts">
	import Clock from '@lucide/svelte/icons/clock';
	import Paperclip from '@lucide/svelte/icons/paperclip';
	import Star from '@lucide/svelte/icons/star';
	import { formatTimestamp, t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as conversationLabels } from '../conversations/labels';
	import { categories, type Category } from '../categories/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageRowDto } from './types';
	import WdgRowBadges from './WdgRowBadges.svelte';
	import { parseSender, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		row: MessageRowDto;
		posinset: number;
		setsize: number;
		workspace: InboxWorkspace;
		onSelect: () => void;
		onKeydown: (event: KeyboardEvent) => void;
	};

	let { row, posinset, setsize, workspace, onSelect, onKeydown }: Props = $props();

	const inbox = $derived(workspace.inbox);
	const selected = $derived(inbox.selectedId === row.id);
	const sender = $derived(parseSender(row.sender));
	const conv = $derived(inbox.conversations ? inbox.conversationRows[row.id] : undefined);
	const tags = $derived(inbox.localTags[row.id] ?? row.tags);

	const subLabel = $derived.by(() => {
		const override = inbox.folderLabels[row.id];
		if (override) return override;
		if (row.category_badge && categories.includes(row.category_badge as Category)) {
			return appStore.formatCategoryDisplay(row.category_badge);
		}
		return row.category_badge || row.folder;
	});
</script>

<div
	id={`message-${row.id}`}
	role="option"
	tabindex="-1"
	aria-selected={selected}
	aria-label={`${row.is_read ? '' : 'Unread, '}${row.sender}, ${row.subject || t(appStore.locale, 'noSubject')}`}
	aria-posinset={posinset}
	aria-setsize={setsize}
	style:height={`${appStore.rowHeight}px`}
	onclick={onSelect}
	onkeydown={onKeydown}
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
					aria-hidden="true"
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
				<Paperclip class="h-3 w-3" aria-hidden="true" />
			{/if}
			<span>{subLabel}</span>
		</span>
	</div>

	<div class="flex items-center gap-2 overflow-hidden pl-4">
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
