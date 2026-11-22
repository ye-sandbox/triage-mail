<script lang="ts">
	import Lock from '@lucide/svelte/icons/lock';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';

	const t = $derived(settingsLabels[i18n.locale].privacy);
</script>

<section class="flex flex-col gap-5" aria-label={t.heading} aria-labelledby="privacy-heading">
	<div class="border-b border-separator pb-3">
		<h2 id="privacy-heading" class="font-display text-base font-semibold text-foreground">
			{t.heading}
		</h2>
		<p class="mt-1 text-xs text-secondary">{t.subtitle}</p>
	</div>

	<div class="divide-y divide-separator rounded-lg border border-separator bg-surface">
		<div class="flex flex-wrap items-start justify-between gap-4 p-4">
			<div class="max-w-xl space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<ShieldCheck class="h-4 w-4 text-status-healthy" aria-hidden="true" />
					<span>{t.zeroTelemetryTitle}</span>
				</div>
				<p class="text-xs leading-relaxed text-secondary">{t.zeroTelemetryBody}</p>
			</div>
			<span class="shrink-0 whitespace-nowrap font-mono text-xs font-medium text-status-healthy">
				{t.zeroTelemetryBadge}
			</span>
		</div>

		<div class="flex flex-wrap items-start justify-between gap-4 p-4">
			<div class="max-w-xl space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<Lock class="h-4 w-4 text-status-healthy" aria-hidden="true" />
					<span>{t.remoteImagesTitle}</span>
				</div>
				<p class="text-xs leading-relaxed text-secondary">{t.remoteImagesBody}</p>
			</div>
			<span
				class="shrink-0 whitespace-nowrap rounded border border-status-healthy/40 bg-status-healthy/10 px-2 py-1 font-mono text-xs font-medium text-status-healthy"
			>
				{t.remoteImagesBadge}
			</span>
		</div>

		<div class="grid grid-cols-1 gap-4 p-4 sm:grid-cols-2">
			<div class="space-y-1">
				<div class="text-xs font-medium text-foreground">{t.vaultEncryptionTitle}</div>
				<p class="text-xs text-secondary">{t.vaultEncryptionBody}</p>
			</div>
			<div class="space-y-1">
				<div class="text-xs font-medium text-foreground">{t.htmlSanitizationTitle}</div>
				<p class="text-xs text-secondary">{t.htmlSanitizationBody}</p>
			</div>
			<div class="space-y-1">
				<div class="text-xs font-medium text-foreground">{t.offlineTransportTitle}</div>
				<p class="text-xs text-secondary">{t.offlineTransportBody}</p>
			</div>
			<div class="space-y-1">
				<div class="text-xs font-medium text-foreground">{t.onDemandAttachmentsTitle}</div>
				<p class="text-xs text-secondary">{t.onDemandAttachmentsBody}</p>
			</div>
		</div>
	</div>
</section>
