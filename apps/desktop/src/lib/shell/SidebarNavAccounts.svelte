<script lang="ts">
	import Archive from '@lucide/svelte/icons/archive';
	import AtSign from '@lucide/svelte/icons/at-sign';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Inbox from '@lucide/svelte/icons/inbox';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { t } from '$lib/i18n';
	import { avatarFallback } from '$lib/onboarding/avatar-controller.svelte';
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
			{@const avatarSrc = appStore.accountAvatars[account.account_id] ?? ''}
			{@const fallback = avatarFallback(account.account_id, account.address)}
			{@const collapsed = appStore.isAccountCollapsed(account.account_id)}
			<div class="flex flex-col gap-1">
				<button
					type="button"
					onclick={() => appStore.toggleAccountCollapsed(account.account_id)}
					aria-expanded={!collapsed}
					aria-label="{collapsed ? t(appStore.locale, 'expandAccountOptions') : t(appStore.locale, 'collapseAccountOptions')}: {account.address}"
					class="flex min-h-9 w-full items-center gap-2 rounded-md px-2 py-1 text-left text-[11px] font-medium text-foreground transition-colors hover:bg-field md:min-h-0"
				>
					{#if avatarSrc}
						<img
							src={avatarSrc}
							alt=""
							referrerpolicy="no-referrer"
							class="h-5 w-5 shrink-0 rounded-full border border-separator object-cover"
						/>
					{:else}
						<span
							aria-hidden="true"
							class="grid h-5 w-5 shrink-0 place-items-center rounded-full border border-separator font-display text-[9px] font-semibold {fallback.color ===
							0
								? 'bg-accent text-accent-foreground'
								: 'bg-field text-foreground'}"
						>
							{fallback.initials}
						</span>
					{/if}
					<span class="min-w-0 flex-1 truncate">{account.address}</span>
					<WdgAccountPlanBadge
						label={appStore.accountSubscriptions[account.account_id]?.effective_badge ?? null}
						quota={appStore.accountSubscriptions[account.account_id]?.quota}
					/>
					{#if collapsed}
						<ChevronRight class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
					{:else}
						<ChevronDown class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
					{/if}
				</button>
				{#if !collapsed}
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
					{#each appStore.accountAliases[account.account_id] ?? [] as alias (alias.address)}
						{@const aliasScope: ScopeFilter = { kind: 'alias', account_id: account.account_id, address: alias.address }}
						<button
							type="button"
							title={alias.address}
							aria-label="{t(appStore.locale, 'aliasMailbox')} {alias.address}"
							onclick={() => void onSelectScope(aliasScope)}
							class="flex min-h-11 w-full items-center rounded-md py-1 pl-6 pr-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
								aliasScope
							)
								? 'bg-selection font-medium text-foreground'
								: 'text-secondary hover:bg-field hover:text-foreground'}"
						>
							<span class="flex items-center gap-2 truncate">
								<AtSign class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
								<span class="truncate">{alias.label || alias.address}</span>
							</span>
						</button>
					{/each}
				{/if}
			</div>
		{/each}
	</section>
{/if}
