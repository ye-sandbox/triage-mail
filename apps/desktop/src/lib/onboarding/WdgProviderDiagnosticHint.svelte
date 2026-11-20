<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { ConnectionDiagnosticCode } from '../inbox/types';
	import { providerHintKey } from './provider-hints';
	import type { AuthMethod } from './types';

	type Props = {
		code: ConnectionDiagnosticCode | null | undefined;
		host: string;
		method: AuthMethod;
	};
	let { code, host, method }: Props = $props();
	const key = $derived(providerHintKey(code, host, method));
	const hint = $derived(key ? i18n.t.accountForm.providerHints[key] : '');
</script>

{#if hint}
	<p class="leading-relaxed text-secondary">{hint}</p>
{/if}
