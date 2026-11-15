<script lang="ts">
	import { Globe, Lock, Monitor, ShieldCheck } from 'lucide-svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SupportedLocale } from '$lib/i18n/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { ThemeMode } from '$lib/types';
	import { settingsLabels } from './labels';

	interface Props {
		section: 'general' | 'privacy';
	}

	let { section }: Props = $props();

	const t = $derived(settingsLabels[i18n.locale]);

	const osScheme = $derived.by((): ThemeMode => {
		if (typeof window === 'undefined' || !window.matchMedia) return 'light';
		return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
	});

	function selectLocale(next: SupportedLocale) {
		appStore.setLocale(next);
	}

	function syncWithOsScheme() {
		appStore.setTheme(osScheme);
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
