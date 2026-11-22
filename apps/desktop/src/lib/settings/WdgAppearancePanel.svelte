<script lang="ts">
	import Monitor from '@lucide/svelte/icons/monitor';
	import Moon from '@lucide/svelte/icons/moon';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Sun from '@lucide/svelte/icons/sun';
	import ZoomIn from '@lucide/svelte/icons/zoom-in';
	import ZoomOut from '@lucide/svelte/icons/zoom-out';
	import {
		FONT_CATALOG,
		LIST_DENSITIES,
		PALETTE_CATALOG,
		TEXT_SCALES,
		UI_ZOOMS
	} from '$lib/appearance/palettes';
	import type { ListDensity, ThemeScheme } from '$lib/appearance/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import { settingsLabels } from './labels';

	const t = $derived(settingsLabels[i18n.locale].appearance);
	const isPt = $derived(i18n.locale === 'pt-BR');

	const schemeOptions: { id: ThemeScheme; icon: typeof Sun }[] = [
		{ id: 'light', icon: Sun },
		{ id: 'dark', icon: Moon },
		{ id: 'system', icon: Monitor }
	];

	function schemeLabel(id: ThemeScheme): string {
		if (id === 'light') return t.schemeLight;
		if (id === 'dark') return t.schemeDark;
		return t.schemeSystem;
	}

	function densityLabel(id: ListDensity): string {
		if (id === 'compact') return t.densityCompact;
		if (id === 'comfortable') return t.densityComfortable;
		return t.densityDefault;
	}
</script>

<section class="flex flex-col gap-5" aria-label={t.heading} aria-labelledby="appearance-heading">
	<div class="flex flex-wrap items-start justify-between gap-3 border-b border-separator pb-3">
		<div>
			<h2 id="appearance-heading" class="font-display text-base font-semibold text-foreground">
				{t.heading}
			</h2>
			<p class="mt-1 text-xs text-secondary">{t.subtitle}</p>
		</div>
		<button
			type="button"
			onclick={() => appStore.resetAppearance()}
			class="flex min-h-[36px] shrink-0 items-center gap-2 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
		>
			<RotateCcw class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
			<span>{t.resetBtn}</span>
		</button>
	</div>

	<div class="divide-y divide-separator rounded-lg border border-separator bg-surface">
		<div class="flex flex-wrap items-center justify-between gap-4 p-4">
			<div class="max-w-lg space-y-1">
				<div class="text-sm font-medium text-foreground">{t.schemeTitle}</div>
				<p class="text-xs text-secondary">{t.schemeDesc}</p>
			</div>
			<div
				class="flex shrink-0 items-center gap-1 rounded-md border border-separator bg-field p-1"
				role="radiogroup"
				aria-label={t.schemeTitle}
			>
				{#each schemeOptions as opt (opt.id)}
					{@const Icon = opt.icon}
					{@const active = appStore.appearance.scheme === opt.id}
					<button
						type="button"
						role="radio"
						aria-checked={active}
						onclick={() => appStore.setScheme(opt.id)}
						class="flex min-h-[36px] items-center gap-2 rounded px-3 py-1 text-xs font-medium transition-colors {active
							? 'bg-surface text-foreground shadow-sm'
							: 'text-secondary hover:text-foreground'}"
					>
						<Icon class="h-3.5 w-3.5" aria-hidden="true" />
						<span>{schemeLabel(opt.id)}</span>
					</button>
				{/each}
			</div>
		</div>

		<div class="space-y-3 p-4">
			<div class="space-y-1">
				<div class="text-sm font-medium text-foreground">{t.paletteTitle}</div>
				<p class="text-xs text-secondary">{t.paletteDesc}</p>
			</div>
			<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4" role="radiogroup" aria-label={t.paletteTitle}>
				{#each PALETTE_CATALOG as pal (pal.id)}
					{@const swatch = appStore.theme === 'dark' ? pal.dark : pal.light}
					{@const active = appStore.appearance.palette === pal.id}
					<button
						type="button"
						role="radio"
						aria-checked={active}
						onclick={() => appStore.setPalette(pal.id)}
						class="flex flex-col gap-2 rounded-md border p-2 text-left transition-colors {active
							? 'border-accent bg-selection-soft'
							: 'border-separator bg-field hover:bg-surface'}"
					>
						<div
							class="flex h-8 w-full items-center justify-between rounded border border-separator px-2"
							style:background-color={swatch.bg}
						>
							<span class="h-3 w-8 rounded-sm" style:background-color={swatch.surface}></span>
							<span class="h-3 w-3 rounded-full" style:background-color={swatch.accent}></span>
						</div>
						<div class="flex items-center justify-between gap-1">
							<span class="truncate text-xs font-medium text-foreground">
								{isPt ? pal.labelPt : pal.labelEn}
							</span>
							{#if pal.accessibility}
								<span class="rounded bg-surface px-1 py-0.5 font-mono text-[10px] text-secondary">
									{t.accessibilityBadge}
								</span>
							{/if}
						</div>
					</button>
				{/each}
			</div>
		</div>

		<div class="space-y-3 p-4">
			<div class="space-y-1">
				<div class="text-sm font-medium text-foreground">{t.fontTitle}</div>
				<p class="text-xs text-secondary">{t.fontDesc}</p>
			</div>
			<div class="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3" role="radiogroup" aria-label={t.fontTitle}>
				{#each FONT_CATALOG as font (font.id)}
					{@const active = appStore.appearance.font === font.id}
					<button
						type="button"
						role="radio"
						aria-checked={active}
						onclick={() => appStore.setFont(font.id)}
						class="flex flex-col gap-1 rounded-md border p-3 text-left transition-colors {active
							? 'border-accent bg-selection-soft'
							: 'border-separator bg-field hover:bg-surface'}"
					>
						<span class="text-xs font-semibold text-foreground" style:font-family={font.cssStack}>
							{isPt ? font.labelPt : font.labelEn}
						</span>
						<span class="truncate text-[11px] text-secondary" style:font-family={font.cssStack}>
							{font.sample}
						</span>
					</button>
				{/each}
			</div>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-4 p-4">
			<div class="max-w-lg space-y-1">
				<div class="text-sm font-medium text-foreground">{t.textScaleTitle}</div>
				<p class="text-xs text-secondary">{t.textScaleDesc}</p>
			</div>
			<div
				class="flex shrink-0 items-center gap-1 rounded-md border border-separator bg-field p-1"
				role="radiogroup"
				aria-label={t.textScaleTitle}
			>
				{#each TEXT_SCALES as scale (scale)}
					{@const active = appStore.appearance.textScale === scale}
					<button
						type="button"
						role="radio"
						aria-checked={active}
						onclick={() => appStore.setTextScale(scale)}
						class="min-h-[36px] rounded px-3 py-1 font-mono text-xs font-medium tabular-nums transition-colors {active
							? 'bg-surface text-foreground shadow-sm'
							: 'text-secondary hover:text-foreground'}"
					>
						{scale}%
					</button>
				{/each}
			</div>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-4 p-4">
			<div class="max-w-lg space-y-1">
				<div class="text-sm font-medium text-foreground">{t.uiZoomTitle}</div>
				<p class="text-xs text-secondary">{t.uiZoomDesc}</p>
			</div>
			<div class="flex shrink-0 items-center gap-2">
				<button
					type="button"
					aria-label={t.zoomOutAria}
					disabled={appStore.appearance.uiZoom <= UI_ZOOMS[0]}
					onclick={() => appStore.zoomOut()}
					class="flex h-9 w-9 items-center justify-center rounded-md border border-separator bg-field text-foreground hover:bg-surface disabled:opacity-40"
				>
					<ZoomOut class="h-4 w-4" aria-hidden="true" />
				</button>
				<span class="min-w-[52px] text-center font-mono text-xs font-medium tabular-nums text-foreground">
					{appStore.appearance.uiZoom}%
				</span>
				<button
					type="button"
					aria-label={t.zoomInAria}
					disabled={appStore.appearance.uiZoom >= UI_ZOOMS[UI_ZOOMS.length - 1]}
					onclick={() => appStore.zoomIn()}
					class="flex h-9 w-9 items-center justify-center rounded-md border border-separator bg-field text-foreground hover:bg-surface disabled:opacity-40"
				>
					<ZoomIn class="h-4 w-4" aria-hidden="true" />
				</button>
				{#if appStore.appearance.uiZoom !== 100}
					<button
						type="button"
						onclick={() => appStore.resetZoom()}
						class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-secondary hover:text-foreground"
					>
						{t.zoomReset}
					</button>
				{/if}
			</div>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-4 p-4">
			<div class="max-w-lg space-y-1">
				<div class="text-sm font-medium text-foreground">{t.densityTitle}</div>
				<p class="text-xs text-secondary">{t.densityDesc}</p>
			</div>
			<div
				class="flex shrink-0 items-center gap-1 rounded-md border border-separator bg-field p-1"
				role="radiogroup"
				aria-label={t.densityTitle}
			>
				{#each LIST_DENSITIES as density (density)}
					{@const active = appStore.appearance.density === density}
					<button
						type="button"
						role="radio"
						aria-checked={active}
						onclick={() => appStore.setDensity(density)}
						class="min-h-[36px] rounded px-3 py-1 text-xs font-medium transition-colors {active
							? 'bg-surface text-foreground shadow-sm'
							: 'text-secondary hover:text-foreground'}"
					>
						{densityLabel(density)}
					</button>
				{/each}
			</div>
		</div>

		<div class="space-y-3 p-4">
			<div class="space-y-1">
				<div class="text-sm font-medium text-foreground">{t.a11yTitle}</div>
				<p class="text-xs text-secondary">{t.a11yDesc}</p>
			</div>
			<div class="flex flex-col gap-3">
				<label class="flex items-center justify-between gap-4 text-xs">
					<span>
						<span class="block font-medium text-foreground">{t.underlineLinksLabel}</span>
						<span class="text-secondary">{t.underlineLinksDesc}</span>
					</span>
					<input
						type="checkbox"
						checked={appStore.appearance.underlineLinks}
						onchange={(e) => appStore.setUnderlineLinks(e.currentTarget.checked)}
						class="h-4 w-4 accent-accent"
					/>
				</label>
				<label class="flex items-center justify-between gap-4 text-xs">
					<span>
						<span class="block font-medium text-foreground">{t.reduceMotionLabel}</span>
						<span class="text-secondary">{t.reduceMotionDesc}</span>
					</span>
					<input
						type="checkbox"
						checked={appStore.appearance.reduceMotion}
						onchange={(e) => appStore.setReduceMotion(e.currentTarget.checked)}
						class="h-4 w-4 accent-accent"
					/>
				</label>
			</div>
		</div>
	</div>
</section>
