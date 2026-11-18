<script lang="ts">
	import { onDestroy } from 'svelte';
	import { Switch } from 'bits-ui';
	import { Globe, Lock, Monitor, PanelBottomClose, Power, ShieldCheck } from 'lucide-svelte';
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

	function syncWithOsScheme() {
		appStore.setTheme(osScheme);
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
			<p class="mt-0.5 text-xs text-secondary">{t.general.subtitle}</p>
		</div>

		<div class="divide-y divide-separator rounded-lg border border-separator bg-surface">
			<div class="flex flex-wrap items-center justify-between gap-4 p-4">
				<div class="max-w-lg space-y-1">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<Globe class="h-4 w-4 text-secondary" />
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
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded px-3 py-1.5 text-xs font-medium transition-colors {i18n.locale ===
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
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded px-3 py-1.5 text-xs font-medium transition-colors {i18n.locale ===
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
						<Monitor class="h-4 w-4 text-secondary" />
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
						onclick={syncWithOsScheme}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft"
					>
						{t.general.matchOs(osScheme)}
					</button>
					<button
						type="button"
						onclick={() => appStore.toggleTheme()}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground"
					>
						{t.general.previewTheme(appStore.theme === 'dark' ? t.general.light : t.general.dark)}
					</button>
				</div>
			</div>

			<div class="space-y-4 p-4">
				<div class="space-y-1">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<PanelBottomClose class="h-4 w-4 text-secondary" />
						<span>{t.general.trayTitle}</span>
					</div>
					<p class="text-xs text-secondary">{t.general.trayDesc}</p>
				</div>

				<div class="flex flex-col gap-3 rounded-md border border-separator bg-field p-3">
					<div class="flex items-center justify-between gap-4">
						<div class="space-y-0.5">
							<div class="text-xs font-medium text-foreground">{t.general.closeToTrayLabel}</div>
							<p class="text-xs text-secondary">{t.general.closeToTrayDesc}</p>
						</div>
						<Switch.Root
							checked={shellPrefs.close_to_tray}
							disabled={savingTray || quitting || !transport.saveShellPreferences}
							onCheckedChange={(checked) => void updateTrayPreference('close_to_tray', checked)}
							aria-label={t.general.closeToTrayLabel}
							class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
						>
							<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
						</Switch.Root>
					</div>

					<div class="h-px bg-separator"></div>

					<div class="flex items-center justify-between gap-4">
						<div class="space-y-0.5">
							<div class="text-xs font-medium text-foreground">{t.general.minimizeToTrayLabel}</div>
							<p class="text-xs text-secondary">{t.general.minimizeToTrayDesc}</p>
						</div>
						<Switch.Root
							checked={shellPrefs.minimize_to_tray}
							disabled={savingTray || quitting || !transport.saveShellPreferences}
							onCheckedChange={(checked) => void updateTrayPreference('minimize_to_tray', checked)}
							aria-label={t.general.minimizeToTrayLabel}
							class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
						>
							<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
						</Switch.Root>
					</div>
				</div>

				{#if trayError}
					<p role="alert" class="text-xs text-status-danger">{trayError}</p>
				{/if}

				<div class="flex flex-wrap items-center justify-between gap-4 pt-1">
					<p class="text-xs text-secondary">{t.general.quitCompletelyHint}</p>
					<button
						type="button"
						onclick={() => void handleQuitCompletely()}
						disabled={quitting || !transport.quitApp}
						class="inline-flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-1.5 text-xs font-medium text-status-danger hover:bg-status-danger/15 disabled:opacity-50"
					>
						<Power class="h-3.5 w-3.5" />
						<span>{t.general.quitCompletelyBtn}</span>
					</button>
				</div>
			</div>

			<div class="space-y-1.5 p-4">
				<h3 class="text-sm font-medium text-foreground">{t.general.aboutTitle}</h3>
				<p class="text-xs text-secondary">{t.general.aboutBody}</p>
				<p class="font-mono text-[11px] text-secondary">{t.general.aboutStack}</p>
			</div>
		</div>
	</section>
{:else}
	<section class="flex flex-col gap-5" aria-label={t.privacy.heading} aria-labelledby="privacy-heading">
		<div class="border-b border-separator pb-3">
			<h2 id="privacy-heading" class="font-display text-base font-semibold text-foreground">
				{t.privacy.heading}
			</h2>
			<p class="mt-0.5 text-xs text-secondary">{t.privacy.subtitle}</p>
		</div>

		<div class="divide-y divide-separator rounded-lg border border-separator bg-surface">
			<div class="flex flex-wrap items-start justify-between gap-4 p-4">
				<div class="max-w-xl space-y-1">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<ShieldCheck class="h-4 w-4 text-status-healthy" />
						<span>{t.privacy.zeroTelemetryTitle}</span>
					</div>
					<p class="text-xs leading-relaxed text-secondary">{t.privacy.zeroTelemetryBody}</p>
				</div>
				<span class="shrink-0 whitespace-nowrap font-mono text-xs font-medium text-status-healthy">
					{t.privacy.zeroTelemetryBadge}
				</span>
			</div>

			<div class="flex flex-wrap items-start justify-between gap-4 p-4">
				<div class="max-w-xl space-y-1">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<Lock class="h-4 w-4 text-status-healthy" />
						<span>{t.privacy.remoteImagesTitle}</span>
					</div>
					<p class="text-xs leading-relaxed text-secondary">{t.privacy.remoteImagesBody}</p>
				</div>
				<span
					class="shrink-0 whitespace-nowrap rounded border border-status-healthy/40 bg-status-healthy/10 px-2 py-0.5 font-mono text-xs font-medium text-status-healthy"
				>
					{t.privacy.remoteImagesBadge}
				</span>
			</div>

			<div class="grid grid-cols-1 gap-4 p-4 sm:grid-cols-2">
				<div class="space-y-1">
					<div class="text-xs font-medium text-foreground">{t.privacy.vaultEncryptionTitle}</div>
					<p class="text-xs text-secondary">{t.privacy.vaultEncryptionBody}</p>
				</div>
				<div class="space-y-1">
					<div class="text-xs font-medium text-foreground">{t.privacy.htmlSanitizationTitle}</div>
					<p class="text-xs text-secondary">{t.privacy.htmlSanitizationBody}</p>
				</div>
				<div class="space-y-1">
					<div class="text-xs font-medium text-foreground">{t.privacy.offlineTransportTitle}</div>
					<p class="text-xs text-secondary">{t.privacy.offlineTransportBody}</p>
				</div>
				<div class="space-y-1">
					<div class="text-xs font-medium text-foreground">{t.privacy.onDemandAttachmentsTitle}</div>
					<p class="text-xs text-secondary">{t.privacy.onDemandAttachmentsBody}</p>
				</div>
			</div>
		</div>
	</section>
{/if}
