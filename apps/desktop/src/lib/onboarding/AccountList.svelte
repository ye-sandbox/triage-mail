<script lang="ts">
	import type { AccountConfig } from './types';
	import AccountPhoto from './AccountPhoto.svelte';
	import type { AvatarTransport } from './avatar-types';
	import { labels as backupLabels } from '../backup/labels';
	import { i18n } from '../i18n/index.svelte';

	let {
		accounts = [],
		avatarTransport
	}: {
		accounts: AccountConfig[];
		avatarTransport?: AvatarTransport;
	} = $props();

	const t = $derived(i18n.t.onboarding);
	const formT = $derived(i18n.t.accountForm);
</script>

<ul
	class="divide-y divide-[var(--separator)] rounded-2xl border border-[var(--separator)] bg-[var(--surface)] px-6"
	aria-label={t.connectedAccounts}
>
	{#each accounts as account (account.account_id)}
		<li class="space-y-4 py-4">
			<AccountPhoto
				accountId={account.account_id}
				label={account.email_display}
				transport={avatarTransport}
			/>
			<p class="break-all font-medium text-[var(--text)]">
				{account.email_display}
			</p>
			<p class="mt-1 break-all text-sm text-[var(--secondary)]">
				{#if !account.imap_host}{backupLabels[i18n.locale]
						.disconnected}{:else}{account.imap_host}:<span class="tabular-nums"
						>{account.imap_port}</span
					>
					· {account.auth_method === 'AppPassword'
						? formT.methodAppPassword
						: account.auth_method === 'ProtonBridgeLocal'
							? 'Proton Bridge'
							: 'OAuth'}{/if}
			</p>
		</li>
	{/each}
</ul>
