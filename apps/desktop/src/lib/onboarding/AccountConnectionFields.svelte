<script lang="ts">
	import type { DiscoveryController } from './discovery-controller.svelte';
	import type { AuthMethod } from './types';
	import { i18n } from '../i18n/index.svelte';
	let { discovery, beforeEdit }: { discovery: DiscoveryController; beforeEdit: () => void } =
		$props();
	let expanded = $state(false);
	$effect(() => {
		if (discovery.status === 'unresolved') expanded = true;
	});
	const t = $derived(i18n.t.accountForm);
	const oauth = $derived(
		discovery.settings.method === 'OAuth2PublicPkce' ||
			discovery.settings.method === 'OAuth2ByoCredentials'
	);
	function edit() {
		beforeEdit();
		discovery.edit();
	}
</script>

<details class="space-y-4 rounded-lg border border-[var(--separator)] p-4" bind:open={expanded}>
	<summary class="cursor-pointer text-sm font-medium">{t.advancedSettings}</summary>
	<label class="block text-sm"
		>{t.methodLabel}
		<select
			class="setup-input mt-2"
			value={discovery.settings.method}
			onchange={(event) => {
				edit();
				discovery.settings.method = event.currentTarget.value as AuthMethod;
				if (discovery.settings.method === 'ProtonBridgeLocal') {
					discovery.settings.host = '127.0.0.1';
					discovery.settings.port = 1143;
				}
			}}
		>
			<option value="AppPassword">{t.methodAppPassword}</option>
			<option value="OAuth2PublicPkce">{t.methodOAuthPublic}</option>
			<option value="OAuth2ByoCredentials">{t.methodOAuthByo}</option>
			<option value="ProtonBridgeLocal">{t.methodProton}</option>
		</select>
	</label>
	<div class="grid grid-cols-[minmax(0,1fr)_6rem] gap-4">
		<label class="block text-sm"
			>{t.hostLabel}
			<input
				class="setup-input mt-2"
				name="host"
				required
				maxlength="253"
				bind:value={discovery.settings.host}
				oninput={() => discovery.edit()}
			/>
		</label>
		<label class="block text-sm"
			>{t.portLabel}
			<input
				class="setup-input mt-2 tabular-nums"
				name="port"
				type="number"
				required
				min="1"
				max="65535"
				bind:value={discovery.settings.port}
				oninput={() => discovery.edit()}
			/>
		</label>
	</div>
	{#if oauth}
		<label class="block text-sm"
			>{t.authEndpointLabel}
			<input
				class="setup-input mt-2"
				name="authorize"
				type="url"
				required
				pattern="https://.*"
				maxlength="2048"
				bind:value={discovery.settings.authorize}
				oninput={() => discovery.edit()}
			/>
		</label>
		<label class="block text-sm"
			>{t.tokenEndpointLabel}
			<input
				class="setup-input mt-2"
				name="token"
				type="url"
				required
				pattern="https://.*"
				maxlength="2048"
				bind:value={discovery.settings.token}
				oninput={() => discovery.edit()}
			/>
		</label>
		<label class="block text-sm"
			>{t.scopesLabel}
			<input
				class="setup-input mt-2"
				name="scopes"
				required
				maxlength="4096"
				aria-describedby="scope-help"
				bind:value={discovery.settings.scopes}
				oninput={() => discovery.edit()}
			/>
		</label>
		<p id="scope-help" class="text-xs text-[var(--secondary)]">
			{t.scopesHelp}
		</p>
	{/if}
</details>
