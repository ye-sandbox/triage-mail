<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import {
		guidanceKey,
		recommendedOptions,
		type ProviderKind,
		type RecommendedAction
	} from './recommendation';
	import type { AuthMethod } from './types';

	type Props = {
		kind: ProviderKind;
		method: AuthMethod;
		onAction: (action: RecommendedAction) => void;
		onOffline: () => void;
	};
	let { kind, method, onAction, onOffline }: Props = $props();

	const t = $derived(i18n.t.accountForm);
	const options = $derived(recommendedOptions(kind));
	const BTN =
		'rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors focus-visible:outline-2 focus-visible:outline-accent';
	const ON = 'border-accent bg-selection-soft text-accent';
	const OFF = 'border-separator bg-surface text-foreground hover:bg-selection-soft';
</script>

<div class="space-y-2.5 rounded-md border border-separator bg-field/70 px-3.5 py-3">
	<p class="text-xs leading-relaxed text-secondary">{t[guidanceKey(kind)]}</p>
	<div class="flex flex-wrap items-center gap-2">
		{#each options as opt (opt.label)}
			<button
				type="button"
				aria-pressed={method === opt.activeWhen}
				onclick={() => onAction(opt.action)}
				class="{BTN} {method === opt.activeWhen ? ON : OFF}"
			>
				{t[opt.label]}{opt.recommended ? ` · ${t.recommendedOption}` : ''}
			</button>
		{/each}
		<button type="button" onclick={onOffline} class="{BTN} {OFF}">
			{t.offlineImportOption}
		</button>
	</div>
</div>
