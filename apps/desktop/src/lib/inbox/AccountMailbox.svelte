<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { AccountConfig } from '../onboarding/types';
	import type { AvatarTransport } from '../onboarding/avatar-types';
	import type { MailboxScope, MailboxView, FolderDto } from './types';
	import { folderGroups, sameView, type Catalog } from './mailboxes.svelte';
	import AccountThumbnail from './AccountThumbnail.svelte';
	let {
		account,
		expanded,
		toggle,
		catalog,
		scope,
		select,
		retry,
		avatarTransport,
		photoRevision = 0
	}: {
		account: AccountConfig;
		expanded: boolean;
		toggle: () => void;
		catalog?: Catalog;
		scope?: MailboxScope;
		select: (scope?: MailboxScope) => void;
		retry: () => void;
		avatarTransport?: AvatarTransport;
		photoRevision?: number;
	} = $props();
	let otherOpen = $state(false);
	const t = $derived(i18n.t.mailboxes);
	const groups = $derived(folderGroups(catalog?.folders ?? []));
	function choose(view: MailboxView) {
		select({ AccountView: { account_id: account.account_id, view } });
	}
	function name(folder: FolderDto) {
		if (folder.role === 'spam' || folder.role === 'junk') return t.spam;
		if (folder.role === 'archive') return t.archive;
		if (folder.role === 'trash') return t.trash;
		return folder.display_name;
	}
</script>

<div class="min-w-0">
	<button
		type="button"
		class="flex w-full min-w-0 items-center gap-2 rounded-lg px-2 py-2 text-left text-sm hover:bg-[var(--surface)]"
		aria-expanded={expanded}
		onclick={toggle}
		title={account.email_display}
	>
		<AccountThumbnail
			accountId={account.account_id}
			label={account.email_display}
			transport={avatarTransport}
			revision={photoRevision}
		/>
		<span class="min-w-0 flex-1 truncate">{account.email_display}</span><span aria-hidden="true"
			>{expanded ? '⌄' : '›'}</span
		>
	</button>
	{#if expanded}
		<div class="ml-4 border-l border-[var(--separator)] pl-2">
			{#each ['Inbox', 'Starred', 'Snoozed'] as view}
				{@render item(view as MailboxView, t[view as 'Inbox' | 'Starred' | 'Snoozed'])}
			{/each}
			{#each groups.special as folder (folder.folder_name)}{@render item(
					{ Folder: folder.folder_name },
					name(folder)
				)}{/each}
			{#if groups.other.length}
				<button
					type="button"
					aria-expanded={otherOpen}
					onclick={() => (otherOpen = !otherOpen)}
					class="flex w-full items-center justify-between gap-2 rounded px-2 py-2 text-left text-sm"
					><span>{t.other}</span><span aria-hidden="true">{otherOpen ? '⌄' : '›'}</span></button
				>
				{#if otherOpen}<div class="ml-2">
						{#each groups.other as folder (folder.folder_name)}{@render item(
								{ Folder: folder.folder_name },
								folder.display_name
							)}{/each}
					</div>{/if}
			{/if}
			{#if catalog?.loading}<p role="status" class="px-2 py-2 text-xs text-[var(--secondary)]">
					{t.loadingFolders}
				</p>{/if}
			{#if catalog?.error}<p role="status" class="px-2 py-2 text-xs text-[var(--secondary)]">
					{t.catalogError}
				</p>
				<button type="button" onclick={retry} class="px-2 py-2 text-xs text-[var(--accent)]"
					>{t.retry}</button
				>
			{:else if catalog?.loaded && !catalog.folders.length}<p
					role="status"
					class="px-2 py-2 text-xs text-[var(--secondary)]"
				>
					{t.notSynced}
				</p>{/if}
		</div>
	{/if}
</div>
{#snippet item(view: MailboxView, label: string)}
	<button
		type="button"
		aria-current={sameView(scope, account.account_id, view) ? 'page' : undefined}
		onclick={() => choose(view)}
		title={label}
		class="block w-full truncate rounded-lg px-2 py-2 text-left text-sm aria-[current=page]:bg-[var(--selection-soft)] aria-[current=page]:text-[var(--accent)] hover:bg-[var(--surface)]"
		>{label}</button
	>
{/snippet}
