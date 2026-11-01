<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { AuthMethod } from './types';

	let { selectedMethod }: { selectedMethod?: AuthMethod } = $props();
	let open = $state(false);
	let activeTab = $state<'AppPassword' | 'OAuth2PublicPkce' | 'OAuth2ByoCredentials' | 'ProtonBridgeLocal'>('AppPassword');

	// Synchronize active tab with user's selected form method if changed
	$effect(() => {
		if (selectedMethod) {
			activeTab = selectedMethod;
		}
	});

	const t = $derived(i18n.t.guide);
</script>

<div class="rounded-xl border border-[var(--separator)] bg-[var(--surface)]/70 text-sm overflow-hidden backdrop-blur-xs">
	<button
		type="button"
		class="flex w-full items-center justify-between px-4 py-3 text-left font-medium text-[var(--text)] transition-colors hover:bg-[var(--separator)]/30 active:scale-[0.99]"
		onclick={() => (open = !open)}
		aria-expanded={open}
		aria-controls="account-help-content"
	>
		<span class="flex items-center gap-2">
			<svg
				class="h-4 w-4 text-[var(--accent)]"
				fill="none"
				viewBox="0 0 24 24"
				stroke="currentColor"
				stroke-width="2"
				aria-hidden="true"
			>
				<circle cx="12" cy="12" r="10" />
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 16v-4m0-4h.01" />
			</svg>
			<span>{t.title}</span>
		</span>
		<span class="text-xs text-[var(--secondary)] flex items-center gap-1.5">
			<span>{open ? t.toggleClose : t.toggleOpen}</span>
			<svg
				class="h-3.5 w-3.5 transition-transform duration-200 {open ? 'rotate-180' : ''}"
				fill="none"
				viewBox="0 0 24 24"
				stroke="currentColor"
				stroke-width="2.5"
				aria-hidden="true"
			>
				<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
			</svg>
		</span>
	</button>

	{#if open}
		<div id="account-help-content" class="border-t border-[var(--separator)] p-4 space-y-4">
			<!-- Tab selector for methods -->
			<div class="flex flex-wrap gap-1 border-b border-[var(--separator)] pb-2" role="tablist">
				<button
					type="button"
					role="tab"
					aria-selected={activeTab === 'AppPassword'}
					class="rounded-lg px-2.5 py-1 text-xs font-medium transition-colors {activeTab === 'AppPassword' ? 'bg-[var(--accent)] text-white' : 'text-[var(--secondary)] hover:text-[var(--text)]'}"
					onclick={() => (activeTab = 'AppPassword')}
				>
					{t.appPassword.title}
				</button>
				<button
					type="button"
					role="tab"
					aria-selected={activeTab === 'OAuth2PublicPkce'}
					class="rounded-lg px-2.5 py-1 text-xs font-medium transition-colors {activeTab === 'OAuth2PublicPkce' ? 'bg-[var(--accent)] text-white' : 'text-[var(--secondary)] hover:text-[var(--text)]'}"
					onclick={() => (activeTab = 'OAuth2PublicPkce')}
				>
					{t.oauthPublic.title}
				</button>
				<button
					type="button"
					role="tab"
					aria-selected={activeTab === 'OAuth2ByoCredentials'}
					class="rounded-lg px-2.5 py-1 text-xs font-medium transition-colors {activeTab === 'OAuth2ByoCredentials' ? 'bg-[var(--accent)] text-white' : 'text-[var(--secondary)] hover:text-[var(--text)]'}"
					onclick={() => (activeTab = 'OAuth2ByoCredentials')}
				>
					{t.oauthByo.title}
				</button>
				<button
					type="button"
					role="tab"
					aria-selected={activeTab === 'ProtonBridgeLocal'}
					class="rounded-lg px-2.5 py-1 text-xs font-medium transition-colors {activeTab === 'ProtonBridgeLocal' ? 'bg-[var(--accent)] text-white' : 'text-[var(--secondary)] hover:text-[var(--text)]'}"
					onclick={() => (activeTab = 'ProtonBridgeLocal')}
				>
					{t.proton.title}
				</button>
			</div>

			<!-- Tab contents -->
			<div class="space-y-2 text-xs leading-relaxed">
				{#if activeTab === 'AppPassword'}
					<p class="text-[var(--text)]">{t.appPassword.summary}</p>
					<div class="rounded-lg bg-[var(--background)] p-3 space-y-1.5 border border-[var(--separator)]/50">
						<p><strong class="text-[var(--text)]">{t.requirementsLabel}</strong> <span class="text-[var(--secondary)]">{t.appPassword.requirements}</span></p>
						<p><strong class="text-[var(--text)]">{t.servicesLabel}</strong> <span class="text-[var(--secondary)]">{t.appPassword.services}</span></p>
						<p class="text-[var(--secondary)] text-[11px] pt-1">{t.appPassword.hostTip}</p>
					</div>
				{:else if activeTab === 'OAuth2PublicPkce'}
					<p class="text-[var(--text)]">{t.oauthPublic.summary}</p>
					<div class="rounded-lg bg-[var(--background)] p-3 space-y-1.5 border border-[var(--separator)]/50">
						<p><strong class="text-[var(--text)]">{t.requirementsLabel}</strong> <span class="text-[var(--secondary)]">{t.oauthPublic.requirements}</span></p>
						<p><strong class="text-[var(--text)]">{t.servicesLabel}</strong> <span class="text-[var(--secondary)]">{t.oauthPublic.services}</span></p>
						<p class="text-[var(--secondary)] text-[11px] pt-1">{t.oauthPublic.redirectTip}</p>
					</div>
				{:else if activeTab === 'OAuth2ByoCredentials'}
					<p class="text-[var(--text)]">{t.oauthByo.summary}</p>
					<div class="rounded-lg bg-[var(--background)] p-3 space-y-1.5 border border-[var(--separator)]/50">
						<p><strong class="text-[var(--text)]">{t.requirementsLabel}</strong> <span class="text-[var(--secondary)]">{t.oauthByo.requirements}</span></p>
						<p><strong class="text-[var(--text)]">{t.servicesLabel}</strong> <span class="text-[var(--secondary)]">{t.oauthByo.services}</span></p>
						<p class="text-[var(--secondary)] text-[11px] pt-1">{t.oauthByo.gcpTip}</p>
					</div>
				{:else if activeTab === 'ProtonBridgeLocal'}
					<p class="text-[var(--text)]">{t.proton.summary}</p>
					<div class="rounded-lg bg-[var(--background)] p-3 space-y-1.5 border border-[var(--separator)]/50">
						<p><strong class="text-[var(--text)]">{t.requirementsLabel}</strong> <span class="text-[var(--secondary)]">{t.proton.requirements}</span></p>
						<p><strong class="text-[var(--text)]">{t.servicesLabel}</strong> <span class="text-[var(--secondary)]">{t.proton.services}</span></p>
						<p class="text-amber-500/90 font-medium text-[11px] pt-1">{t.proton.warning}</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
