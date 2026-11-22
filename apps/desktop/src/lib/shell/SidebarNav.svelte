<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Bookmark from '@lucide/svelte/icons/bookmark';
	import Clock from '@lucide/svelte/icons/clock';
	import Folder from '@lucide/svelte/icons/folder';
	import Inbox from '@lucide/svelte/icons/inbox';
	import Layers from '@lucide/svelte/icons/layers';
	import Star from '@lucide/svelte/icons/star';
	import UserCheck from '@lucide/svelte/icons/user-check';
	import UserX from '@lucide/svelte/icons/user-x';
	import { formatCategoryLabel, t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import type { MessageCategory, ScopeFilter } from '$lib/types';
	import SidebarNavAccounts from './SidebarNavAccounts.svelte';
	import SidebarNavRoutes from './SidebarNavRoutes.svelte';

	type Props = {
		onNavigate?: () => void;
	};

	let { onNavigate }: Props = $props();

	const CATEGORIES: MessageCategory[] = [
		'Security',
		'Financial',
		'Shopping',
		'Newsletter',
		'Notifications',
		'Personal'
	];

	const pathname = $derived(page.url.pathname);

	function isScopeActive(scope: ScopeFilter): boolean {
		if (pathname !== '/') return false;
		const cur = appStore.activeScope;
		if (cur.kind !== scope.kind) return false;
		if (cur.kind === 'folder' && scope.kind === 'folder') {
			return cur.account_id === scope.account_id && cur.folder === scope.folder;
		}
		if (cur.kind === 'alias' && scope.kind === 'alias') {
			return cur.account_id === scope.account_id && cur.address === scope.address;
		}
		if (cur.kind === 'new_senders' && scope.kind === 'new_senders') {
			return cur.status === scope.status;
		}
		if (cur.kind === 'category' && scope.kind === 'category') {
			return cur.category === scope.category;
		}
		if (cur.kind === 'saved_view' && scope.kind === 'saved_view') {
			return cur.view_id === scope.view_id;
		}
		return true;
	}

	async function selectScope(scope: ScopeFilter) {
		appStore.setScope(scope);
		if (pathname !== '/') {
			try {
				await goto('/');
			} catch {
				// Unit tests mount SidebarNav without booting SvelteKit's client router.
			}
		}
		onNavigate?.();
	}
</script>

<nav aria-label="Workspace navigation" class="flex h-full flex-col gap-4 overflow-y-auto p-3 text-xs">
	<section class="flex flex-col gap-1">
		<div class="px-2 py-1 font-display text-[11px] font-semibold tracking-wide text-secondary">
			{t(appStore.locale, 'workspace')}
		</div>
		<button
			type="button"
			onclick={() => void selectScope({ kind: 'all' })}
			class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
				{ kind: 'all' }
			)
				? 'bg-selection font-medium text-foreground'
				: 'text-secondary hover:bg-field hover:text-foreground'}"
		>
			<span class="flex items-center gap-2 truncate">
				<Layers class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span class="truncate">{t(appStore.locale, 'globalAllMail')}</span>
			</span>
			<span class="font-mono text-[11px] tabular-nums">{appStore.loadedCount}</span>
		</button>
		<button
			type="button"
			onclick={() => void selectScope({ kind: 'inbox' })}
			class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
				{ kind: 'inbox' }
			)
				? 'bg-selection font-medium text-foreground'
				: 'text-secondary hover:bg-field hover:text-foreground'}"
		>
			<span class="flex items-center gap-2 truncate">
				<Inbox class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span class="truncate">{t(appStore.locale, 'inbox')}</span>
			</span>
			<span class="font-mono text-[11px] tabular-nums">{appStore.unreadTotal}</span>
		</button>
		<button
			type="button"
			onclick={() => void selectScope({ kind: 'starred' })}
			class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
				{ kind: 'starred' }
			)
				? 'bg-selection font-medium text-foreground'
				: 'text-secondary hover:bg-field hover:text-foreground'}"
		>
			<span class="flex items-center gap-2 truncate">
				<Star class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span class="truncate">{t(appStore.locale, 'starred')}</span>
			</span>
		</button>
		<button
			type="button"
			onclick={() => void selectScope({ kind: 'snoozed' })}
			class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
				{ kind: 'snoozed' }
			)
				? 'bg-selection font-medium text-foreground'
				: 'text-secondary hover:bg-field hover:text-foreground'}"
		>
			<span class="flex items-center gap-2 truncate">
				<Clock class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span class="truncate">{t(appStore.locale, 'snoozed')}</span>
			</span>
		</button>
	</section>

	<SidebarNavAccounts {isScopeActive} onSelectScope={selectScope} />

	<section class="flex flex-col gap-1 border-t border-separator pt-3">
		<div class="px-2 py-1 font-display text-[11px] font-semibold tracking-wide text-secondary">
			{t(appStore.locale, 'newSendersReview')}
		</div>
		<button
			type="button"
			onclick={() => void selectScope({ kind: 'new_senders', status: 'Pending' })}
			class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
				{ kind: 'new_senders', status: 'Pending' }
			)
				? 'bg-selection font-medium text-foreground'
				: 'text-secondary hover:bg-field hover:text-foreground'}"
		>
			<span class="flex items-center gap-2 truncate">
				<UserCheck class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span>{t(appStore.locale, 'pending')}</span>
			</span>
			<span class="font-mono text-[11px] tabular-nums">
				{appStore.senderGovernance.filter((s) => s.status === 'Pending').length}
			</span>
		</button>
		<button
			type="button"
			onclick={() => void selectScope({ kind: 'new_senders', status: 'Declined' })}
			class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-2 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
				{ kind: 'new_senders', status: 'Declined' }
			)
				? 'bg-selection font-medium text-foreground'
				: 'text-secondary hover:bg-field hover:text-foreground'}"
		>
			<span class="flex items-center gap-2 truncate">
				<UserX class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span>{t(appStore.locale, 'declined')}</span>
			</span>
			<span class="font-mono text-[11px] tabular-nums">
				{appStore.senderGovernance.filter((s) => s.status === 'Declined').length}
			</span>
		</button>

		<div class="mt-2 px-2 py-1 font-display text-[11px] font-semibold tracking-wide text-secondary">
			{t(appStore.locale, 'localCategories')}
		</div>
		{#each CATEGORIES as cat (cat)}
			{@const scope: ScopeFilter = { kind: 'category', category: cat }}
			<button
				type="button"
				onclick={() => void selectScope(scope)}
				class="flex min-h-11 w-full items-center gap-2 rounded-md px-2 py-1 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
					scope
				)
					? 'bg-selection font-medium text-foreground'
					: 'text-secondary hover:bg-field hover:text-foreground'}"
			>
				<Folder class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span class="truncate">{formatCategoryLabel(appStore.locale, cat)}</span>
			</button>
		{/each}

		{#if appStore.savedViews.length > 0}
			<div
				class="mt-2 px-2 py-1 font-display text-[11px] font-semibold tracking-wide text-secondary"
			>
				{t(appStore.locale, 'savedViews')}
			</div>
			{#each appStore.savedViews as view (view.id)}
				{@const scope: ScopeFilter = { kind: 'saved_view', view_id: view.id }}
				<button
					type="button"
					onclick={() => void selectScope(scope)}
					class="flex min-h-11 w-full items-center gap-2 rounded-md px-2 py-1 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {isScopeActive(
						scope
					)
						? 'bg-selection font-medium text-foreground'
						: 'text-secondary hover:bg-field hover:text-foreground'}"
				>
					<Bookmark class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
					<span class="truncate">{view.name}</span>
				</button>
			{/each}
		{/if}
	</section>

	<SidebarNavRoutes {onNavigate} />
</nav>
