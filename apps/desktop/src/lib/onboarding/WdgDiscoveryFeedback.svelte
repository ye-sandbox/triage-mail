<script lang="ts">
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import X from '@lucide/svelte/icons/x';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountController } from './controller.svelte';
	import type { AuthMethod } from './types';
	import WdgAccountHelpGuide from './WdgAccountHelpGuide.svelte';
	import WdgProviderDiagnosticHint from './WdgProviderDiagnosticHint.svelte';

	interface Props {
		controller: AccountController;
		host: string;
		method: AuthMethod;
		selectedPreset: string;
		routing: boolean;
		guideOpen: boolean;
		guideTab: AuthMethod | 'EmailRouting';
	}

	let {
		controller,
		host,
		method,
		selectedPreset,
		routing,
		guideOpen = $bindable(),
		guideTab = $bindable()
	}: Props = $props();

	const t = $derived(i18n.t.accountForm);
</script>

{#if controller.authorizing}
	<div
		role="status"
		class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-separator bg-field px-4 py-3 text-xs"
	>
		<span class="text-foreground">{t.browserAuthPending}</span>
		<button
			type="button"
			onclick={() => void controller.cancel()}
			class="flex min-h-[34px] items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
		>
			<X class="h-3.5 w-3.5" aria-hidden="true" />
			<span>{t.cancelAuth}</span>
		</button>
	</div>
{/if}

{#if controller.testResult?.ok}
	<div
		role="status"
		class="flex items-start gap-3 rounded-md border border-status-healthy/40 bg-field px-4 py-3 text-xs"
	>
		<CheckCircle2 class="mt-1 h-4 w-4 shrink-0 text-status-healthy" aria-hidden="true" />
		<p class="font-medium text-foreground">{t.testLoginSuccess}</p>
	</div>
{:else if controller.diagnostic}
	{@const guidance = t.diagnostics[controller.diagnostic]}
	<div
		role="alert"
		class="flex items-start gap-3 rounded-md border border-status-warning/50 bg-field px-4 py-3 text-xs"
	>
		<AlertTriangle class="mt-1 h-4 w-4 shrink-0 text-status-warning" aria-hidden="true" />
		<div class="space-y-1">
			<p class="font-semibold text-foreground">{guidance.title}</p>
			<p class="leading-relaxed text-secondary">{guidance.hint}</p>
			<WdgProviderDiagnosticHint code={controller.diagnostic} {host} {method} />
		</div>
	</div>
{:else if controller.error}
	<p role="alert" class="rounded-md border border-separator bg-field px-4 py-3 text-xs font-medium text-status-danger">
		{controller.error}
	</p>
{/if}

<div id="account-help-guide">
	<WdgAccountHelpGuide
		selectedMethod={method}
		{selectedPreset}
		{routing}
		bind:open={guideOpen}
		bind:activeTab={guideTab}
	/>
</div>
