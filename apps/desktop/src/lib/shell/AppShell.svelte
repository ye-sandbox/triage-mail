<script lang="ts">
	import type { Snippet } from 'svelte';
	import {
		AlertTriangle,
		CheckCircle2,
		Command,
		Menu,
		RefreshCw,
		RotateCw,
		ShieldAlert
	} from 'lucide-svelte';
	import { Drawer } from 'vaul-svelte';
	import { formatSyncStatus, t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import { createTauriWindowTransport } from '$lib/window/transport';
	import type { WindowTransport } from '$lib/window/types';
	import SidebarNav from './SidebarNav.svelte';
	import WindowControls from './WindowControls.svelte';
	import WorkstationDock from './WorkstationDock.svelte';
	import { windowController } from './window.svelte';

	type Props = {
		children: Snippet;
		transport?: WindowTransport;
	};

	let { children, transport = createTauriWindowTransport() }: Props = $props();

	$effect(() => {
		return windowController.connect(transport);
	});

	function handleGlobalKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			if (windowController.mode === 'minimized') {
				windowController.restoreFromDock();
			} else if (windowController.mode === 'closed') {
				windowController.reopen();
			}
			appStore.commandOpen = !appStore.commandOpen;
		}
	}

	const syncColorClass = $derived.by(() => {
		switch (appStore.syncStatus) {
			case 'Idle':
				return 'text-status-healthy';
			case 'Syncing':
			case 'MutationPending':
				return 'text-status-info';
			case 'Retrying':
			case 'ResyncRequired':
				return 'text-status-warning';
			case 'MutationFailed':
				return 'text-status-danger';
		}
	});

	const isRestored = $derived(windowController.mode === 'restored');
	const isMaximized = $derived(windowController.mode === 'maximized');
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if windowController.mode === 'closed'}
	<WorkstationDock variant="closed" />
{:else}
	<div
		class="desktop-shell relative h-screen w-screen overflow-hidden {isRestored
			? 'workstation-backdrop bg-sidebar/85'
			: 'bg-background'}"
	>
		{#if windowController.mode === 'minimized'}
			<WorkstationDock variant="minimized" />
		{:else}
			<div
				style={isRestored && !transport.isAvailable()
					? `left:${windowController.x}px;top:${windowController.y}px;width:${windowController.width}px;height:${windowController.height}px;`
					: undefined}
				class="flex flex-col overflow-hidden bg-background text-foreground {isMaximized ||
				transport.isAvailable()
					? 'h-screen w-screen'
					: 'fixed rounded-xl border border-separator shadow-2xl'} {windowController.isDragging
					? 'select-none'
					: ''}"
			>
				<header
					aria-label={t(appStore.locale, 'appTitle')}
					class="flex h-11 shrink-0 items-center justify-between gap-2 border-b border-separator bg-toolbar px-2.5 sm:px-3"
				>
					<div class="window-no-drag flex shrink-0 items-center gap-1.5 sm:gap-2">
						<button
							type="button"
							onclick={() => (appStore.mobileDrawerOpen = true)}
							aria-label={t(appStore.locale, 'openDrawer')}
							class="flex min-h-9 min-w-9 items-center justify-center rounded-md text-secondary hover:bg-field hover:text-foreground md:hidden"
						>
							<Menu class="h-4 w-4" />
						</button>
						<a
							href="/"
							class="shrink-0 whitespace-nowrap font-display text-sm font-semibold tracking-tight text-foreground transition-opacity"
							class:opacity-70={!windowController.focused}
						>
							{t(appStore.locale, 'appTitle')}
						</a>
						{#if windowController.actionError}
							<span role="status" aria-live="polite" class="truncate text-xs text-secondary">
								{windowController.actionError}
							</span>
						{/if}
					</div>

					<div
						role="presentation"
						data-tauri-drag-region
						onpointerdown={(e) => void windowController.startDrag(e)}
						ondblclick={() => void windowController.toggleMaximize()}
						title={t(appStore.locale, 'windowDrag')}
						class="window-drag-region h-full min-w-4 flex-1 cursor-grab active:cursor-grabbing"
					></div>

					<div class="window-no-drag flex shrink-0 items-center gap-1.5 sm:gap-2">
						<button
							type="button"
							onclick={() => appStore.triggerSync()}
							class="flex h-8 shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border border-separator bg-surface px-2 py-1 text-xs font-medium transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] sm:px-2.5 md:h-7"
							title={t(appStore.locale, 'syncNow')}
						>
							{#if appStore.syncStatus === 'Idle'}
								<CheckCircle2 class="h-3.5 w-3.5 {syncColorClass}" />
							{:else if appStore.syncStatus === 'Syncing' || appStore.syncStatus === 'MutationPending'}
								<RefreshCw class="h-3.5 w-3.5 animate-spin {syncColorClass}" />
							{:else if appStore.syncStatus === 'MutationFailed'}
								<ShieldAlert class="h-3.5 w-3.5 {syncColorClass}" />
							{:else}
								<AlertTriangle class="h-3.5 w-3.5 {syncColorClass}" />
							{/if}
							<span class="hidden sm:inline {syncColorClass}"
								>{formatSyncStatus(appStore.locale, appStore.syncStatus)}</span
							>
						</button>

						<button
							type="button"
							onclick={() => (appStore.commandOpen = true)}
							aria-label={t(appStore.locale, 'commandPalette')}
							class="flex h-8 shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-2 py-1 text-xs text-secondary transition-transform duration-100 ease-out hover:text-foreground active:scale-[0.98] sm:px-2.5 md:h-7"
						>
							<Command class="h-3.5 w-3.5 shrink-0" />
							<span class="hidden lg:inline">{t(appStore.locale, 'commandPalette')}</span>
							<kbd class="hidden font-mono text-[10px] text-secondary sm:inline">Ctrl+K</kbd>
						</button>

						<div class="hidden h-4 w-px bg-separator sm:block" aria-hidden="true"></div>

						<WindowControls />
					</div>
				</header>

				<div class="desktop-shell-content flex min-h-0 flex-1 overflow-hidden">
					<div class="flex min-h-0 flex-1 overflow-hidden">
						<aside
							aria-label="Primary sidebar"
							inert={!appStore.sidebarOpen}
							class="hidden shrink-0 border-r border-separator bg-sidebar transition-all duration-150 ease-out md:block {appStore.sidebarOpen
								? 'w-56'
								: 'w-0 overflow-hidden border-r-0'}"
						>
							<div class="h-full w-56">
								<SidebarNav />
							</div>
						</aside>

						<main class="flex min-w-0 flex-1 flex-col overflow-hidden bg-background">
							{@render children()}
						</main>
					</div>
				</div>
			</div>
		{/if}
	</div>
{/if}

<Drawer.Root bind:open={appStore.mobileDrawerOpen} direction="left">
	<Drawer.Portal>
		<Drawer.Overlay class="fixed inset-0 z-40 bg-foreground/25" />
		<Drawer.Content
			class="fixed inset-y-0 left-0 z-50 flex w-64 flex-col border-r border-separator bg-sidebar outline-none"
		>
			<div class="flex h-11 items-center justify-between border-b border-separator px-3">
				<span class="font-display text-sm font-semibold text-foreground">
					{t(appStore.locale, 'appTitle')}
				</span>
				<button
					type="button"
					onclick={() => appStore.triggerSync()}
					aria-label={t(appStore.locale, 'syncNow')}
					class="flex min-h-11 min-w-11 items-center justify-center rounded-md text-secondary hover:text-foreground"
				>
					<RotateCw class="h-4 w-4" />
				</button>
			</div>
			<div class="min-h-0 flex-1 overflow-y-auto">
				<SidebarNav onNavigate={() => (appStore.mobileDrawerOpen = false)} />
			</div>
		</Drawer.Content>
	</Drawer.Portal>
</Drawer.Root>
