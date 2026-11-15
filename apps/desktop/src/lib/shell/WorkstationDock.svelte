<script lang="ts">
	import { Mail, Maximize2 } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import { windowController } from './window.svelte';

	type Props = {
		variant: 'minimized' | 'closed';
	};

	let { variant }: Props = $props();
</script>

{#if variant === 'closed'}
	<div
		class="workstation-backdrop flex h-screen w-screen items-center justify-center bg-sidebar/90 p-4 text-foreground"
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-4 rounded-xl border border-separator bg-surface p-6 text-center shadow-xl"
		>
			<div
				class="flex h-11 w-11 items-center justify-center rounded-lg border border-separator bg-field text-accent"
			>
				<Mail class="h-5 w-5" />
			</div>
			<div class="flex flex-col gap-1">
				<h1 class="font-display text-base font-semibold text-foreground">
					{t(appStore.locale, 'windowClosedTitle')}
				</h1>
				<p class="text-xs leading-relaxed text-secondary">
					{t(appStore.locale, 'windowClosedBody')}
				</p>
			</div>
			<button
				type="button"
				onclick={() => windowController.reopen()}
				class="flex min-h-10 items-center gap-2 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground transition-transform duration-100 ease-out hover:opacity-95 active:scale-[0.98]"
			>
				<Maximize2 class="h-3.5 w-3.5" />
				<span>{t(appStore.locale, 'windowReopen')}</span>
			</button>
		</div>
	</div>
{:else}
	<div class="flex h-full w-full items-end justify-center p-4">
		<div
			class="flex items-center gap-3 rounded-xl border border-separator bg-surface px-3.5 py-2 shadow-xl"
		>
			<div class="flex items-center gap-2">
				<span class="flex h-7 w-7 items-center justify-center rounded-md bg-field text-accent">
					<Mail class="h-3.5 w-3.5" />
				</span>
				<div class="flex flex-col">
					<span class="font-display text-xs font-semibold text-foreground">
						{t(appStore.locale, 'appTitle')}
					</span>
					<span class="text-[11px] text-secondary">
						{t(appStore.locale, 'windowMinimizedHint')}
					</span>
				</div>
			</div>
			<div class="h-5 w-px bg-separator"></div>
			<button
				type="button"
				onclick={() => windowController.restoreFromDock()}
				class="flex min-h-9 items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground transition-transform duration-100 ease-out hover:opacity-95 active:scale-[0.98]"
			>
				<Maximize2 class="h-3.5 w-3.5" />
				<span>{t(appStore.locale, 'windowRestoreAction')}</span>
			</button>
		</div>
	</div>
{/if}
