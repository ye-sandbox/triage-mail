<script lang="ts">
	import LanguageSelector from '../../ui/LanguageSelector.svelte';
	import type { SettingsModuleProps } from '../types';

	let {
		settings,
		transport,
		busy = false,
		onReload
	}: SettingsModuleProps = $props();
</script>

<div class="space-y-6" aria-label="General settings">
	<div>
		<h2 class="text-lg font-semibold text-[var(--text)]">General</h2>
		<p class="text-sm text-[var(--secondary)]">Configure interface language and general app preferences.</p>
	</div>

	<div class="space-y-2">
		<span class="block text-sm font-medium text-[var(--text)]">Interface language</span>
		<LanguageSelector />
	</div>

	<div class="rounded-lg border border-[var(--separator)] bg-[var(--surface)] p-4 text-sm text-[var(--secondary)] space-y-2">
		<h3 class="font-medium text-[var(--text)]">About triage-mail</h3>
		<p>100% Local-First, Provider-Agnostic, Open-Source (AGPLv3) Email Aggregator &amp; Triage Engine.</p>
		<p class="text-xs">Zero Electron, zero Python/Node sidecars, zero telemetry.</p>
	</div>
</div>
