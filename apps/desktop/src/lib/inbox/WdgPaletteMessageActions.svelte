<script lang="ts">
	import { Command } from 'bits-ui';
	import Archive from '@lucide/svelte/icons/archive';
	import Bell from '@lucide/svelte/icons/bell';
	import Clock from '@lucide/svelte/icons/clock';
	import Mail from '@lucide/svelte/icons/mail';
	import MailMinus from '@lucide/svelte/icons/mail-minus';
	import MailOpen from '@lucide/svelte/icons/mail-open';
	import Star from '@lucide/svelte/icons/star';
	import Tag from '@lucide/svelte/icons/tag';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { t } from '$lib/i18n';
	import { i18n } from '$lib/i18n/index.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import type { TriageAction } from './triage';
	import type { MessageRowDto } from './types';

	type Props = {
		selected: MessageRowDto;
		canUnsubscribe: boolean;
		onAction: (action: TriageAction) => void;
		onOpenSnooze: () => void;
		onOpenTag: () => void;
	};

	let { selected, canUnsubscribe, onAction, onOpenSnooze, onOpenTag }: Props = $props();
</script>

<Command.Group>
	<Command.GroupHeading class="px-2 py-2 font-display text-[11px] font-semibold text-secondary">
		{i18n.t.mailboxes.actions} · {selected.subject || t(appStore.locale, 'noSubject')}
	</Command.GroupHeading>
	<Command.GroupItems>
		<Command.Item
			value={`mark read ${i18n.t.mailboxes.markRead}`}
			onSelect={() => onAction('MarkRead')}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<MailOpen class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{i18n.t.mailboxes.markRead}</span>
			</span>
			<kbd class="font-mono text-[10px] text-secondary">R</kbd>
		</Command.Item>

		<Command.Item
			value={`mark unread ${i18n.t.mailboxes.markUnread}`}
			onSelect={() => onAction('MarkUnread')}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<Mail class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{i18n.t.mailboxes.markUnread}</span>
			</span>
			<kbd class="font-mono text-[10px] text-secondary">Shift U</kbd>
		</Command.Item>

		<Command.Item
			value={`archive ${i18n.t.mailboxes.archive}`}
			onSelect={() => onAction('Archive')}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<Archive class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{i18n.t.mailboxes.archive}</span>
			</span>
			<kbd class="font-mono text-[10px] text-secondary">E</kbd>
		</Command.Item>

		<Command.Item
			value={`move to trash delete ${i18n.t.mailboxes.moveTrash}`}
			onSelect={() => onAction('Trash')}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<Trash2 class="h-3.5 w-3.5 text-status-danger" aria-hidden="true" />
				<span>{i18n.t.mailboxes.moveTrash}</span>
			</span>
			<kbd class="font-mono text-[10px] text-secondary">Delete</kbd>
		</Command.Item>

		<Command.Item
			value={`star toggle ${selected.is_starred ? i18n.t.mailboxes.unstar : i18n.t.mailboxes.star}`}
			onSelect={() => onAction(selected.is_starred ? 'Unstar' : 'Star')}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<Star class="h-3.5 w-3.5 text-status-warning" aria-hidden="true" />
				<span>{selected.is_starred ? i18n.t.mailboxes.unstar : i18n.t.mailboxes.star}</span>
			</span>
		</Command.Item>

		<Command.Item
			value={`snooze ${i18n.t.mailboxes.snooze}`}
			onSelect={onOpenSnooze}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<Clock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{i18n.t.mailboxes.snooze}…</span>
			</span>
		</Command.Item>

		{#if selected.snooze_until_epoch_ms}
			<Command.Item
				value={`unsnooze ${i18n.t.mailboxes.unsnooze}`}
				onSelect={() => onAction('Unsnooze')}
				class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
			>
				<span class="flex items-center gap-2">
					<Bell class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{i18n.t.mailboxes.unsnooze}</span>
				</span>
			</Command.Item>
		{/if}

		<Command.Item
			value={`add local tag ${i18n.t.mailboxes.tag}`}
			onSelect={onOpenTag}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<Tag class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{i18n.t.mailboxes.tag}…</span>
			</span>
			<kbd class="font-mono text-[10px] text-secondary">T</kbd>
		</Command.Item>

		<Command.Item
			value={`one-click unsubscribe ${i18n.t.mailboxes.unsubscribe}`}
			disabled={!canUnsubscribe}
			onSelect={() => onAction('OneClickUnsubscribe')}
			class="flex cursor-pointer items-center justify-between rounded-md px-3 py-2 text-foreground outline-none data-[disabled]:cursor-not-allowed data-[disabled]:opacity-40 data-[selected]:bg-selection-soft"
		>
			<span class="flex items-center gap-2">
				<MailMinus class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{i18n.t.mailboxes.unsubscribe} (RFC 8058)</span>
			</span>
			<span class="text-[10px] text-secondary">
				{canUnsubscribe ? 'Supported' : 'Unavailable'}
			</span>
		</Command.Item>
	</Command.GroupItems>
</Command.Group>
