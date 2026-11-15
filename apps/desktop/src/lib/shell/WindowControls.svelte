<script lang="ts">
	import { Copy, GripHorizontal, Minus, Square, X } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import { windowController } from './window.svelte';

	const isMaximized = $derived(windowController.mode === 'maximized');
</script>

<div
	role="group"
	aria-label={t(appStore.locale, 'windowControls')}
	class="window-no-drag flex items-center gap-0.5 rounded-md border border-separator bg-surface/80 p-0.5"
>
	<button
		type="button"
		data-tauri-drag-region
		onpointerdown={(e) => windowController.startDrag(e)}
		onclick={(e) => {
			if (e.detail === 0) {
				void windowController.toggleMaximize();
			}
		}}
		aria-label={t(appStore.locale, 'windowDrag')}
		title={t(appStore.locale, 'windowDrag')}
		class="window-drag-region flex h-7 w-7 cursor-grab items-center justify-center rounded text-secondary transition-colors hover:bg-field hover:text-foreground active:cursor-grabbing"
	>
		<GripHorizontal class="pointer-events-none h-3.5 w-3.5" />
	</button>

	<button
		type="button"
		onclick={() => void windowController.minimize()}
		aria-label={t(appStore.locale, 'windowMinimize')}
		title={t(appStore.locale, 'windowMinimize')}
		class="window-no-drag flex h-7 w-7 items-center justify-center rounded text-secondary transition-colors hover:bg-field hover:text-foreground active:scale-[0.96]"
	>
		<Minus class="h-3.5 w-3.5" />
	</button>

	<button
		type="button"
		aria-pressed={isMaximized}
		onclick={() => void windowController.toggleMaximize()}
		aria-label={isMaximized
			? t(appStore.locale, 'windowRestore')
			: t(appStore.locale, 'windowMaximize')}
		title={isMaximized
			? t(appStore.locale, 'windowRestore')
			: t(appStore.locale, 'windowMaximize')}
		class="window-no-drag flex h-7 w-7 items-center justify-center rounded text-secondary transition-colors hover:bg-field hover:text-foreground active:scale-[0.96]"
	>
		{#if isMaximized}
			<Copy class="h-3 w-3" />
		{:else}
			<Square class="h-3 w-3" />
		{/if}
	</button>

	<button
		type="button"
		onclick={() => void windowController.close()}
		aria-label={t(appStore.locale, 'windowClose')}
		title={t(appStore.locale, 'windowClose')}
		class="window-no-drag flex h-7 w-7 items-center justify-center rounded text-secondary transition-colors hover:bg-status-danger/15 hover:text-status-danger active:scale-[0.96]"
	>
		<X class="h-3.5 w-3.5" />
	</button>
</div>
