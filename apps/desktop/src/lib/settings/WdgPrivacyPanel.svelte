<script lang="ts">
	import { onDestroy } from 'svelte';
	import Globe from '@lucide/svelte/icons/globe';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SupportedLocale } from '$lib/i18n/types';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { appStore } from '$lib/stores/app.svelte';
	import type { ThemeMode } from '$lib/types';
	import { settingsLabels } from './labels';
	import { createDemoSettingsTransport } from './settings.fixtures';
	import { nativeSettingsTransport } from './transport';
	import type { SettingsTransport, ShellPreferences } from './types';
	import WdgPrivacyGuaranteesSection from './WdgPrivacyGuaranteesSection.svelte';
	import WdgTrayPreferencesCard from './WdgTrayPreferencesCard.svelte';

	interface Props {
		section: 'general' | 'privacy';
		transport?: SettingsTransport;
	}

	const defaultDemoTransport = createDemoSettingsTransport();

	let {
		section,
		transport = nativeAvailable() ? nativeSettingsTransport : defaultDemoTransport
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale]);

	const osScheme = $derived.by((): ThemeMode => {
		if (typeof window === 'undefined' || !window.matchMedia) return 'light';
		return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
	});

	let shellPrefs = $state<ShellPreferences>({
		close_to_tray: appStore.shellPreferences.close_to_tray,
		minimize_to_tray: appStore.shellPreferences.minimize_to_tray
	});
	let savingTray = $state(false);
	let quitting = $state(false);
	let trayError = $state('');
	let disposed = false;

	onDestroy(() => {
		disposed = true;
	});

	$effect(() => {
		if (section !== 'general' || !transport.shellPreferences) return;
		let active = true;
		const loadPrefs = transport.shellPreferences;
		void loadPrefs()
			.then((prefs) => {
				if (active && !disposed) {
					shellPrefs = prefs;
					appStore.setShellPreferences(prefs);
					trayError = '';
				}
			})
			.catch(() => {
				// Best-effort hydration: retain current preferences if vault is unavailable.
			});
		return () => {
			active = false;
		};
	});

	function selectLocale(next: SupportedLocale) {
		appStore.setLocale(next);
	}

	async function updateTrayPreference(key: keyof ShellPreferences, value: boolean) {
		if (savingTray || quitting || !transport.saveShellPreferences) return;
		const previous = { ...shellPrefs };
		const next: ShellPreferences = { ...shellPrefs, [key]: value };
		shellPrefs = next;
		appStore.setShellPreferences(next);
		savingTray = true;
		trayError = '';
		try {
			const saved = await transport.saveShellPreferences(next);
			if (!disposed) {
				shellPrefs = saved;
				appStore.setShellPreferences(saved);
			}
		} catch {
			if (!disposed) {
				shellPrefs = previous;
				appStore.setShellPreferences(previous);
				trayError = t.general.traySaveError;
				toast.error(t.general.traySaveError);
			}
		} finally {
			if (!disposed) savingTray = false;
		}
	}

	async function handleQuitCompletely() {
		if (quitting || !transport.quitApp) return;
		quitting = true;
		trayError = '';
		try {
			await transport.quitApp();
		} catch {
			if (!disposed) {
				trayError = t.general.traySaveError;
				toast.error(t.general.traySaveError);
			}
		} finally {
			if (!disposed) quitting = false;
		}
	}
</script>

{#if section === 'general'}
	<section class="flex flex-col gap-5" aria-label={t.general.heading} aria-labelledby="general-heading">
		<div class="border-b border-separator pb-3">
			<h2 id="general-heading" class="font-display text-base font-semibold text-foreground">
				{t.general.heading}
			</h2>
			<p class="mt-1 text-xs text-secondary">{t.general.subtitle}</p>
		</div>

		<div class="divide-y divide-separator rounded-lg border border-separator bg-surface">
			<div class="flex flex-wrap items-center justify-between gap-4 p-4">
				<div class="max-w-lg space-y-1">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<Globe class="h-4 w-4 text-secondary" aria-hidden="true" />
						<span>{t.general.languageTitle}</span>
					</div>
					<p class="text-xs text-secondary">{t.general.languageDesc}</p>
				</div>

				<div
					class="flex shrink-0 items-center gap-1 rounded-md border border-separator bg-field p-1"
					role="radiogroup"
					aria-label={t.general.languageAria}
				>
					<button
						type="button"
						role="radio"
						aria-checked={i18n.locale === 'en'}
						aria-label={t.general.english}
						onclick={() => selectLocale('en')}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded px-3 py-2 text-xs font-medium transition-colors {i18n.locale ===
						'en'
							? 'bg-surface text-foreground shadow-sm'
							: 'text-secondary hover:text-foreground'}"
					>
						{t.general.english}
					</button>
					<button
						type="button"
						role="radio"
						aria-checked={i18n.locale === 'pt-BR'}
						aria-label={t.general.portuguese}
						onclick={() => selectLocale('pt-BR')}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded px-3 py-2 text-xs font-medium transition-colors {i18n.locale ===
						'pt-BR'
							? 'bg-surface text-foreground shadow-sm'
							: 'text-secondary hover:text-foreground'}"
					>
						{t.general.portuguese}
					</button>
				</div>
			</div>

			<div class="flex flex-wrap items-center justify-between gap-4 p-4">
				<div class="max-w-lg space-y-1">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<Monitor class="h-4 w-4 text-secondary" aria-hidden="true" />
						<span>{t.general.themeTitle}</span>
					</div>
					<p class="text-xs leading-relaxed text-secondary">
						{t.general.themeDescPrefix}
						{t.general.detectedScheme}: <span class="font-mono text-foreground">{osScheme}</span>
						<span aria-hidden="true">·</span>
						{t.general.activeCanvas}: <span class="font-mono text-foreground">{appStore.theme}</span>
					</p>
				</div>

				<div class="flex shrink-0 items-center gap-2">
					<button
						type="button"
						onclick={() => appStore.setTheme(osScheme)}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-2 text-xs font-medium text-foreground hover:bg-selection-soft"
					>
						{t.general.matchOs(osScheme)}
					</button>
					<button
						type="button"
						onclick={() => appStore.toggleTheme()}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground"
					>
						{t.general.previewTheme(appStore.theme === 'dark' ? t.general.light : t.general.dark)}
					</button>
				</div>
			</div>

			<WdgTrayPreferencesCard
				{shellPrefs}
				{savingTray}
				{quitting}
				{trayError}
				canSave={Boolean(transport.saveShellPreferences)}
				canQuit={Boolean(transport.quitApp)}
				onUpdateTray={updateTrayPreference}
				onQuit={handleQuitCompletely}
			/>

			<div class="space-y-2 p-4">
				<h3 class="text-sm font-medium text-foreground">{t.general.aboutTitle}</h3>
				<p class="text-xs text-secondary">{t.general.aboutBody}</p>
				<p class="font-mono text-[11px] text-secondary">{t.general.aboutStack}</p>
			</div>
		</div>
	</section>
{:else}
	<WdgPrivacyGuaranteesSection />
{/if}
