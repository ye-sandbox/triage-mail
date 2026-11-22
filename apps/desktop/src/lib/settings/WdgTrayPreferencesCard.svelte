<script lang="ts">
	import { Switch } from 'bits-ui';
	import PanelBottomClose from '@lucide/svelte/icons/panel-bottom-close';
	import Power from '@lucide/svelte/icons/power';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';
	import type { ShellPreferences } from './types';

	interface Props {
		shellPrefs: ShellPreferences;
		savingTray: boolean;
		quitting: boolean;
		trayError: string;
		canSave: boolean;
		canQuit: boolean;
		onUpdateTray: (key: keyof ShellPreferences, value: boolean) => void | Promise<void>;
		onQuit: () => void | Promise<void>;
	}

	let {
		shellPrefs,
		savingTray,
		quitting,
		trayError,
		canSave,
		canQuit,
		onUpdateTray,
		onQuit
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].general);
</script>

<div class="space-y-4 p-4">
	<div class="space-y-1">
		<div class="flex items-center gap-2 text-sm font-medium text-foreground">
			<PanelBottomClose class="h-4 w-4 text-secondary" aria-hidden="true" />
			<span>{t.trayTitle}</span>
		</div>
		<p class="text-xs text-secondary">{t.trayDesc}</p>
	</div>

	<div class="flex flex-col gap-3 rounded-md border border-separator bg-field p-3">
		<div class="flex items-center justify-between gap-4">
			<div class="space-y-1">
				<div class="text-xs font-medium text-foreground">{t.closeToTrayLabel}</div>
				<p class="text-xs text-secondary">{t.closeToTrayDesc}</p>
			</div>
			<Switch.Root
				checked={shellPrefs.close_to_tray}
				disabled={savingTray || quitting || !canSave}
				onCheckedChange={(checked) => void onUpdateTray('close_to_tray', checked)}
				aria-label={t.closeToTrayLabel}
				class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
			>
				<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
			</Switch.Root>
		</div>

		<div class="h-px bg-separator"></div>

		<div class="flex items-center justify-between gap-4">
			<div class="space-y-1">
				<div class="text-xs font-medium text-foreground">{t.minimizeToTrayLabel}</div>
				<p class="text-xs text-secondary">{t.minimizeToTrayDesc}</p>
			</div>
			<Switch.Root
				checked={shellPrefs.minimize_to_tray}
				disabled={savingTray || quitting || !canSave}
				onCheckedChange={(checked) => void onUpdateTray('minimize_to_tray', checked)}
				aria-label={t.minimizeToTrayLabel}
				class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
			>
				<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
			</Switch.Root>
		</div>
	</div>

	{#if trayError}
		<p role="alert" class="text-xs text-status-danger">{trayError}</p>
	{/if}

	<div class="flex flex-wrap items-center justify-between gap-4 pt-1">
		<p class="text-xs text-secondary">{t.quitCompletelyHint}</p>
		<button
			type="button"
			onclick={() => void onQuit()}
			disabled={quitting || !canQuit}
			class="inline-flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs font-medium text-status-danger hover:bg-status-danger/15 disabled:opacity-50"
		>
			<Power class="h-3.5 w-3.5" aria-hidden="true" />
			<span>{t.quitCompletelyBtn}</span>
		</button>
	</div>
</div>
