<script lang="ts">
	import { ChevronDown, Info } from 'lucide-svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { GuidePlaybook } from '../i18n/types';
	import type { AuthMethod } from './types';

	interface Props {
		selectedMethod?: AuthMethod;
		selectedPreset?: string;
	}

	let { selectedMethod, selectedPreset }: Props = $props();
	let open = $state(false);
	let activeTab = $state<AuthMethod>('AppPassword');

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
		{ id: 'ProtonBridgeLocal' as const, label: t.proton.title },
		{ id: 'LocalArchive' as const, label: t.localArchive.title }
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
			case 'LocalArchive':
				return {
					summary: t.localArchive.summary,
					requirements: t.localArchive.requirements,
					services: t.localArchive.services,
					note: t.localArchive.note,
					isWarning: false,
					fields: t.localArchive.fields,
					playbooks: t.localArchive.playbooks
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

	const displayedPlaybooks = $derived.by(() => {
		if (selectedPreset) {
			const matching = activeContent.playbooks.filter(
				(p) =>
					p.id === selectedPreset ||
					p.provider.toLowerCase().includes(selectedPreset.toLowerCase())
			);
			if (matching.length > 0) return matching;
		}
		return activeContent.playbooks;
	});
</script>

{#snippet playbookCard(playbook: GuidePlaybook)}
	<div class="space-y-2 rounded-md border border-separator bg-background p-3">
		<h4 class="text-xs font-semibold text-foreground">{playbook.provider}</h4>
		<ol class="list-decimal space-y-1.5 pl-4 text-xs text-secondary">
			{#each playbook.steps as step}
				<li class="leading-relaxed">{step}</li>
			{/each}
		</ol>
	</div>
{/snippet}

<div class="overflow-hidden rounded-lg border border-separator bg-field/60 text-xs">
	<button
		type="button"
		onclick={() => (open = !open)}
		aria-expanded={open}
		aria-controls="account-help-content"
		class="flex w-full items-center justify-between px-4 py-3 text-left font-medium text-foreground transition-colors hover:bg-field"
	>
		<span class="flex items-center gap-2">
			<Info class="h-4 w-4 shrink-0 text-accent" />
			<span>{t.title}</span>
		</span>
		<span class="flex shrink-0 items-center gap-1.5 text-xs text-secondary">
			<span>{open ? t.toggleClose : t.toggleOpen}</span>
			<ChevronDown class="h-3.5 w-3.5 transition-transform duration-150 {open ? 'rotate-180' : ''}" />
		</span>
	</button>

	{#if open}
		<div id="account-help-content" class="space-y-4 border-t border-separator bg-surface p-4">
			<div class="flex flex-wrap gap-1 border-b border-separator pb-2" role="tablist">
				{#each tabs as tab (tab.id)}
					<button
						type="button"
						role="tab"
						aria-selected={activeTab === tab.id}
						onclick={() => (activeTab = tab.id)}
						class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors {activeTab ===
						tab.id
							? 'bg-accent text-accent-foreground'
							: 'text-secondary hover:text-foreground'}"
					>
						{tab.label}
					</button>
				{/each}
			</div>

			<div class="space-y-3 text-xs leading-relaxed">
				<p class="text-foreground">{activeContent.summary}</p>
				<div class="space-y-1.5 rounded-md border border-separator bg-field p-3">
					<p>
						<strong class="text-foreground">{t.requirementsLabel}</strong>
						<span class="text-secondary">{activeContent.requirements}</span>
					</p>
					<p>
						<strong class="text-foreground">{t.servicesLabel}</strong>
						<span class="text-secondary">{activeContent.services}</span>
					</p>
					<p class="pt-1 {activeContent.isWarning ? 'font-medium text-status-warning' : 'text-secondary'}">
						{activeContent.note}
					</p>
				</div>

				<div class="space-y-1.5 rounded-md border border-separator bg-field p-3">
					<h3 class="font-semibold text-foreground">{t.fieldsTitle}</h3>
					<ul class="list-disc space-y-1 pl-4 text-secondary">
						{#each activeContent.fields as field}
							<li>{field}</li>
						{/each}
					</ul>
				</div>

				<div class="space-y-2">
					<h3 class="font-semibold text-foreground">{t.stepsTitle}</h3>
					<div class="space-y-2">
						{#each displayedPlaybooks as playbook}
							{@render playbookCard(playbook)}
						{/each}
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
