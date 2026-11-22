<script lang="ts">
	import Bell from '@lucide/svelte/icons/bell';
	import ShieldAlert from '@lucide/svelte/icons/shield-alert';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels } from './labels';

	interface Props {
		enabledRulesCount: number;
		onOpenConfig: () => void;
	}

	let { enabledRulesCount, onOpenConfig }: Props = $props();
	const t = $derived(labels[i18n.locale]);
</script>

<header class="flex flex-wrap items-start justify-between gap-4 border-b border-separator pb-4">
	<div class="space-y-1">
		<h1 class="font-display text-lg font-semibold text-foreground">{t.title}</h1>
		<p class="text-xs text-secondary">{t.subtitle}</p>
	</div>

	<button
		type="button"
		onclick={onOpenConfig}
		class="flex min-h-[44px] shrink-0 items-center gap-2 rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium whitespace-nowrap text-foreground transition-colors hover:bg-selection-soft focus-visible:outline-2 focus-visible:outline-accent active:scale-[0.98] md:min-h-[36px]"
	>
		<Bell class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
		<span>{t.configureSchedule}</span>
		<span class="font-mono text-[11px] tabular-nums text-secondary">
			({enabledRulesCount > 0 ? t.activeRulesCount(enabledRulesCount) : t.paused})
		</span>
	</button>
</header>

<div
	role="note"
	aria-label={t.disclaimerTitle}
	class="flex items-start gap-3 rounded-lg border border-status-warning/35 bg-status-warning/10 px-4 py-3 text-xs text-foreground"
>
	<ShieldAlert class="mt-1 h-4 w-4 shrink-0 text-status-warning" aria-hidden="true" />
	<p class="leading-relaxed">
		<strong class="font-semibold">{t.disclaimerTitle}</strong>
		{t.disclaimer}
	</p>
</div>
