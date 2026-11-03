<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';
	import AccountMailbox from './AccountMailbox.svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountConfig } from '../onboarding/types';
	import type { AvatarTransport } from '../onboarding/avatar-types';
	import type { MailboxScope } from './types';
	import type { Mailboxes } from './mailboxes.svelte';
	let {
		children,
		savedActive = false,
		status,
		openSettings,
		openDueDates,
		dueCount = 0,
		accounts = [],
		mailboxes,
		scope,
		select = () => {},
		avatarTransport,
		photoRevision = 0,
		dismiss
	}: {
		children?: Snippet;
		savedActive?: boolean;
		status: string;
		openSettings?: () => void;
		openDueDates?: () => void;
		dueCount?: number;
		accounts?: AccountConfig[];
		mailboxes?: Mailboxes;
		scope?: MailboxScope;
		select?: (scope?: MailboxScope) => void;
		avatarTransport?: AvatarTransport;
		photoRevision?: number;
		dismiss?: () => void;
	} = $props();
	const t = $derived(i18n.t.mailboxes);
</script>

<aside
	class="flex h-full min-h-0 flex-col border-r border-[var(--separator)] bg-[var(--sidebar)] px-4 py-6"
	aria-label={t.navigation}
>
	<button type="button" class="mb-4 self-end px-2 py-2 text-sm md:hidden" onclick={dismiss}
		>{i18n.t.mailboxes.close}</button
	>
	<span class="mb-8 flex items-center gap-3 px-2 text-base font-semibold tracking-tight"
		><Icon name="inbox" />Triage Mail</span
	>
	<p class="mb-2 px-2 text-xs font-semibold text-[var(--secondary)]">
		{t.mailboxes}
	</p>
	<nav aria-label={t.mailboxes} class="min-h-0 flex-1 overflow-y-auto overscroll-contain">
		<button
			type="button"
			onclick={() => select()}
			aria-current={!savedActive && (!scope || scope === 'GlobalAllMail') ? 'page' : undefined}
			class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm font-medium aria-[current=page]:bg-[var(--selection-soft)]"
			><Icon name="inbox" size={18} />{t.allMail}</button
		>
		{#if openDueDates}
			<button
				type="button"
				onclick={openDueDates}
				class="flex w-full items-center justify-between rounded-lg px-2 py-2 text-left text-sm font-medium text-[var(--foreground)] hover:bg-[var(--surface)] transition-colors"
			>
				<span class="flex items-center gap-2"><Icon name="calendar" size={18} />{t.dueDates}</span>
				{#if dueCount > 0}
					<span
						class="rounded-full bg-amber-500/15 px-1.5 py-0.5 text-xs font-semibold text-amber-600 dark:text-amber-400"
					>
						{dueCount}
					</span>
				{/if}
			</button>
		{/if}
		{#each accounts as account (account.account_id)}
			<AccountMailbox
				{account}
				expanded={mailboxes?.expanded.includes(account.account_id) ?? false}
				toggle={() => mailboxes?.toggle(account.account_id)}
				catalog={mailboxes?.catalogs[account.account_id]}
				{scope}
				{select}
				retry={() => void mailboxes?.refresh(account.account_id)}
				{avatarTransport}
				{photoRevision}
			/>
		{/each}
		{@render children?.()}
	</nav>
	<div class="mt-4 shrink-0 space-y-4 px-2">
		<div class="flex items-center gap-2 text-xs text-[var(--secondary)]" role="status">
			<span class="size-1.5 shrink-0 rounded-full bg-[var(--accent)]"></span><span class="truncate"
				>{status}</span
			>
		</div>
		{#if openSettings}<button
				type="button"
				class="flex w-full items-center gap-2 rounded-lg py-2 text-left text-xs text-[var(--secondary)] hover:bg-[var(--surface)]"
				aria-label="Settings"
				aria-haspopup="dialog"
				onclick={openSettings}><Icon name="settings" size={16} />{t.settings}</button
			>{/if}
	</div>
</aside>
