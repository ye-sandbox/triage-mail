<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { GuidePlaybook } from '../i18n/types';
	import type { AuthMethod } from './types';

	let { selectedMethod }: { selectedMethod?: AuthMethod } = $props();
	let open = $state(false);
	let activeTab = $state<AuthMethod>('AppPassword');

	// Synchronize active tab with user's selected form method if changed
	$effect(() => {
		if (selectedMethod) {
			activeTab = selectedMethod;
		}
	});

	const t = $derived(i18n.t.guide);

	const tabs = $derived([
		{ id: 'AppPassword' as const, label: t.appPassword.title },
		{ id: 'OAuth2PublicPkce' as const, label: t.oauthPublic.title },
		{ id: 'OAuth2ByoCredentials' as const, label: t.oauthByo.title },
		{ id: 'ProtonBridgeLocal' as const, label: t.proton.title }
	]);

	const activeContent = $derived.by(() => {
		switch (activeTab) {
			case 'OAuth2PublicPkce':
				return {
					summary: t.oauthPublic.summary,
					requirements: t.oauthPublic.requirements,
					services: t.oauthPublic.services,
					note: t.oauthPublic.redirectTip,
					isWarning: false,
					fields: t.oauthPublic.fields,
					playbooks: t.oauthPublic.playbooks
				};
			case 'OAuth2ByoCredentials':
				return {
					summary: t.oauthByo.summary,
					requirements: t.oauthByo.requirements,
					services: t.oauthByo.services,
					note: t.oauthByo.gcpTip,
					isWarning: false,
					fields: t.oauthByo.fields,
					playbooks: t.oauthByo.playbooks
				};
			case 'ProtonBridgeLocal':
				return {
					summary: t.proton.summary,
					requirements: t.proton.requirements,
					services: t.proton.services,
					note: t.proton.warning,
					isWarning: true,
					fields: t.proton.fields,
					playbooks: t.proton.playbooks
				};
			default:
				return {
					summary: t.appPassword.summary,
					requirements: t.appPassword.requirements,
					services: t.appPassword.services,
					note: t.appPassword.hostTip,
					isWarning: false,
					fields: t.appPassword.fields,
					playbooks: t.appPassword.playbooks
				};
		}
	});
</script>

{#snippet playbookCard(playbook: GuidePlaybook)}
	<div class="rounded-lg border border-[var(--separator)]/60 bg-[var(--background)] p-3 space-y-2">
		<h4 class="font-semibold text-[var(--text)]">{playbook.provider}</h4>
		<ol class="list-decimal pl-4 space-y-1.5 text-[var(--secondary)]">
			{#each playbook.steps as step}
				<li class="leading-relaxed">{step}</li>
			{/each}
		</ol>
	</div>
{/snippet}

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
				class="h-4 w-4 text-[var(--accent)] shrink-0"
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
		<span class="text-xs text-[var(--secondary)] flex items-center gap-1.5 shrink-0">
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
				{#each tabs as tab}
					<button
						type="button"
						role="tab"
						aria-selected={activeTab === tab.id}
						class="rounded-lg px-2.5 py-1 text-xs font-medium transition-colors {activeTab === tab.id ? 'bg-[var(--accent)] text-white' : 'text-[var(--secondary)] hover:text-[var(--text)]'}"
						onclick={() => (activeTab = tab.id)}
					>
						{tab.label}
					</button>
				{/each}
			</div>

			<!-- Tab contents -->
			<div class="space-y-3 text-xs leading-relaxed">
				<p class="text-[var(--text)]">{activeContent.summary}</p>

				<!-- Requirements & Services -->
				<div class="rounded-lg bg-[var(--background)] p-3 space-y-1.5 border border-[var(--separator)]/50">
					<p><strong class="text-[var(--text)]">{t.requirementsLabel}</strong> <span class="text-[var(--secondary)]">{activeContent.requirements}</span></p>
					<p><strong class="text-[var(--text)]">{t.servicesLabel}</strong> <span class="text-[var(--secondary)]">{activeContent.services}</span></p>
					<p class="{activeContent.isWarning ? 'text-amber-500/90 font-medium' : 'text-[var(--secondary)]'} pt-1">
						{activeContent.note}
					</p>
				</div>

				<!-- Fields to fill -->
				<div class="rounded-lg bg-[var(--background)] p-3 space-y-1.5 border border-[var(--separator)]/50">
					<h3 class="font-semibold text-[var(--text)]">{t.fieldsTitle}</h3>
					<ul class="list-disc pl-4 space-y-1 text-[var(--secondary)]">
						{#each activeContent.fields as field}
							<li>{field}</li>
						{/each}
					</ul>
				</div>

				<!-- Step-by-step Playbooks -->
				<div class="space-y-2">
					<h3 class="font-semibold text-[var(--text)]">{t.stepsTitle}</h3>
					<div class="space-y-2">
						{#each activeContent.playbooks as playbook}
							{@render playbookCard(playbook)}
						{/each}
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
