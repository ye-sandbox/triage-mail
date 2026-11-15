<script lang="ts">
	import { untrack } from 'svelte';
	import { DropdownMenu } from 'bits-ui';
	import {
		Archive,
		ArrowLeft,
		ChevronDown,
		Clock,
		Mail,
		MailOpen,
		RotateCcw,
		Star,
		Tag,
		Trash2,
		UserCheck,
		UserX,
		Workflow
	} from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { formatCategoryLabel, formatTimestamp, t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { categories, type Category, type CategoryMetadata } from '../categories/types';
	import { labels as reviewLabels } from '../sender-review/labels';
	import type { ReviewState, SenderChoice } from '../sender-review/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageCategory } from '$lib/types';
	import type { MessageRowDto } from './types';
	import { parseSender, workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		message: MessageRowDto;
		workspace?: InboxWorkspace;
		fallbackCategory?: Category;
		onBackToList?: () => void;
	};

	let {
		message,
		workspace = workspaceRegistry.active!,
		fallbackCategory,
		onBackToList
	}: Props = $props();

	let categoryMeta = $state<CategoryMetadata | null>(null);
	let categoryBusy = $state(false);
	let reviewChoice = $state<SenderChoice | null>(null);
	let reviewBusy = $state(false);

	const inbox = $derived(workspace.inbox);
	const sender = $derived(parseSender(message.sender));
	const tags = $derived(inbox.localTags[message.id] ?? message.tags);
	const folderLabel = $derived(inbox.folderLabels[message.id] || message.folder);
	const effectiveCategory = $derived<MessageCategory>(
		(categoryMeta?.effective ??
			fallbackCategory ??
			(categories.includes(message.category_badge as Category)
				? (message.category_badge as Category)
				: 'Personal')) as MessageCategory
	);

	$effect(() => {
		const id = message.id;
		const rev = inbox.badgeRevision;
		const loadCategory =
			workspace.readerTransport.categoryMetadata ?? inbox.categoryTransport?.metadata;
		const loadReview = inbox.reviewTransport?.metadata;
		let active = true;
		untrack(() => {
			categoryMeta = null;
			reviewChoice = null;
			rev;
			if (loadCategory) {
				void loadCategory(id)
					.then((value) => {
						if (active) categoryMeta = value;
					})
					.catch(() => {});
			}
			if (loadReview) {
				void loadReview(id)
					.then((value) => {
						if (active) reviewChoice = value;
					})
					.catch(() => {});
			}
		});
		return () => {
			active = false;
		};
	});

	async function correctCategory(next: Category | null) {
		const correct =
			workspace.readerTransport.correctCategory ?? inbox.categoryTransport?.correct;
		if (!correct || categoryBusy) return;
		categoryBusy = true;
		try {
			const updated = await correct(
				message.id,
				categoryMeta?.correction_revision ?? 0,
				next
			);
			categoryMeta = updated;
			inbox.badgeRevision++;
			toast.success(
				next
					? `Category updated to ${formatCategoryLabel(appStore.locale, next as MessageCategory)}`
					: i18n.t.mailboxes.localSaved
			);
		} catch {
			toast.error(i18n.t.mailboxes.saveError);
		} finally {
			categoryBusy = false;
		}
	}

	async function decideSender(state: ReviewState) {
		const save = inbox.reviewTransport?.save;
		if (!save || !reviewChoice || reviewBusy) return;
		reviewBusy = true;
		try {
			const updated = await save({ ...reviewChoice, state });
			reviewChoice = updated;
			await inbox.refresh();
		} catch {
			toast.error(i18n.t.mailboxes.saveError);
		} finally {
			reviewBusy = false;
		}
	}

	const disabled = $derived(!workspace.connected || inbox.mutating || inbox.loading);
</script>

<div class="flex shrink-0 flex-col gap-3 border-b border-separator bg-surface px-4 py-3">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div class="flex min-w-0 flex-wrap items-center gap-2">
			<button
				type="button"
				onclick={() => onBackToList?.()}
				class="flex min-h-11 items-center gap-1.5 rounded-md border border-separator bg-field px-2.5 py-1 text-xs font-medium text-foreground md:hidden"
			>
				<ArrowLeft class="h-4 w-4" />
				<span>{t(appStore.locale, 'backToList')}</span>
			</button>

			<DropdownMenu.Root>
				<DropdownMenu.Trigger
					disabled={disabled || categoryBusy}
					aria-label="Message category"
					class="flex min-h-11 shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border border-separator bg-field px-2.5 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] disabled:opacity-50 md:min-h-8"
				>
					<span>{formatCategoryLabel(appStore.locale, effectiveCategory)}</span>
					<ChevronDown class="h-3 w-3 text-secondary" />
				</DropdownMenu.Trigger>
				<DropdownMenu.Portal>
					<DropdownMenu.Content
						class="z-50 min-w-[170px] rounded-md border border-separator bg-surface p-1 shadow-md"
						sideOffset={4}
					>
						{#each categories as cat (cat)}
							<DropdownMenu.Item
								onSelect={() => void correctCategory(cat)}
								class="cursor-pointer rounded px-2.5 py-1.5 text-xs text-foreground outline-none data-[highlighted]:bg-selection-soft"
							>
								{formatCategoryLabel(appStore.locale, cat as MessageCategory)}
							</DropdownMenu.Item>
						{/each}
						{#if categoryMeta && categoryMeta.correction_revision > 0}
							<DropdownMenu.Separator class="my-1 h-px bg-separator" />
							<DropdownMenu.Item
								onSelect={() => void correctCategory(null)}
								class="flex cursor-pointer items-center gap-1.5 rounded px-2.5 py-1.5 text-xs text-secondary outline-none data-[highlighted]:bg-selection-soft data-[highlighted]:text-foreground"
							>
								<RotateCcw class="h-3 w-3" />
								<span>{t(appStore.locale, 'resetCategory')}</span>
							</DropdownMenu.Item>
						{/if}
					</DropdownMenu.Content>
				</DropdownMenu.Portal>
			</DropdownMenu.Root>

			<span class="rounded bg-field px-2 py-0.5 text-[11px] text-secondary">{folderLabel}</span>

			{#if message.snooze_until_epoch_ms}
				<span class="rounded bg-field px-2 py-0.5 font-mono text-[11px] text-status-warning">
					{i18n.t.mailboxes.snoozedUntil} {formatTimestamp(new Date(message.snooze_until_epoch_ms).toISOString(), appStore.locale)}
				</span>
			{/if}

			{#each tags as tag (tag)}
				<span class="text-xs font-medium text-secondary">#{tag}</span>
			{/each}
		</div>

		<div class="flex flex-wrap items-center gap-1.5">
			<button
				type="button"
				{disabled}
				onclick={() => void workspace.mutate(message.id, message.is_starred ? 'Unstar' : 'Star')}
				title={message.is_starred ? t(appStore.locale, 'unstar') : t(appStore.locale, 'star')}
				aria-label={message.is_starred ? t(appStore.locale, 'unstar') : t(appStore.locale, 'star')}
				class="flex min-h-11 min-w-11 items-center justify-center rounded-md border border-separator bg-surface text-secondary transition-transform duration-100 ease-out hover:bg-field hover:text-foreground active:scale-[0.98] disabled:opacity-50 md:min-h-8 md:min-w-8"
			>
				<Star
					class="h-4 w-4 {message.is_starred ? 'fill-status-warning text-status-warning' : ''}"
				/>
			</button>

			<button
				type="button"
				{disabled}
				onclick={() => void workspace.mutate(message.id, message.is_read ? 'MarkUnread' : 'MarkRead')}
				title={message.is_read ? t(appStore.locale, 'markUnread') : t(appStore.locale, 'markRead')}
				aria-label={message.is_read ? t(appStore.locale, 'markUnread') : t(appStore.locale, 'markRead')}
				class="flex min-h-11 min-w-11 items-center justify-center rounded-md border border-separator bg-surface text-secondary transition-transform duration-100 ease-out hover:bg-field hover:text-foreground active:scale-[0.98] disabled:opacity-50 md:min-h-8 md:min-w-8"
			>
				{#if message.is_read}
					<Mail class="h-4 w-4" />
				{:else}
					<MailOpen class="h-4 w-4" />
				{/if}
			</button>

			<button
				type="button"
				{disabled}
				onclick={() => workspace.openTagDialogFor(message.id)}
				title={t(appStore.locale, 'addTag')}
				aria-label={t(appStore.locale, 'addTag')}
				class="flex min-h-11 min-w-11 items-center justify-center rounded-md border border-separator bg-surface text-secondary transition-transform duration-100 ease-out hover:bg-field hover:text-foreground active:scale-[0.98] disabled:opacity-50 md:min-h-8 md:min-w-8"
			>
				<Tag class="h-4 w-4" />
			</button>

			<button
				type="button"
				{disabled}
				onclick={() => workspace.openSnoozeFor(message.id)}
				title={t(appStore.locale, 'snooze')}
				class="flex min-h-11 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50 md:min-h-8"
			>
				<Clock class="h-3.5 w-3.5 text-secondary" />
				<span>{t(appStore.locale, 'snooze')}</span>
			</button>

			<button
				type="button"
				{disabled}
				onclick={() => void workspace.mutate(message.id, 'Archive')}
				title={t(appStore.locale, 'archive')}
				class="flex min-h-11 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50 md:min-h-8"
			>
				<Archive class="h-3.5 w-3.5 text-secondary" />
				<span class="hidden sm:inline">{t(appStore.locale, 'archive')}</span>
			</button>

			<button
				type="button"
				{disabled}
				onclick={() => void workspace.mutate(message.id, 'Trash')}
				title={t(appStore.locale, 'trash')}
				aria-label={t(appStore.locale, 'trash')}
				class="flex min-h-11 min-w-11 items-center justify-center rounded-md border border-separator bg-surface text-secondary transition-transform duration-100 ease-out hover:bg-field hover:text-status-danger active:scale-[0.98] disabled:opacity-50 md:min-h-8 md:min-w-8"
			>
				<Trash2 class="h-4 w-4" />
			</button>

			<button
				type="button"
				disabled={!workspace.connected}
				onclick={() => workspace.createRuleFor(message)}
				title={t(appStore.locale, 'createRule')}
				class="flex min-h-11 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50 md:min-h-8"
			>
				<Workflow class="h-3.5 w-3.5 text-secondary" />
				<span class="hidden lg:inline">{t(appStore.locale, 'createRule')}</span>
			</button>
		</div>
	</div>

	<div class="space-y-1">
		<h1 class="font-display text-base font-semibold text-balance text-foreground">
			{message.subject || t(appStore.locale, 'noSubject')}
		</h1>
		<div class="flex flex-wrap items-center justify-between gap-2 text-xs text-secondary">
			<span>
				<strong class="font-medium text-foreground">{sender.name || t(appStore.locale, 'unknownSender')}</strong>
				{#if sender.email && sender.email !== sender.name}
					<span class="ml-1 font-mono text-[11px]">&lt;{sender.email}&gt;</span>
				{/if}
			</span>
			<time
				datetime={new Date(message.received_at_epoch_ms).toISOString()}
				class="font-mono text-[11px] tabular-nums"
			>
				{formatTimestamp(new Date(message.received_at_epoch_ms).toISOString(), appStore.locale)}
			</time>
		</div>
	</div>

	{#if reviewChoice && (reviewChoice.state === 'Pending' || inbox.review)}
		<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2 text-xs">
			<span class="text-secondary">
				{reviewLabels[i18n.locale].title}: <strong class="text-foreground">{reviewLabels[i18n.locale][reviewChoice.state]}</strong>
			</span>
			<div class="flex items-center gap-2">
				<button
					type="button"
					disabled={reviewBusy || reviewChoice.state === 'Accepted'}
					onclick={() => void decideSender('Accepted')}
					class="flex items-center gap-1 rounded border border-separator bg-surface px-2.5 py-1 font-medium text-status-healthy hover:bg-selection-soft disabled:opacity-50"
				>
					<UserCheck class="h-3.5 w-3.5" />
					<span>{reviewLabels[i18n.locale].accept}</span>
				</button>
				<button
					type="button"
					disabled={reviewBusy || reviewChoice.state === 'Declined'}
					onclick={() => void decideSender('Declined')}
					class="flex items-center gap-1 rounded border border-separator bg-surface px-2.5 py-1 font-medium text-status-danger hover:bg-selection-soft disabled:opacity-50"
				>
					<UserX class="h-3.5 w-3.5" />
					<span>{reviewLabels[i18n.locale].decline}</span>
				</button>
			</div>
		</div>
	{/if}
</div>
