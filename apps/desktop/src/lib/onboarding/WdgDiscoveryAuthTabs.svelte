<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import { isMethodPrimary, type ProviderKind } from './recommendation';
	import type { AuthMethod } from './types';

	interface Props {
		providerKind: ProviderKind;
		method: AuthMethod;
		showAllMethods: boolean;
		onSelectMethod: (method: AuthMethod) => void;
		onToggleAllMethods: () => void;
	}

	let {
		providerKind,
		method,
		showAllMethods,
		onSelectMethod,
		onToggleAllMethods
	}: Props = $props();

	const t = $derived(i18n.t.accountForm);
	const authTabs = $derived<{ id: AuthMethod; label: string }[]>([
		{ id: 'AppPassword', label: t.methodAppPassword },
		{ id: 'OAuth2PublicPkce', label: t.methodOAuthPublic },
		{ id: 'OAuth2ByoCredentials', label: t.methodOAuthByo },
		{ id: 'ProtonBridgeLocal', label: t.methodProton },
		{ id: 'LocalArchive', label: t.methodLocalArchive }
	]);
</script>

<div class="flex flex-wrap items-center justify-between gap-2">
	<div
		role="tablist"
		aria-label={t.methodLabel}
		hidden={providerKind !== 'custom' && !showAllMethods}
		class="flex flex-1 flex-wrap gap-1 rounded-md border border-separator bg-field p-1"
	>
		{#each authTabs as tab (tab.id)}
			<button
				type="button"
				role="tab"
				hidden={!isMethodPrimary(tab.id, providerKind, method, showAllMethods)}
				aria-selected={method === tab.id}
				onclick={() => onSelectMethod(tab.id)}
				class="min-h-[36px] flex-1 rounded px-3 py-2 text-xs font-medium whitespace-nowrap transition-colors {method ===
				tab.id
					? 'bg-surface text-foreground shadow-xs'
					: 'text-secondary hover:text-foreground'}"
			>
				{tab.label}
			</button>
		{/each}
	</div>
	{#if providerKind !== 'custom'}
		<button
			type="button"
			aria-expanded={showAllMethods}
			onclick={onToggleAllMethods}
			class="text-[11px] font-medium text-secondary whitespace-nowrap hover:text-foreground hover:underline"
		>
			{showAllMethods ? t.fewerSignInMethods : t.allSignInMethods}
		</button>
	{/if}
</div>
