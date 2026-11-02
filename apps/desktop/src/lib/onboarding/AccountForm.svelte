<script lang="ts">
	import { untrack } from 'svelte';
	import { clearCredentialInputs } from './forms';
	import type { AccountController } from './controller.svelte';
	import type { AccountConfig, AccountSubmission } from './types';
	import { i18n } from '../i18n/index.svelte';
	import AccountHelpGuide from './AccountHelpGuide.svelte';
	import AccountConnectionFields from './AccountConnectionFields.svelte';
	import AccountCredentialFields from './AccountCredentialFields.svelte';
	import { microsoftPreset, providerPresets, type ProviderPreset } from './presets';
	import { nativeDiscoveryTransport, type DiscoveryTransport } from './discovery';
	import { DiscoveryController } from './discovery-controller.svelte';

	let {
		controller,
		discoveryTransport = nativeDiscoveryTransport
	}: {
		controller: AccountController;
		discoveryTransport?: DiscoveryTransport;
	} = $props();
	const discovery = new DiscoveryController(
		untrack(() => discoveryTransport),
		() => clearCredentialInputs(form)
	);
	let form = $state<HTMLFormElement>();
	let emailInput = $state<HTMLInputElement>();
	const t = $derived(i18n.t.accountForm);
	const method = $derived(discovery.settings.method);
	const oauth = $derived(method === 'OAuth2PublicPkce' || method === 'OAuth2ByoCredentials');

	function applyPreset(preset: ProviderPreset) {
		clearCredentialInputs(form);
		discovery.preset(preset);
	}
	$effect(() => {
		const current = form;
		const currentMethod = method;
		const inputs = current?.querySelectorAll<HTMLInputElement>('[data-credential]');
		// INVARIANT: Capture the actual inputs, including ones detached by a mode change.
		return () => {
			if (currentMethod)
				inputs?.forEach((input) => {
					input.value = '';
				});
		};
	});
	$effect(() => () => discovery.dispose());

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (controller.busy || discovery.busy || !form) return;
		discovery.invalidate();
		const data = new FormData(form);
		const value = (key: string) => String(data.get(key) ?? '').trim();
		const config: AccountConfig = {
			account_id: '',
			email_display: value('email'),
			imap_host: value('host'),
			imap_port: Number(value('port')),
			auth_method: method
		};
		let submission: AccountSubmission;
		if (oauth) {
			submission = {
				authorization: {
					authorization_endpoint: value('authorize'),
					token_endpoint: value('token'),
					scopes: value('scopes'),
					client_id: value('client'),
					...(method === 'OAuth2ByoCredentials'
						? { client_secret: String(data.get('secret') ?? '') }
						: {})
				}
			};
		} else {
			const secret = String(data.get('secret') ?? '');
			submission = {
				credentials:
					method === 'ProtonBridgeLocal'
						? { ProtonBridgeLocal: { bridge_password: secret } }
						: { AppPassword: { secret } }
			};
		}
		const pending = controller.add(config, submission);
		submission = { credentials: { AppPassword: { secret: '' } } };
		for (const key of ['secret', 'client']) data.delete(key);
		clearCredentialInputs(form);
		await pending;
	}
</script>

<section aria-labelledby="add-account-title" class="space-y-6">
	<div>
		<h2 id="add-account-title" class="text-xl font-semibold">{t.title}</h2>
		<p class="mt-2 text-sm text-[var(--secondary)]">{t.discoveryIntro}</p>
	</div>
	<form bind:this={form} onsubmit={submit} autocomplete="off" class="space-y-4">
		<fieldset disabled={controller.busy} class="space-y-4">
			<label class="block text-sm"
				>{method === 'ProtonBridgeLocal' ? t.emailLabelBridge : t.emailLabel}
				<input
					bind:this={emailInput}
					class="setup-input mt-2"
					name="email"
					type="text"
					required
					maxlength="320"
					autocomplete="off"
					oninput={() => {
						clearCredentialInputs(form);
						discovery.input();
					}}
					onblur={(event) => discovery.blur(event.currentTarget.value)}
				/>
			</label>
			<div class="space-y-2">
				<span class="text-xs font-medium text-[var(--secondary)]">{t.quickFillLabel}</span>
				<div class="flex flex-wrap gap-2">
					{#each providerPresets as preset}
						<button
							type="button"
							class="setup-secondary text-sm aria-pressed:bg-[var(--selection-soft)] aria-pressed:text-[var(--accent)]"
							aria-pressed={discovery.selectedPreset === preset.name}
							onclick={() => applyPreset(preset)}>{preset.name}</button
						>
					{/each}
				</div>
			</div>
			<button type="button" class="setup-secondary" onclick={() => applyPreset(microsoftPreset)}
				>{t.microsoftSetup}</button
			>
			<p class="text-xs text-[var(--secondary)]">{t.microsoftSetupHelp}</p>
			<div class="space-y-2" aria-busy={discovery.busy}>
				<p class="text-xs text-[var(--secondary)]">{t.onlineLookupHelp}</p>
				<button
					type="button"
					class="setup-secondary"
					disabled={discovery.busy}
					onclick={() => void discovery.online(emailInput?.value ?? '')}>{t.onlineLookup}</button
				>
				{#if discovery.settings.host && discovery.settings.port === 993 && method !== 'ProtonBridgeLocal'}
					<p class="text-xs text-[var(--secondary)]">{t.probeHelp}</p>
					<button
						type="button"
						class="setup-secondary"
						disabled={discovery.busy}
						onclick={() => void discovery.probe()}
						>{t.confirmProbe} {discovery.settings.host}:993</button
					>
				{/if}
				<p role="status" class="text-sm text-[var(--secondary)]">
					{#if discovery.busy}{t.discovering}
					{:else if discovery.status === 'found'}{t.providerFound}
						{discovery.provider?.provider_name} · {discovery.settings.host}:{discovery.settings
							.port}
					{:else if discovery.status === 'unresolved'}{t.discoveryUnresolved}
					{:else if discovery.status === 'invalid'}{t.discoveryInvalid}
					{:else if discovery.status === 'unavailable'}{t.discoveryUnavailable}
					{:else if discovery.status === 'probeSuccess'}{t.probeSuccess}
					{:else if discovery.status === 'probeFailed'}{t.probeFailed}{/if}
				</p>
			</div>
			{#if discovery.selectedPreset === 'Gmail' && method === 'AppPassword'}
				<a
					class="text-sm text-[var(--accent)] underline"
					href="https://myaccount.google.com/apppasswords"
					target="_blank"
					rel="noopener noreferrer">{t.gmailAppPasswordHelp}</a
				>
			{/if}
			<AccountConnectionFields {discovery} beforeEdit={() => clearCredentialInputs(form)} />
			{#key method}<AccountCredentialFields
					{method}
					onEdit={() => discovery.protectCredentials()}
				/>{/key}
			<p class="text-xs leading-relaxed text-[var(--secondary)]">
				{oauth ? t.oauthVaultNotice : t.connectVaultNotice}
			</p>
			<button type="submit" class="setup-primary" disabled={discovery.busy}
				>{controller.busy ? t.connecting : oauth ? t.authorizeAndConnect : t.connectAccount}</button
			>
		</fieldset>
	</form>
	<AccountHelpGuide selectedMethod={method} selectedPreset={discovery.selectedPreset} />
	{#if controller.authorizing}
		<p role="status" class="text-sm">{t.browserAuthPending}</p>
		<button class="setup-secondary" type="button" onclick={() => void controller.cancel()}
			>{t.cancelAuth}</button
		>
	{/if}
</section>
