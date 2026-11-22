<script lang="ts">
	import Cpu from '@lucide/svelte/icons/cpu';
	import Lock from '@lucide/svelte/icons/lock';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';

	interface Props {
		provider: 'Ollama' | 'Byok';
		disabled: boolean;
		onSelect: (next: 'Ollama' | 'Byok') => void;
	}

	let { provider, disabled, onSelect }: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].ai);
</script>

<div class="flex flex-col gap-2">
	<span class="text-xs font-medium text-foreground">{t.providerLabel}</span>
	<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
		<button
			type="button"
			{disabled}
			aria-pressed={provider === 'Ollama'}
			onclick={() => onSelect('Ollama')}
			class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:cursor-not-allowed {provider ===
			'Ollama'
				? 'border-accent bg-selection-soft'
				: 'border-separator bg-field hover:bg-selection-soft/40'}"
		>
			<div class="flex w-full items-center justify-between gap-2">
				<span class="flex items-center gap-2 text-xs font-semibold text-foreground">
					<Cpu class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{t.ollamaTitle}</span>
				</span>
				<span class="font-mono text-[11px] text-status-healthy">{t.ollamaBadge}</span>
			</div>
			<span class="font-mono text-[11px] text-secondary">{t.ollamaDesc}</span>
		</button>

		<button
			type="button"
			{disabled}
			aria-pressed={provider === 'Byok'}
			onclick={() => onSelect('Byok')}
			class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:cursor-not-allowed {provider ===
			'Byok'
				? 'border-accent bg-selection-soft'
				: 'border-separator bg-field hover:bg-selection-soft/40'}"
		>
			<div class="flex w-full items-center justify-between gap-2">
				<span class="flex items-center gap-2 text-xs font-semibold text-foreground">
					<Lock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{t.byokTitle}</span>
				</span>
				<span class="font-mono text-[11px] text-status-warning">{t.byokBadge}</span>
			</div>
			<span class="font-mono text-[11px] text-secondary">{t.byokDesc}</span>
		</button>
	</div>
</div>
