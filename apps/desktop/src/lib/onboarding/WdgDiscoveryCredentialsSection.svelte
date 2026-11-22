<script lang="ts">
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Timer from '@lucide/svelte/icons/timer';
	import X from '@lucide/svelte/icons/x';
	import { i18n } from '../i18n/index.svelte';
	import { formatPkceTimer } from './onboarding.fixtures';
	import type { AuthMethod } from './types';
	import WdgGmailAppPasswordHint from './WdgGmailAppPasswordHint.svelte';

	interface Props {
		method: AuthMethod;
		oauth: boolean;
		isBridge: boolean;
		demoMode: boolean;
		selectedPreset: string;
		gmailSecretSuspect: boolean;
		inputCls: string;
		demoPkceState: 'idle' | 'binding' | 'waiting' | 'authorized';
		demoPkceReady: boolean;
		demoPkceSeconds: number;
		onProtectCredentials: () => void;
		onRefreshGmailHint: () => void;
		onStartDemoPkce: () => void;
		onCancelDemoPkce: () => void;
		onCompleteDemoPkce: () => void;
		onOpenGoogleAppPasswords: () => void;
	}

	let {
		method,
		oauth,
		isBridge,
		demoMode,
		selectedPreset,
		gmailSecretSuspect,
		inputCls,
		demoPkceState,
		demoPkceReady,
		demoPkceSeconds,
		onProtectCredentials,
		onRefreshGmailHint,
		onStartDemoPkce,
		onCancelDemoPkce,
		onCompleteDemoPkce,
		onOpenGoogleAppPasswords
	}: Props = $props();

	const t = $derived(i18n.t.accountForm);
</script>

{#key method}
	<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
		{#if oauth}
			<label class="flex flex-col gap-1 {method === 'OAuth2PublicPkce' ? 'sm:col-span-2' : ''}">
				<span class="text-xs font-medium text-secondary">{t.clientIdLabel}</span>
				<input
					name="client"
					type="text"
					data-credential
					required
					maxlength="4096"
					autocomplete="off"
					oninput={() => {
						onProtectCredentials();
						onRefreshGmailHint();
					}}
					class="{inputCls} font-mono"
				/>
			</label>
		{/if}
		{#if method !== 'OAuth2PublicPkce'}
			<label class="flex flex-col gap-1 {!oauth ? 'sm:col-span-2' : ''}">
				<span class="text-xs font-medium text-secondary">
					{method === 'OAuth2ByoCredentials'
						? t.clientSecretLabel
						: isBridge
							? t.bridgePasswordLabel
							: t.appPasswordLabel}
				</span>
				<input
					name="secret"
					type="password"
					data-credential
					required
					maxlength="8192"
					autocomplete="off"
					oninput={() => {
						onProtectCredentials();
						onRefreshGmailHint();
					}}
					class="{inputCls} font-mono"
				/>
			</label>
		{/if}
	</div>
{/key}

{#if demoMode && method === 'OAuth2PublicPkce'}
	<div class="flex flex-wrap items-center gap-2 rounded-md border border-separator bg-field p-3">
		<button
			type="button"
			onclick={onStartDemoPkce}
			disabled={demoPkceState === 'binding' || demoPkceState === 'waiting'}
			class="flex min-h-[38px] items-center gap-2 rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground disabled:opacity-60"
		>
			<KeyRound class="h-3.5 w-3.5" aria-hidden="true" />
			<span>{t.authorizeAndConnect} (127.0.0.1 PKCE)</span>
		</button>
		<button
			type="button"
			onclick={onCancelDemoPkce}
			disabled={!demoPkceReady}
			class="flex min-h-[38px] items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground disabled:opacity-40"
		>
			<X class="h-3.5 w-3.5" aria-hidden="true" />
			<span>{t.cancelAuth}</span>
		</button>
		{#if demoPkceState === 'binding' || demoPkceState === 'waiting'}
			<span class="flex items-center gap-2 font-mono text-xs tabular-nums text-status-info">
				<Timer class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{formatPkceTimer(demoPkceSeconds)}</span>
			</span>
			{#if demoPkceReady}
				<button
					type="button"
					onclick={onCompleteDemoPkce}
					class="ml-auto rounded border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
				>
					Simulate Loopback Callback
				</button>
			{/if}
		{:else if demoPkceState === 'authorized'}
			<span class="flex items-center gap-2 text-xs font-medium text-status-healthy">
				<CheckCircle2 class="h-3.5 w-3.5" aria-hidden="true" />
				<span>127.0.0.1 PKCE OK</span>
			</span>
		{/if}
	</div>
{/if}

{#if selectedPreset === 'Gmail' && method === 'AppPassword'}
	<WdgGmailAppPasswordHint
		suspect={gmailSecretSuspect}
		hint={t.gmailRegularPasswordHint}
		buttonLabel={t.gmailAppPasswordHelp}
		onOpen={onOpenGoogleAppPasswords}
	/>
{/if}
