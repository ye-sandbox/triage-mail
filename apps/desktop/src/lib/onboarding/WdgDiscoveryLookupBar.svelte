<script lang="ts">
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import Search from '@lucide/svelte/icons/search';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import { i18n } from '../i18n/index.svelte';
	import type { DiscoveryController } from './discovery-controller.svelte';
	import type { IspdbPreset, TlsProbeResult } from './onboarding.fixtures';
	import { providerPresets, type ProviderPreset } from './presets';

	interface Props {
		discovery: DiscoveryController;
		demoMode: boolean;
		isBridge: boolean;
		showAllPresets: boolean;
		demoIspdb: IspdbPreset | null;
		demoTls: TlsProbeResult | null;
		onLookup: () => void;
		onDetectMx: () => void;
		onProbe: () => void;
		onApplyPreset: (preset: ProviderPreset) => void;
		onToggleAllPresets: () => void;
		onOpenAliasGuide: () => void;
	}

	let {
		discovery,
		demoMode,
		isBridge,
		showAllPresets,
		demoIspdb,
		demoTls,
		onLookup,
		onDetectMx,
		onProbe,
		onApplyPreset,
		onToggleAllPresets,
		onOpenAliasGuide
	}: Props = $props();

	const t = $derived(i18n.t.accountForm);
	const PRIMARY_PRESET_NAMES = new Set(['Gmail', 'Outlook', 'iCloud', 'Proton', 'Other IMAP']);

	function isPresetVisible(name: string): boolean {
		return showAllPresets || PRIMARY_PRESET_NAMES.has(name) || discovery.selectedPreset === name;
	}
</script>

<div class="flex flex-wrap items-center gap-2">
	<button
		type="button"
		disabled={discovery.busy}
		onclick={onLookup}
		class="flex min-h-[34px] shrink-0 items-center justify-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
	>
		<Search class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
		<span>{t.onlineLookup}</span>
	</button>
	{#if !demoMode}
		<button
			type="button"
			title={t.detectMxHelp}
			disabled={discovery.busy}
			onclick={onDetectMx}
			class="flex min-h-[34px] shrink-0 items-center justify-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
		>
			<Search class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
			<span>{t.detectMx}</span>
		</button>
	{/if}
	{#if demoMode || (discovery.settings.host && discovery.settings.port === 993 && !isBridge)}
		<button
			type="button"
			disabled={discovery.busy}
			onclick={onProbe}
			class="flex min-h-[34px] shrink-0 items-center justify-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
		>
			<ShieldCheck class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
			<span>{t.confirmProbe} {discovery.settings.host || '993'}:993</span>
		</button>
	{/if}
</div>

<div class="flex flex-wrap items-center gap-2 pt-1">
	<span class="mr-1 text-[11px] font-medium text-secondary">{t.quickFillLabel}</span>
	{#each providerPresets as preset (preset.name)}
		<button
			type="button"
			hidden={!isPresetVisible(preset.name)}
			aria-pressed={discovery.selectedPreset === preset.name}
			onclick={() => onApplyPreset(preset)}
			class="rounded-md border px-3 py-1 text-[11px] font-medium transition-colors {discovery.selectedPreset ===
			preset.name
				? 'border-accent bg-selection-soft text-accent'
				: 'border-separator bg-field text-secondary hover:text-foreground'}"
		>
			{preset.name}
		</button>
	{/each}
	<button
		type="button"
		aria-expanded={showAllPresets}
		onclick={onToggleAllPresets}
		class="rounded-md border border-dashed border-separator px-3 py-1 text-[11px] font-medium text-secondary transition-colors hover:border-foreground/30 hover:text-foreground"
	>
		{showAllPresets ? t.fewerPresets : t.morePresets}
	</button>
</div>

<div class="flex flex-wrap items-center justify-between gap-2">
	<button
		type="button"
		onclick={onOpenAliasGuide}
		class="text-left text-[11px] font-medium text-accent hover:underline"
	>
		{t.aliasGuideCta}
	</button>
</div>

<p role="status" class="min-h-4 text-xs text-secondary">
	{#if discovery.busy}
		{t.discovering}
	{:else if discovery.status === 'found'}
		{t.providerFound} {discovery.provider?.provider_name} · {discovery.settings.host}:{discovery.settings.port}
	{:else if discovery.status === 'unresolved'}
		{t.discoveryUnresolved}
	{:else if discovery.status === 'invalid'}
		{t.discoveryInvalid}
	{:else if discovery.status === 'unavailable'}
		{t.discoveryUnavailable}
	{:else if discovery.status === 'probeSuccess'}
		{t.probeSuccess}
	{:else if discovery.status === 'probeFailed'}
		{t.probeFailed}
	{:else if demoIspdb}
		{demoIspdb.note}
	{/if}
</p>

{#if demoTls}
	<div class="flex flex-wrap items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 font-mono text-[11px] text-foreground">
		<CheckCircle2 class="h-3.5 w-3.5 shrink-0 text-status-healthy" aria-hidden="true" />
		<span>{demoTls.host}:{demoTls.port}</span>
		<span aria-hidden="true">·</span>
		<span>{demoTls.protocol}</span>
		<span aria-hidden="true">·</span>
		<span class="truncate text-secondary">{demoTls.spki_sha256}</span>
	</div>
{/if}
