<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { FolderDto } from '../inbox/types';

	export interface FolderCatalogState {
		loading: boolean;
		error: boolean;
		folders: FolderDto[];
		loaded: boolean;
	}

	interface Props {
		catalog?: FolderCatalogState;
		onRetry: () => void | Promise<void>;
	}

	let { catalog, onRetry }: Props = $props();
	const t = $derived(i18n.t.onboarding);
</script>

<div class="overflow-x-auto border-t border-separator bg-toolbar/50 px-4 py-3">
	{#if catalog?.loading}
		<p role="status" class="text-xs text-secondary">{t.loadingFolders}</p>
	{:else if catalog?.error}
		<div class="flex items-center justify-between gap-2 text-xs text-status-warning">
			<span>{t.foldersUnavailable}</span>
			<button
				type="button"
				onclick={() => void onRetry()}
				class="rounded border border-separator bg-surface px-2 py-1 text-foreground hover:bg-field"
			>
				{i18n.t.mailboxes.retry}
			</button>
		</div>
	{:else if !catalog?.folders.length}
		<p class="text-xs text-secondary">{t.noFolders}</p>
	{:else}
		<table class="w-full text-left text-xs">
			<thead>
				<tr class="border-b border-separator text-[11px] font-medium text-secondary">
					<th class="py-2 pr-4">{t.folderColumn}</th>
					<th class="py-2 pr-4">{t.roleColumn}</th>
					<th class="py-2 pr-4 text-right">{t.totalColumn}</th>
					<th class="py-2 text-right">{t.unreadColumn}</th>
				</tr>
			</thead>
			<tbody class="divide-y divide-separator">
				{#each catalog.folders as folder (folder.folder_name)}
					<tr class="h-9 hover:bg-field/60">
						<td class="py-2 pr-4 font-medium text-foreground">
							{folder.display_name || folder.folder_name}
						</td>
						<td class="py-2 pr-4 font-mono text-[11px] text-secondary">{folder.role}</td>
						<td class="py-2 pr-4 text-right font-mono tabular-nums text-foreground">
							{folder.total_messages ?? '—'}
						</td>
						<td
							class="py-2 text-right font-mono tabular-nums {(folder.unread_messages ?? 0) > 0
								? 'font-semibold text-accent'
								: 'text-secondary'}"
						>
							{folder.unread_messages ?? '—'}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>
