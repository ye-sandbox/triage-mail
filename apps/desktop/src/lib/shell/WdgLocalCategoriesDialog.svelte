<script lang="ts">
	import { Dialog } from 'bits-ui';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import Check from '@lucide/svelte/icons/check';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Plus from '@lucide/svelte/icons/plus';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import X from '@lucide/svelte/icons/x';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';

	type Props = {
		open: boolean;
		onOpenChange: (next: boolean) => void;
	};

	let { open, onOpenChange }: Props = $props();

	let newName = $state('');
	let editingId = $state<string | null>(null);
	let editingName = $state('');

	function submitAdd(event: SubmitEvent) {
		event.preventDefault();
		const created = appStore.addLocalCategory(newName);
		if (created) newName = '';
	}

	function startEdit(id: string) {
		editingId = id;
		editingName = appStore.formatCategoryDisplay(id);
	}

	function commitEdit(id: string) {
		appStore.renameLocalCategory(id, editingName);
		editingId = null;
		editingName = '';
	}

	function handleRemove(id: string) {
		appStore.removeLocalCategory(id);
		if (
			(appStore.activeScope.kind === 'category' && appStore.activeScope.category === id) ||
			(appStore.activeScope.kind === 'custom_category' && appStore.activeScope.category_id === id)
		) {
			appStore.setScope({ kind: 'inbox' });
		}
	}
</script>

<Dialog.Root {open} {onOpenChange}>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			class="fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[min(92vw,480px)] -translate-x-1/2 -translate-y-1/2 flex-col gap-4 rounded-xl border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
		>
			<div>
				<Dialog.Title class="font-display text-sm font-semibold">
					{t(appStore.locale, 'customizeCategories')}
				</Dialog.Title>
				<Dialog.Description class="mt-1 text-xs text-secondary">
					{t(appStore.locale, 'customizeCategoriesDesc')}
				</Dialog.Description>
			</div>

			<form onsubmit={submitAdd} class="flex items-center gap-2">
				<input
					type="text"
					bind:value={newName}
					maxlength={64}
					placeholder={t(appStore.locale, 'categoryNamePlaceholder')}
					aria-label={t(appStore.locale, 'addCategory')}
					class="h-9 flex-1 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
				/>
				<button
					type="submit"
					disabled={!newName.trim()}
					class="flex min-h-9 items-center gap-1 rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground disabled:opacity-40"
				>
					<Plus class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t(appStore.locale, 'addCategory')}</span>
				</button>
			</form>

			<div class="flex max-h-64 flex-col gap-1 overflow-y-auto pr-1">
				{#each appStore.visibleLocalCategories as item, idx (item.id)}
					<div class="flex items-center justify-between gap-2 rounded-md border border-separator bg-field px-2 py-1 text-xs">
						{#if editingId === item.id}
							<form
								onsubmit={(e) => {
									e.preventDefault();
									commitEdit(item.id);
								}}
								class="flex flex-1 items-center gap-1"
							>
								<input
									type="text"
									bind:value={editingName}
									maxlength={64}
									aria-label={t(appStore.locale, 'renameCategory')}
									class="h-7 flex-1 rounded border border-separator bg-surface px-2 text-xs text-foreground focus:border-accent focus:outline-none"
								/>
								<button
									type="submit"
									aria-label={t(appStore.locale, 'save')}
									class="grid h-7 w-7 place-items-center rounded text-status-healthy hover:bg-surface"
								>
									<Check class="h-3.5 w-3.5" aria-hidden="true" />
								</button>
								<button
									type="button"
									onclick={() => (editingId = null)}
									aria-label={t(appStore.locale, 'cancel')}
									class="grid h-7 w-7 place-items-center rounded text-secondary hover:bg-surface hover:text-foreground"
								>
									<X class="h-3.5 w-3.5" aria-hidden="true" />
								</button>
							</form>
						{:else}
							<span class="flex min-w-0 flex-1 items-center gap-2 truncate font-medium">
								<span class="truncate">{appStore.formatCategoryDisplay(item.id)}</span>
								{#if item.kind === 'custom'}
									<span class="rounded bg-surface px-1 text-[10px] text-secondary">#{item.tag}</span>
								{/if}
							</span>
							<div class="flex shrink-0 items-center gap-1">
								<button
									type="button"
									disabled={idx === 0}
									onclick={() => appStore.moveLocalCategory(item.id, 'up')}
									aria-label={t(appStore.locale, 'moveUp')}
									class="grid h-7 w-7 place-items-center rounded text-secondary hover:bg-surface hover:text-foreground disabled:opacity-30"
								>
									<ArrowUp class="h-3.5 w-3.5" aria-hidden="true" />
								</button>
								<button
									type="button"
									disabled={idx === appStore.visibleLocalCategories.length - 1}
									onclick={() => appStore.moveLocalCategory(item.id, 'down')}
									aria-label={t(appStore.locale, 'moveDown')}
									class="grid h-7 w-7 place-items-center rounded text-secondary hover:bg-surface hover:text-foreground disabled:opacity-30"
								>
									<ArrowDown class="h-3.5 w-3.5" aria-hidden="true" />
								</button>
								<button
									type="button"
									onclick={() => startEdit(item.id)}
									aria-label="{t(appStore.locale, 'renameCategory')} {appStore.formatCategoryDisplay(item.id)}"
									class="grid h-7 w-7 place-items-center rounded text-secondary hover:bg-surface hover:text-foreground"
								>
									<Pencil class="h-3.5 w-3.5" aria-hidden="true" />
								</button>
								<button
									type="button"
									onclick={() => handleRemove(item.id)}
									aria-label="{t(appStore.locale, 'removeCategory')} {appStore.formatCategoryDisplay(item.id)}"
									class="grid h-7 w-7 place-items-center rounded text-secondary hover:bg-surface hover:text-status-danger"
								>
									<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
								</button>
							</div>
						{/if}
					</div>
				{/each}
			</div>

			{#if appStore.hiddenBuiltinCategories.length > 0}
				<div class="space-y-2 border-t border-separator pt-3">
					<div class="text-[11px] font-medium text-secondary">
						{t(appStore.locale, 'hiddenCategories')}
					</div>
					<div class="flex flex-wrap gap-1">
						{#each appStore.hiddenBuiltinCategories as hiddenItem (hiddenItem.id)}
							<button
								type="button"
								onclick={() => appStore.restoreLocalCategory(hiddenItem.id)}
								class="flex items-center gap-1 rounded-md border border-separator bg-field px-2 py-1 text-[11px] text-secondary hover:text-foreground"
							>
								<Plus class="h-3 w-3" aria-hidden="true" />
								<span>{appStore.formatCategoryDisplay(hiddenItem.id)}</span>
							</button>
						{/each}
					</div>
				</div>
			{/if}

			<p class="text-[11px] text-secondary">{t(appStore.locale, 'customCategoryHint')}</p>

			<div class="flex items-center justify-between gap-2 border-t border-separator pt-3">
				<button
					type="button"
					onclick={() => appStore.resetLocalCategories()}
					class="flex min-h-9 items-center gap-1 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground"
				>
					<RotateCcw class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t(appStore.locale, 'restoreDefaultCategories')}</span>
				</button>
				<button
					type="button"
					onclick={() => onOpenChange(false)}
					class="min-h-9 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground"
				>
					{t(appStore.locale, 'close')}
				</button>
			</div>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
