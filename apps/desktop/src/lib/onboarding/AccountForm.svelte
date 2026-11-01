<script lang="ts">
	import { clearCredentialInputs } from './forms';
	import type { AccountController } from './controller.svelte';
	import type { AccountConfig, AuthMethod, AccountSubmission } from './types';
	import { i18n } from '../i18n/index.svelte';
	import AccountHelpGuide from './AccountHelpGuide.svelte';

	let { controller }: { controller: AccountController } = $props();
	let method = $state<AuthMethod>('AppPassword');
	let form = $state<HTMLFormElement>();

	let presetHost = $state('');
	let presetPort = $state(993);
	let presetAuth = $state('');
	let presetToken = $state('');
	let presetScopes = $state('');

	const t = $derived(i18n.t.accountForm);
	const oauth = $derived(method === 'OAuth2PublicPkce' || method === 'OAuth2ByoCredentials');

	interface Preset {
		name: string;
		host: string;
		port: number;
		method: AuthMethod;
		authEndpoint?: string;
		tokenEndpoint?: string;
		scopes?: string;
	}

	const presets: Preset[] = [
		{ name: 'Gmail', host: 'imap.gmail.com', port: 993, method: 'AppPassword' },
		{
			name: 'Outlook',
			host: 'outlook.office365.com',
			port: 993,
			method: 'OAuth2PublicPkce',
			authEndpoint: 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize',
			tokenEndpoint: 'https://login.microsoftonline.com/common/oauth2/v2.0/token',
			scopes: 'offline_access https://outlook.office.com/IMAP.AccessAsUser.All'
		},
		{ name: 'iCloud', host: 'imap.mail.me.com', port: 993, method: 'AppPassword' },
		{ name: 'Yahoo', host: 'imap.mail.yahoo.com', port: 993, method: 'AppPassword' },
		{ name: 'Fastmail', host: 'imap.fastmail.com', port: 993, method: 'AppPassword' },
		{ name: 'Proton', host: '127.0.0.1', port: 1143, method: 'ProtonBridgeLocal' }
	];

	function applyPreset(p: Preset) {
		presetHost = p.host;
		presetPort = p.port;
		presetAuth = p.authEndpoint ?? '';
		presetToken = p.tokenEndpoint ?? '';
		presetScopes = p.scopes ?? '';
		method = p.method;
	}

	$effect(() => {
		const current = form;
		// INVARIANT: Capture the old form so a mode switch clears even detached inputs.
		return () => clearCredentialInputs(current);
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (controller.busy || !form) return;
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
		<p class="mt-2 text-sm text-[var(--secondary)]">
			{t.subtitle}
		</p>
	</div>

	<!-- Interactive Login Methods Guide -->
	<AccountHelpGuide selectedMethod={method} />

	<!-- Provider Presets -->
	<div class="space-y-2">
		<span class="text-xs font-medium text-[var(--secondary)]">{t.quickFillLabel}</span>
		<div class="flex flex-wrap gap-1.5">
			{#each presets as p}
				<button
					type="button"
					disabled={controller.busy}
					class="rounded-full border border-[var(--separator)] bg-[var(--surface)] px-2.5 py-1 text-xs font-medium text-[var(--secondary)] transition-all hover:border-[var(--accent)] hover:text-[var(--text)] active:scale-95 disabled:opacity-50"
					onclick={() => applyPreset(p)}
				>
					{p.name}
				</button>
			{/each}
		</div>
	</div>

	<label class="block text-sm"
		>{t.methodLabel}
		<select class="setup-input mt-2" bind:value={method} disabled={controller.busy}>
			<option value="AppPassword">{t.methodAppPassword}</option>
			<option value="OAuth2PublicPkce">{t.methodOAuthPublic}</option>
			<option value="OAuth2ByoCredentials">{t.methodOAuthByo}</option>
			<option value="ProtonBridgeLocal">{t.methodProton}</option>
		</select>
	</label>

	{#key method}
		<form bind:this={form} onsubmit={submit} autocomplete="off" class="space-y-4">
			<fieldset disabled={controller.busy} class="space-y-4">
				<label class="block text-sm"
					>{method === 'ProtonBridgeLocal' ? t.emailLabelBridge : t.emailLabel}
					<input
						class="setup-input mt-2"
						name="email"
						type="text"
						required
						maxlength="320"
						autocomplete="off"
					/>
				</label>
				<div class="grid grid-cols-[minmax(0,1fr)_6rem] gap-4">
					<label class="block text-sm"
						>{t.hostLabel}
						<input
							class="setup-input mt-2"
							name="host"
							required
							maxlength="253"
							value={method === 'ProtonBridgeLocal' ? '127.0.0.1' : presetHost}
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
							value={method === 'ProtonBridgeLocal' ? 1143 : presetPort}
						/>
					</label>
				</div>
				{#if oauth}
					<p class="text-sm text-[var(--secondary)]">
						{t.oauthHelp}
					</p>
					<label class="block text-sm"
						>{t.authEndpointLabel}
						<input
							class="setup-input mt-2"
							name="authorize"
							type="url"
							required
							pattern="https://.*"
							maxlength="2048"
							value={presetAuth}
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
							value={presetToken}
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
							value={presetScopes}
						/>
					</label>
					<p id="scope-help" class="text-xs text-[var(--secondary)]">
						{t.scopesHelp}
					</p>
					<label class="block text-sm"
						>{t.clientIdLabel}
						<input
							class="setup-input mt-2"
							name="client"
							data-credential
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
							required
							maxlength="8192"
							autocomplete="off"
						/>
					</label>
				{/if}
				{#if method === 'ProtonBridgeLocal'}
					<p class="text-sm text-[var(--secondary)]">
						{t.protonNotice}
					</p>
				{/if}
				<p class="text-xs leading-relaxed text-[var(--secondary)]">
					{oauth ? t.oauthVaultNotice : t.connectVaultNotice}
				</p>
				<button type="submit" class="setup-primary"
					>{controller.busy
						? t.connecting
						: oauth
							? t.authorizeAndConnect
							: t.connectAccount}</button
				>
			</fieldset>
		</form>
	{/key}
	{#if controller.authorizing}
		<p role="status" class="text-sm">
			{t.browserAuthPending}
		</p>
		<button class="setup-secondary" type="button" onclick={() => void controller.cancel()}
			>{t.cancelAuth}</button
		>
	{/if}
</section>
