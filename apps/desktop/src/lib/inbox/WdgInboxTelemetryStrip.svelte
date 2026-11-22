<script lang="ts">
	import { formatSyncStatus, t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import type { InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace: InboxWorkspace;
	};

	let { workspace }: Props = $props();
</script>

<div
	class="grid grid-cols-2 gap-2 border-t border-separator bg-surface px-3 py-2 text-xs sm:grid-cols-4"
>
	<div class="flex items-center justify-between gap-2 border-r border-separator pr-2">
		<span class="truncate text-secondary">{t(appStore.locale, 'loadedMessages')}</span>
		<span class="font-mono font-medium tabular-nums text-foreground">
			{workspace.inbox.rows.length.toLocaleString()}{workspace.inbox.hasMore ? '+' : ''}
		</span>
	</div>
	<div class="flex items-center justify-between gap-2 sm:border-r sm:border-separator sm:pr-2">
		<span class="truncate text-secondary">{t(appStore.locale, 'unreadCount')}</span>
		<span class="font-mono font-medium tabular-nums text-foreground">
			{appStore.unreadTotal}
		</span>
	</div>
	<div class="flex items-center justify-between gap-2 border-r border-separator pr-2">
		<span class="truncate text-secondary">{t(appStore.locale, 'pendingBoletos')}</span>
		<span class="font-mono font-medium tabular-nums text-status-warning">
			{workspace.pendingBoletosCount}
		</span>
	</div>
	<div class="flex items-center justify-between gap-2">
		<span class="truncate text-secondary">{t(appStore.locale, 'syncWorker')}</span>
		<span class="flex items-center gap-2 font-mono font-medium tabular-nums text-foreground">
			<span>{formatSyncStatus(appStore.locale, appStore.syncStatus)}</span>
			{#if workspace.inbox.totalEmbeddingErrors > 0}
				<span class="text-status-danger">({workspace.inbox.totalEmbeddingErrors} err)</span>
			{/if}
		</span>
	</div>
</div>
