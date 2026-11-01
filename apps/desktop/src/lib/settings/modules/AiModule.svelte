<script lang="ts">
	import type { SettingsModuleProps } from '../types';
	import AiSettingsForm from '../AiSettingsForm.svelte';
	import ModelSettings from '../ModelSettings.svelte';

	let {
		settings,
		transport,
		busy = false,
		onReload
	}: SettingsModuleProps = $props();

	function handleChanged() {
		void onReload?.();
	}
</script>

<div class="space-y-6" aria-label="AI settings module">
	<div>
		<h2 class="text-lg font-semibold text-[var(--text)]">AI &amp; Summaries</h2>
		<p class="text-sm text-[var(--secondary)]">
			Configure local or remote AI providers and on-demand model caching.
		</p>
	</div>

	<AiSettingsForm
		initial={settings.ai}
		hasKey={settings.has_api_key}
		{transport}
		changed={handleChanged}
	/>

	<hr class="border-[var(--separator)]" />

	<ModelSettings
		cached={settings.model_cached}
		{transport}
		changed={handleChanged}
	/>
</div>
