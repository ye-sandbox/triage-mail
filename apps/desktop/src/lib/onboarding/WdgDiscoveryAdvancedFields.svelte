<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { i18n } from '../i18n/index.svelte';
	import type { DiscoveryController } from './discovery-controller.svelte';

	interface Props {
		discovery: DiscoveryController;
		isBridge: boolean;
		oauth: boolean;
		showAdvanced: boolean;
		inputCls: string;
		onToggleAdvanced: () => void;
		onEdit: () => void;
	}

	let {
		discovery,
		isBridge,
		oauth,
		showAdvanced,
		inputCls,
		onToggleAdvanced,
		onEdit
	}: Props = $props();

	const t = $derived(i18n.t.accountForm);
</script>

<div class="flex flex-wrap items-center justify-between gap-2 px-1">
	<span class="min-w-0 truncate font-mono text-[11px] text-secondary">
		{discovery.settings.host.trim()
			? `${t.resolvedServerSummary} · ${discovery.settings.host}:${discovery.settings.port}`
			: t.customImapOption}
	</span>
	<button
		type="button"
		aria-expanded={showAdvanced}
		aria-controls="advanced-connection-settings"
		onclick={onToggleAdvanced}
		class="flex shrink-0 items-center gap-1 text-xs font-medium text-secondary whitespace-nowrap hover:text-foreground"
	>
		<span>{t.advancedSettings}</span>
		<ChevronDown
			class="h-3.5 w-3.5 transition-transform duration-150 {showAdvanced ? 'rotate-180' : ''}"
			aria-hidden="true"
		/>
	</button>
</div>

<div
	id="advanced-connection-settings"
	hidden={!showAdvanced}
	class="space-y-3 rounded-md border border-separator bg-field/50 p-4"
>
	<div class="grid grid-cols-1 gap-3 sm:grid-cols-6">
		<label class="flex flex-col gap-1 sm:col-span-4">
			<span class="text-xs font-medium text-secondary">{t.hostLabel}</span>
			<input
				name="host"
				type="text"
				required
				maxlength="253"
				readonly={isBridge}
				bind:value={discovery.settings.host}
				oninput={onEdit}
				placeholder={isBridge ? '127.0.0.1' : 'imap.example.com'}
				class="{inputCls} font-mono"
			/>
		</label>
		<label class="flex flex-col gap-1 sm:col-span-2">
			<span class="text-xs font-medium text-secondary">{t.portLabel}</span>
			<input
				name="port"
				type="number"
				required
				min="1"
				max="65535"
				readonly={isBridge}
				bind:value={discovery.settings.port}
				oninput={onEdit}
				class="{inputCls} font-mono tabular-nums"
			/>
		</label>
	</div>

	{#if oauth}
		<p class="text-xs text-secondary">{t.oauthHelp}</p>
		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			<label class="flex flex-col gap-1">
				<span class="text-xs font-medium text-secondary">{t.authEndpointLabel}</span>
				<input
					name="authorize"
					type="url"
					required
					pattern="https://.*"
					maxlength="2048"
					bind:value={discovery.settings.authorize}
					oninput={onEdit}
					class="{inputCls} font-mono"
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-xs font-medium text-secondary">{t.tokenEndpointLabel}</span>
				<input
					name="token"
					type="url"
					required
					pattern="https://.*"
					maxlength="2048"
					bind:value={discovery.settings.token}
					oninput={onEdit}
					class="{inputCls} font-mono"
				/>
			</label>
			<label class="flex flex-col gap-1 sm:col-span-2">
				<span class="text-xs font-medium text-secondary">{t.scopesLabel}</span>
				<input
					name="scopes"
					type="text"
					required
					maxlength="4096"
					bind:value={discovery.settings.scopes}
					oninput={onEdit}
					class="{inputCls} font-mono"
				/>
				<span class="text-[11px] text-secondary">{t.scopesHelp}</span>
			</label>
		</div>
	{/if}
</div>
