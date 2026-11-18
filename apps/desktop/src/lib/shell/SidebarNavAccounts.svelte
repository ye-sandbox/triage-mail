<script lang="ts">
	import { Archive, Inbox, Mail, Trash2 } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import WdgAccountPlanBadge from '$lib/onboarding/WdgAccountPlanBadge.svelte';
	import type { FolderId, ScopeFilter } from '$lib/types';

	type Props = {
		isScopeActive: (scope: ScopeFilter) => boolean;
		onSelectScope: (scope: ScopeFilter) => Promise<void>;
	};

	let { isScopeActive, onSelectScope }: Props = $props();

	function folderIcon(folder: FolderId) {
		if (folder === 'INBOX') return Inbox;
		if (folder === 'Archive') return Archive;
		return Trash2;
	}
</script>

{#if appStore.accounts.length > 0}
	<section class="flex flex-col gap-2 border-t border-separator pt-3">
		<div class="px-2 font-display text-[11px] font-semibold tracking-wide text-secondary">
			{t(appStore.locale, 'accounts')}
		</div>
		{#each appStore.accounts as account (account.account_id)}
			<div class="flex flex-col gap-0.5">
				<div class="flex items-center gap-1.5 px-2 py-1 text-[11px] font-medium text-foreground">
					<Mail class="h-3 w-3 shrink-0 text-secondary" />
					<span class="truncate">{account.address}</span>
					<WdgAccountPlanBadge
						label={appStore.accountSubscriptions[account.account_id]?.effective_badge ?? null}
						quota={appStore.accountSubscriptions[account.account_id]?.quota}
					/>
				</div>
				{#each account.folders as folder (folder.id)}
					{@const Icon = folderIcon(folder.id)}
					{@const unread = appStore.unreadForAccountFolder(account.account_id, folder.id)}
					{@const scope: ScopeFilter = { kind: 'folder', account_id: account.account_id, folder: folder.id }}
					<button
						type="button"
						onclick={() => void onSelectScope(scope)}
						class="flex min-h-11 w-full items-center justify-between rounded-md py-1 pl-6 pr-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
							scope
						)
							? 'bg-selection font-medium text-foreground'
							: 'text-secondary hover:bg-field hover:text-foreground'}"
					>
						<span class="flex items-center gap-2 truncate">
							<Icon class="h-3.5 w-3.5 shrink-0" />
							<span class="truncate">{folder.label}</span>
						</span>
						{#if unread > 0}
							<span class="font-mono text-[11px] tabular-nums text-foreground">{unread}</span>
						{/if}
					</button>
				{/each}
			</div>
		{/each}
	</section>
{/if}
