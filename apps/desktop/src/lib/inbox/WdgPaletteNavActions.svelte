<script lang="ts">
	import { Command } from 'bits-ui';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import Globe from '@lucide/svelte/icons/globe';
	import Inbox from '@lucide/svelte/icons/inbox';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Moon from '@lucide/svelte/icons/moon';
	import PanelLeft from '@lucide/svelte/icons/panel-left';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Settings from '@lucide/svelte/icons/settings';
	import Sun from '@lucide/svelte/icons/sun';
	import Workflow from '@lucide/svelte/icons/workflow';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';

	type Props = {
		onNavigate: (path: string) => void | Promise<void>;
		onClose: () => void;
	};

	let { onNavigate, onClose }: Props = $props();
</script>

<Command.Group>
	<Command.GroupHeading class="px-2 py-2 font-display text-[11px] font-semibold text-secondary">
		Navigation & Preferences
	</Command.GroupHeading>
	<Command.GroupItems>
		<Command.Item
			value="go to unified inbox"
			onSelect={() => void onNavigate('/')}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<Inbox class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t(appStore.locale, 'navInbox')}</span>
		</Command.Item>

		<Command.Item
			value="go to due dates boletos reminders"
			onSelect={() => void onNavigate('/due-dates')}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<CalendarClock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t(appStore.locale, 'navDueDates')}</span>
		</Command.Item>

		<Command.Item
			value="go to automation rules alerts sender governance"
			onSelect={() => void onNavigate('/automation')}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<Workflow class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t(appStore.locale, 'navAutomation')}</span>
		</Command.Item>

		<Command.Item
			value="go to onboarding accounts vault"
			onSelect={() => void onNavigate('/onboarding')}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<KeyRound class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t(appStore.locale, 'navOnboarding')}</span>
		</Command.Item>

		<Command.Item
			value="go to settings privacy ai backup"
			onSelect={() => void onNavigate('/settings')}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<Settings class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t(appStore.locale, 'navSettings')}</span>
		</Command.Item>

		<Command.Item
			value="toggle light dark theme appearance"
			onSelect={() => {
				appStore.toggleTheme();
				onClose();
			}}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			{#if appStore.theme === 'dark'}
				<Sun class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>Switch to Light Theme</span>
			{:else}
				<Moon class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>Switch to Dark Theme</span>
			{/if}
		</Command.Item>

		<Command.Item
			value="switch language locale english portuguese"
			onSelect={() => {
				appStore.setLocale(appStore.locale === 'en' ? 'pt-BR' : 'en');
				onClose();
			}}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<Globe class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>
				{appStore.locale === 'en'
					? 'Switch Language to Português (BR)'
					: 'Switch Language to English'}
			</span>
		</Command.Item>

		<Command.Item
			value="toggle sidebar"
			onSelect={() => {
				appStore.toggleSidebar();
				onClose();
			}}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<PanelLeft class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>Toggle Sidebar</span>
		</Command.Item>

		<Command.Item
			value="sync mailboxes refresh"
			onSelect={() => {
				appStore.triggerSync();
				onClose();
			}}
			class="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-foreground outline-none data-[selected]:bg-selection-soft"
		>
			<RefreshCw class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t(appStore.locale, 'syncNow')}</span>
		</Command.Item>
	</Command.GroupItems>
</Command.Group>
