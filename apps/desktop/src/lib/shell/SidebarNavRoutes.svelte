<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { CalendarClock, KeyRound, Settings, Workflow } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';

	type Props = {
		onNavigate?: () => void;
	};

	let { onNavigate }: Props = $props();

	const pathname = $derived(page.url.pathname);

	async function navigateRoute(path: string) {
		try {
			await goto(path);
		} catch {
			// Unit tests mount SidebarNavRoutes without booting SvelteKit's client router.
		}
		onNavigate?.();
	}
</script>

<section class="flex flex-col gap-1 border-t border-separator pt-3">
	<button
		type="button"
		onclick={() => void navigateRoute('/due-dates')}
		class="flex min-h-11 w-full items-center justify-between rounded-md px-2 py-1.5 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {pathname ===
		'/due-dates'
			? 'bg-selection font-medium text-foreground'
			: 'text-secondary hover:bg-field hover:text-foreground'}"
	>
		<span class="flex items-center gap-2 truncate">
			<CalendarClock class="h-3.5 w-3.5 shrink-0" />
			<span class="truncate">{t(appStore.locale, 'navDueDates')}</span>
		</span>
		<span
			class="rounded-full bg-field px-1.5 py-0.5 font-mono text-[10px] font-medium tabular-nums text-status-warning"
			title="Overdue + Due Today"
		>
			{appStore.overdueAndDueTodayCount}
		</span>
	</button>
	<button
		type="button"
		onclick={() => void navigateRoute('/automation')}
		class="flex min-h-11 w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {pathname ===
		'/automation'
			? 'bg-selection font-medium text-foreground'
			: 'text-secondary hover:bg-field hover:text-foreground'}"
	>
		<Workflow class="h-3.5 w-3.5 shrink-0" />
		<span class="truncate">{t(appStore.locale, 'navAutomation')}</span>
	</button>
	<button
		type="button"
		onclick={() => void navigateRoute('/onboarding')}
		class="flex min-h-11 w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {pathname ===
		'/onboarding'
			? 'bg-selection font-medium text-foreground'
			: 'text-secondary hover:bg-field hover:text-foreground'}"
	>
		<KeyRound class="h-3.5 w-3.5 shrink-0" />
		<span class="truncate">{t(appStore.locale, 'navOnboarding')}</span>
	</button>
	<button
		type="button"
		onclick={() => void navigateRoute('/settings')}
		class="flex min-h-11 w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-transform duration-100 ease-out active:scale-[0.98] md:min-h-0 {pathname ===
		'/settings'
			? 'bg-selection font-medium text-foreground'
			: 'text-secondary hover:bg-field hover:text-foreground'}"
	>
		<Settings class="h-3.5 w-3.5 shrink-0" />
		<span class="truncate">{t(appStore.locale, 'navSettings')}</span>
	</button>
</section>
