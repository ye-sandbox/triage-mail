<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { InboxController } from '../inbox/controller.svelte';
	import type { ConversationAction, ConversationPreview, ConversationRow } from './types';
	import { labels } from './labels';

	interface Props {
		inbox: InboxController;
		row: ConversationRow;
		disabled?: boolean;
		accountLabel: string;
	}

	let { inbox, row, disabled = false, accountLabel }: Props = $props();

	const t = $derived(labels[i18n.locale]);
	let kind = $state<'MarkRead' | 'MarkUnread' | 'Archive' | 'Tag'>('MarkRead');
	let tag = $state('');
	let all = $state(false);
	let preview = $state.raw<ConversationPreview | null>(null);
	let open = $state(false);
	let busy = $state(false);
	let error = $state(false);
	let notice = $state(false);
	let generation = 0;
	let alive = true;

	const action = $derived<ConversationAction>(kind === 'Tag' ? { Tag: tag.trim() } : kind);

	$effect(() => {
		inbox.conversationGeneration;
		row.conversation_id;
		inbox.revision;
		kind;
		tag;
		all;
		disabled;
		untrack(() => {
			generation++;
			preview = null;
			open = false;
			error = false;
		});
	});

	$effect(() => () => {
		alive = false;
		generation++;
		preview = null;
	});

	async function prepare() {
		const transport = inbox.conversationTransport;
		if (!transport || disabled || busy || inbox.mutating) return;
		const version = ++generation;
		busy = true;
		error = false;
		notice = false;
		try {
			const result = await transport.preview(
				inbox.conversationQuery(),
				inbox.conversationContext(),
				row.conversation_id,
				all,
				action
			);
			if (alive && version === generation) {
				preview = result;
				open = true;
			}
		} catch {
			if (alive && version === generation) error = true;
		} finally {
			if (alive) busy = false;
		}
	}

	async function apply() {
		const transport = inbox.conversationTransport;
		const snapshot = preview;
		if (!snapshot || !transport || disabled || busy || inbox.mutating) return;
		const version = ++generation;
		busy = true;
		inbox.mutating = true;
		inbox.mutationError = '';
		inbox.mutationNotice = '';
		error = false;
		try {
			await transport.apply(snapshot.token);
			if (alive && version === generation) {
				notice = true;
				inbox.mutationNotice = t.saved;
				preview = null;
				open = false;
			}
		} catch {
			if (alive && version === generation) {
				error = true;
				inbox.mutationError = t.unavailable;
				preview = null;
				open = false;
			}
		} finally {
			inbox.mutating = false;
			if (alive) busy = false;
			await inbox.refresh();
		}
	}
</script>

<div class="shrink-0 space-y-2 border-b border-separator pb-2.5 text-xs">
	<div class="flex flex-wrap items-center gap-2.5">
		<label class="flex items-center gap-1.5 text-secondary">
			<span>{t.action}</span>
			<select
				bind:value={kind}
				disabled={disabled || busy}
				class="h-8 rounded-md border border-separator bg-field px-2 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
			>
				<option value="MarkRead">{t.read}</option>
				<option value="MarkUnread">{t.unreadAction}</option>
				<option value="Archive">{t.archive}</option>
				<option value="Tag">{t.tag}</option>
			</select>
		</label>

		{#if kind === 'Tag'}
			<input
				aria-label={t.tagName}
				bind:value={tag}
				maxlength="128"
				disabled={disabled || busy}
				class="h-8 rounded-md border border-separator bg-field px-2.5 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
			/>
		{/if}

		<button
			type="button"
			disabled={disabled || busy || inbox.loading || (kind === 'Tag' && !tag.trim())}
			onclick={() => void prepare()}
			class="min-h-[32px] rounded-md border border-separator bg-field px-2.5 py-1 text-xs font-medium text-accent hover:bg-selection-soft disabled:opacity-50"
		>
			{t.preview}
		</button>
	</div>

	<label class="flex cursor-pointer items-center gap-2 text-secondary">
		<input type="checkbox" bind:checked={all} disabled={disabled || busy} class="accent-accent" />
		<span>{t.all}</span>
	</label>

	<p class="text-[11px] text-secondary">{t.remote}</p>
	{#if error}
		<p role="alert" class="text-status-danger">{t.unavailable} {t.scope}</p>
	{/if}
	{#if notice}
		<p role="status" class="text-status-healthy">{t.saved}</p>
	{/if}
</div>

<Dialog.Root
	{open}
	onOpenChange={(value) => {
		if (!busy) {
			open = value;
			if (!value) preview = null;
		}
	}}
>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			class="fixed left-1/2 top-1/2 z-50 flex max-h-[80vh] w-[calc(100%-2rem)] max-w-2xl -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-lg border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
			onEscapeKeydown={(event) => {
				if (busy) event.preventDefault();
			}}
			onInteractOutside={(event) => {
				if (busy) event.preventDefault();
			}}
		>
			<Dialog.Title class="font-display text-sm font-semibold text-foreground">{t.confirm}</Dialog.Title>
			<Dialog.Description class="my-3 text-xs text-secondary">{t.scope} {accountLabel}</Dialog.Description>
			{#if preview}
				<p class="text-xs font-medium tabular-nums text-foreground">
					{t.action}: {kind === 'MarkRead'
						? t.read
						: kind === 'MarkUnread'
							? t.unreadAction
							: kind === 'Archive'
								? t.archive
								: `${t.tag}: ${tag}`} · {t.matching}: {preview.messages.length -
						preview.outside_count} · {t.outside}: {preview.outside_count}
				</p>
				<ul class="my-3 min-h-0 flex-1 divide-y divide-separator overflow-auto rounded-md border border-separator bg-field px-3 text-xs">
					{#each preview.messages as message (message.id)}
						<li class="py-2">
							<p class="font-medium text-foreground">{message.subject}</p>
							<p class="text-[11px] text-secondary">
								{message.sender} · {accountLabel} · {message.folder_override ?? message.folder}
							</p>
						</li>
					{/each}
				</ul>
			{/if}
			<div class="flex justify-end gap-2 border-t border-separator pt-3">
				<button
					type="button"
					disabled={busy}
					onclick={() => {
						open = false;
						preview = null;
					}}
					class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground disabled:opacity-50"
				>
					{t.cancel}
				</button>
				<button
					type="button"
					disabled={busy || !preview || disabled}
					onclick={() => void apply()}
					class="min-h-[36px] rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
				>
					{t.apply}
				</button>
			</div>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
