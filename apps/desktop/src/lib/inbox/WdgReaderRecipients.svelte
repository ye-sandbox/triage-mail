<script lang="ts">
	import Inbox from '@lucide/svelte/icons/inbox';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageRowDto } from './types';
	import type { InboxWorkspace } from './workspace.svelte';

	type Props = {
		message: MessageRowDto;
		workspace: InboxWorkspace;
		recipients?: string;
		onBackToList?: () => void;
	};

	let { message, workspace, recipients = '', onBackToList }: Props = $props();

	const accountAddress = $derived(
		workspace.accounts.find((a) => a.account_id === message.account_id)?.email_display ??
			appStore.accounts.find((a) => a.account_id === message.account_id)?.address ??
			''
	);

	const trimmedRecipients = $derived(recipients.trim());
	const displayRecipients = $derived(trimmedRecipients || accountAddress);
	const showSeparateAccount = $derived(
		Boolean(
			trimmedRecipients &&
				accountAddress &&
				!trimmedRecipients.toLowerCase().includes(accountAddress.toLowerCase())
		)
	);
	const inboxActionLabel = $derived(
		accountAddress
			? `${t(appStore.locale, 'openAccountInbox')}: ${accountAddress}`
			: t(appStore.locale, 'openAccountInbox')
	);

	function openAccountInbox() {
		appStore.setScope({
			kind: 'folder',
			account_id: message.account_id,
			folder: 'INBOX'
		});
		onBackToList?.();
	}
</script>

{#if displayRecipients}
	<div class="flex flex-wrap items-center gap-1 text-xs text-secondary">
		<span class="shrink-0">{t(appStore.locale, 'toRecipient')}</span>
		<button
			type="button"
			onclick={openAccountInbox}
			title={inboxActionLabel}
			aria-label={inboxActionLabel}
			class="inline-flex min-w-0 max-w-full items-center gap-1 rounded px-1 py-0 text-left font-mono text-[11px] text-secondary transition-colors hover:bg-field hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
		>
			<Inbox class="h-3 w-3 shrink-0 text-accent" aria-hidden="true" />
			<span class="truncate">{displayRecipients}</span>
			{#if showSeparateAccount}
				<span class="shrink-0 text-secondary">({accountAddress})</span>
			{/if}
		</button>
	</div>
{/if}
