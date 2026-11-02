<script lang="ts">
	import type { AuthMethod } from './types';
	import { i18n } from '../i18n/index.svelte';
	let { method, onEdit }: { method: AuthMethod; onEdit: () => void } = $props();
	const t = $derived(i18n.t.accountForm);
	const oauth = $derived(method === 'OAuth2PublicPkce' || method === 'OAuth2ByoCredentials');
</script>

{#if oauth}
	<p class="text-sm text-[var(--secondary)]">{t.oauthHelp}</p>
	<label class="block text-sm"
		>{t.clientIdLabel}
		<input
			class="setup-input mt-2"
			name="client"
			data-credential
			oninput={onEdit}
			required
			maxlength="4096"
			autocomplete="off"
		/>
	</label>
{/if}
{#if method !== 'OAuth2PublicPkce'}
	<label class="block text-sm"
		>{method === 'OAuth2ByoCredentials'
			? t.clientSecretLabel
			: method === 'ProtonBridgeLocal'
				? t.bridgePasswordLabel
				: t.appPasswordLabel}
		<input
			class="setup-input mt-2"
			name="secret"
			type="password"
			data-credential
			oninput={onEdit}
			required
			maxlength="8192"
			autocomplete="off"
		/>
	</label>
{/if}
{#if method === 'ProtonBridgeLocal'}
	<p class="text-sm text-[var(--secondary)]">{t.protonNotice}</p>
{/if}
