<script lang="ts">
	import { i18n } from '$lib/i18n/index.svelte.ts';
	import { createTauriWindowTransport } from './transport';
	import { getWindowLabels, type WindowState, type WindowTransport } from './types';

	type Props = {
		transport?: WindowTransport;
	};

	let { transport = createTauriWindowTransport() }: Props = $props();

	const labels = $derived(getWindowLabels(i18n.locale));
	const available = $derived(transport.isAvailable());

	let windowState = $state<WindowState>({ maximized: false, focused: true });
	let actionError = $state<string | null>(null);

	$effect(() => {
		if (!transport.isAvailable()) return;
		let cancelled = false;

		void transport
			.getState()
			.then((initial) => {
				if (!cancelled) {
					windowState = initial;
				}
			})
			.catch(() => {});

		const unsubscribe = transport.subscribeState((next) => {
			if (!cancelled) {
				windowState = next;
			}
		});

		return () => {
			cancelled = true;
			unsubscribe();
		};
	});

	async function handleDragPointerDown(event: PointerEvent | MouseEvent) {
		if (event.button !== 0) return;
		if (event.target instanceof Element && event.target.closest('button')) return;
		actionError = null;
		try {
			await transport.startDragging();
		} catch {
			actionError = labels.actionError;
		}
	}

	async function handleMinimize() {
		actionError = null;
		try {
			await transport.minimize();
		} catch {
			actionError = labels.actionError;
		}
	}

	async function handleToggleMaximize() {
		actionError = null;
		try {
			const nextMaximized = await transport.toggleMaximize();
			windowState = { ...windowState, maximized: nextMaximized };
		} catch {
			actionError = labels.actionError;
		}
	}

	async function handleClose() {
		actionError = null;
		try {
			await transport.close();
		} catch {
			actionError = labels.actionError;
		}
	}
</script>

{#if available}
	<header
		aria-label={labels.appTitle}
		class="flex h-10 shrink-0 select-none items-center justify-between border-b border-[var(--separator)] bg-[var(--toolbar)] text-[var(--text)]"
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			data-tauri-drag-region
			onpointerdown={handleDragPointerDown}
			ondblclick={handleToggleMaximize}
			class="flex h-full min-w-0 flex-1 items-center gap-2 px-3"
		>
			<span
				data-tauri-drag-region
				class="truncate text-xs font-semibold tracking-tight transition-opacity"
				class:text-[var(--secondary)]={!windowState.focused}
				class:opacity-70={!windowState.focused}
			>
				{labels.appTitle}
			</span>
			{#if actionError}
				<span role="status" aria-live="polite" class="truncate text-xs text-[var(--secondary)]">
					{actionError}
				</span>
			{/if}
		</div>

		<div class="flex h-full shrink-0 items-center">
			<button
				type="button"
				aria-label={labels.minimize}
				title={labels.minimize}
				onclick={handleMinimize}
				class="flex h-10 w-10 items-center justify-center text-[var(--secondary)] transition-colors hover:bg-[var(--field)] hover:text-[var(--text)] focus-visible:z-10"
			>
				<svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
					<path d="M2 6h8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
				</svg>
			</button>

			<button
				type="button"
				aria-label={windowState.maximized ? labels.restore : labels.maximize}
				title={windowState.maximized ? labels.restore : labels.maximize}
				aria-pressed={windowState.maximized}
				onclick={handleToggleMaximize}
				class="flex h-10 w-10 items-center justify-center text-[var(--secondary)] transition-colors hover:bg-[var(--field)] hover:text-[var(--text)] focus-visible:z-10"
			>
				{#if windowState.maximized}
					<svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
						<rect
							x="2.2"
							y="3.8"
							width="6"
							height="6"
							rx="0.8"
							stroke="currentColor"
							stroke-width="1.3"
						/>
						<path
							d="M4.2 3.8V2.8a.8.8 0 0 1 .8-.8h4.2a.8.8 0 0 1 .8.8V7a.8.8 0 0 1-.8.8H8.2"
							stroke="currentColor"
							stroke-width="1.3"
						/>
					</svg>
				{:else}
					<svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
						<rect
							x="2.25"
							y="2.25"
							width="7.5"
							height="7.5"
							rx="1"
							stroke="currentColor"
							stroke-width="1.4"
						/>
					</svg>
				{/if}
			</button>

			<button
				type="button"
				aria-label={labels.close}
				title={labels.close}
				onclick={handleClose}
				class="flex h-10 w-10 items-center justify-center text-[var(--secondary)] transition-colors hover:bg-red-600 hover:text-white active:bg-red-700 focus-visible:z-10"
			>
				<svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
					<path
						d="M2.5 2.5l7 7M9.5 2.5l-7 7"
						stroke="currentColor"
						stroke-width="1.4"
						stroke-linecap="round"
					/>
				</svg>
			</button>
		</div>
	</header>
{/if}
