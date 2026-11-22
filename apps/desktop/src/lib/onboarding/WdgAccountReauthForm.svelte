<script lang="ts">
	import { untrack } from 'svelte';
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import X from '@lucide/svelte/icons/x';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { ConnectionDiagnosticCode } from '../inbox/types';
	import { clearCredentialInputs } from './forms';
	import { oauthDefaultsForHost } from './presets';
	import type { AccountConfig, Reauthorization } from './types';

	interface Props {
		account: AccountConfig;
		busy: boolean;
		authorizing: boolean;
		reauthorizingAccountId: string | null;
		failed: boolean;
		diagnostic: ConnectionDiagnosticCode | null;
		error: string;
		onReauthorize: (input: Reauthorization) => Promise<boolean>;
		onCancelReauthorize?: () => Promise<void>;
		onClose: () => void;
		onSuccess: () => void;
		onFailure: () => void;
	}

	let {
		account,
		busy,
		authorizing,
		reauthorizingAccountId,
		failed,
		diagnostic,
		error,
		onReauthorize,
		onCancelReauthorize,
		onClose,
		onSuccess,
		onFailure
	}: Props = $props();

	const defaults = untrack(() => oauthDefaultsForHost(account.imap_host));
	let reauthAuthorize = $state(defaults?.authEndpoint ?? '');
	let reauthToken = $state(defaults?.tokenEndpoint ?? '');
	let reauthScopes = $state(defaults?.scopes ?? '');
	let reauthForm = $state<HTMLFormElement>();
	let active = true;

	const t = $derived(i18n.t.onboarding);
	const formT = $derived(i18n.t.accountForm);
	const INPUT_CLS =
		'h-9 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-60';

	$effect(() => {
		active = true;
		const currentForm = reauthForm;
		return () => {
			active = false;
			clearCredentialInputs(currentForm);
		};
	});

	async function submitReauthorize(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !reauthForm) return;
		const data = new FormData(reauthForm);
		const value = (key: string) => String(data.get(key) ?? '').trim();
		const tokenEndpoint = value('token');
		const clientId = value('client');
		const clientSecret =
			account.auth_method === 'OAuth2ByoCredentials' ? String(data.get('secret') ?? '').trim() : '';
		let payload: Reauthorization = {
			account_id: account.account_id,
			authorization_endpoint: value('authorize'),
			...(tokenEndpoint ? { token_endpoint: tokenEndpoint } : {}),
			scopes: value('scopes'),
			...(clientId ? { client_id: clientId } : {}),
			...(clientSecret ? { client_secret: clientSecret } : {})
		};
		const pending = onReauthorize(payload);
		payload = { account_id: '', authorization_endpoint: '', scopes: '' };
		for (const key of ['secret', 'client']) data.delete(key);
		clearCredentialInputs(reauthForm);
		const ok = await pending;
		if (!active) return;
		if (ok) {
			toast.success(t.reauthorizeSuccess);
			onSuccess();
		} else {
			onFailure();
		}
	}
</script>

<form
	bind:this={reauthForm}
	onsubmit={submitReauthorize}
	autocomplete="off"
	class="space-y-3 border-t border-separator bg-field/50 px-4 py-3"
>
	<p class="text-xs text-secondary">{t.reauthorizeHelp}</p>
	<fieldset disabled={busy} class="space-y-3">
		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			<label class="flex flex-col gap-1">
				<span class="text-xs font-medium text-secondary">{formT.authEndpointLabel}</span>
				<input
					name="authorize"
					type="url"
					required
					pattern="https://.*"
					maxlength="2048"
					bind:value={reauthAuthorize}
					class="{INPUT_CLS} font-mono"
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-xs font-medium text-secondary">{formT.tokenEndpointLabel}</span>
				<input
					name="token"
					type="url"
					pattern="https://.*"
					maxlength="2048"
					bind:value={reauthToken}
					class="{INPUT_CLS} font-mono"
				/>
			</label>
			<label class="flex flex-col gap-1 sm:col-span-2">
				<span class="text-xs font-medium text-secondary">{formT.scopesLabel}</span>
				<input
					name="scopes"
					type="text"
					required
					maxlength="4096"
					bind:value={reauthScopes}
					class="{INPUT_CLS} font-mono"
				/>
			</label>
		</div>

		<p class="text-[11px] text-secondary">{t.reauthorizeClientOverrideHint}</p>

		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			<label class="flex flex-col gap-1 {account.auth_method === 'OAuth2PublicPkce' ? 'sm:col-span-2' : ''}">
				<span class="text-xs font-medium text-secondary">{formT.clientIdLabel}</span>
				<input
					name="client"
					type="text"
					data-credential
					maxlength="4096"
					autocomplete="off"
					class="{INPUT_CLS} font-mono"
				/>
			</label>
			{#if account.auth_method === 'OAuth2ByoCredentials'}
				<label class="flex flex-col gap-1">
					<span class="text-xs font-medium text-secondary">{formT.clientSecretLabel}</span>
					<input
						name="secret"
						type="password"
						data-credential
						maxlength="8192"
						autocomplete="off"
						class="{INPUT_CLS} font-mono"
					/>
				</label>
			{/if}
		</div>

		<div class="flex flex-wrap items-center justify-end gap-2">
			<button
				type="button"
				onclick={() => {
					clearCredentialInputs(reauthForm);
					onClose();
				}}
				class="flex min-h-[34px] items-center rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-field"
			>
				{t.reauthorizeCancel}
			</button>
			<button
				type="submit"
				disabled={busy}
				class="flex min-h-[34px] items-center gap-2 rounded-md bg-accent px-4 py-1 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-60"
			>
				<KeyRound class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{reauthorizingAccountId === account.account_id ? t.reauthorizing : t.reauthorizeSubmit}</span>
			</button>
		</div>
	</fieldset>

	{#if authorizing && reauthorizingAccountId === account.account_id}
		<div
			role="status"
			class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-separator bg-surface px-3 py-2 text-xs"
		>
			<span class="text-foreground">{formT.browserAuthPending}</span>
			{#if onCancelReauthorize}
				<button
					type="button"
					onclick={() => void onCancelReauthorize?.()}
					class="flex min-h-[32px] items-center gap-1 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
				>
					<X class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{formT.cancelAuth}</span>
				</button>
			{/if}
		</div>
	{/if}

	{#if failed}
		{#if diagnostic}
			{@const guidance = formT.diagnostics[diagnostic]}
			<div role="alert" class="flex items-start gap-3 rounded-md border border-status-warning/50 bg-surface px-3 py-3 text-xs">
				<AlertTriangle class="mt-1 h-4 w-4 shrink-0 text-status-warning" aria-hidden="true" />
				<div class="space-y-1">
					<p class="font-semibold text-foreground">{guidance.title}</p>
					<p class="leading-relaxed text-secondary">{guidance.hint}</p>
				</div>
			</div>
		{:else if error}
			<p role="alert" class="rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-status-danger">
				{error}
			</p>
		{/if}
	{/if}
</form>
