<script lang="ts">
	import Archive from '@lucide/svelte/icons/archive';
	import Clock from '@lucide/svelte/icons/clock';
	import Mail from '@lucide/svelte/icons/mail';
	import MailOpen from '@lucide/svelte/icons/mail-open';
	import Star from '@lucide/svelte/icons/star';
	import Tag from '@lucide/svelte/icons/tag';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Workflow from '@lucide/svelte/icons/workflow';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageRowDto } from './types';
	import type { InboxWorkspace } from './workspace.svelte';

	type Props = {
		message: MessageRowDto;
		workspace: InboxWorkspace;
		disabled: boolean;
	};

	let { message, workspace, disabled }: Props = $props();
</script>

<div class="flex flex-wrap items-center gap-2">
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
			aria-hidden="true"
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
			<Mail class="h-4 w-4" aria-hidden="true" />
		{:else}
			<MailOpen class="h-4 w-4" aria-hidden="true" />
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
		<Tag class="h-4 w-4" aria-hidden="true" />
	</button>

	<button
		type="button"
		{disabled}
		onclick={() => workspace.openSnoozeFor(message.id)}
		title={t(appStore.locale, 'snooze')}
		class="flex min-h-11 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50 md:min-h-8"
	>
		<Clock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
		<span>{t(appStore.locale, 'snooze')}</span>
	</button>

	<button
		type="button"
		{disabled}
		onclick={() => void workspace.mutate(message.id, 'Archive')}
		title={t(appStore.locale, 'archive')}
		class="flex min-h-11 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50 md:min-h-8"
	>
		<Archive class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
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
		<Trash2 class="h-4 w-4" aria-hidden="true" />
	</button>

	<button
		type="button"
		disabled={!workspace.connected}
		onclick={() => workspace.createRuleFor(message)}
		title={t(appStore.locale, 'createRule')}
		class="flex min-h-11 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50 md:min-h-8"
	>
		<Workflow class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
		<span class="hidden lg:inline">{t(appStore.locale, 'createRule')}</span>
	</button>
</div>
