<script lang="ts">
	import { clearCredentialInputs } from './forms';
	import type { AccountController } from './controller.svelte';
	import type { AccountConfig, AuthMethod, AccountSubmission } from './types';
	let { controller }: { controller: AccountController } = $props();
	let method = $state<AuthMethod>('AppPassword');
	let form = $state<HTMLFormElement>();
	const oauth = $derived(method === 'OAuth2PublicPkce' || method === 'OAuth2ByoCredentials');
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
		<h2 id="add-account-title" class="text-xl font-semibold">Connect an account</h2>
		<p class="mt-2 text-sm text-[var(--secondary)]">
			Use the connection settings supplied by your mail provider.
		</p>
	</div>
	<label class="block text-sm"
		>Sign-in method
		<select class="setup-input mt-2" bind:value={method} disabled={controller.busy}>
			<option value="AppPassword">App password</option>
			<option value="OAuth2PublicPkce">OAuth · Public PKCE</option>
			<option value="OAuth2ByoCredentials">OAuth · Your client credentials</option>
			<option value="ProtonBridgeLocal">Proton Mail Bridge</option>
		</select>
	</label>
	{#key method}
		<form bind:this={form} onsubmit={submit} autocomplete="off" class="space-y-4">
			<fieldset disabled={controller.busy} class="space-y-4">
				<label class="block text-sm"
					>{method === 'ProtonBridgeLocal' ? 'Bridge username' : 'Email address or IMAP username'}
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
						>IMAP host
						<input
							class="setup-input mt-2"
							name="host"
							required
							maxlength="253"
							value={method === 'ProtonBridgeLocal' ? '127.0.0.1' : ''}
						/>
					</label>
					<label class="block text-sm"
						>Port
						<input
							class="setup-input mt-2 tabular-nums"
							name="port"
							type="number"
							required
							min="1"
							max="65535"
							value={method === 'ProtonBridgeLocal' ? 1143 : 993}
						/>
					</label>
				</div>
				{#if oauth}
					<p class="text-sm text-[var(--secondary)]">
						Register a desktop client with your provider using a loopback redirect at <code
							>http://127.0.0.1/callback</code
						> with a dynamic port supported by your provider. Authorization opens in your browser.
					</p>
					<label class="block text-sm"
						>Authorization endpoint (HTTPS)
						<input
							class="setup-input mt-2"
							name="authorize"
							type="url"
							required
							pattern="https://.*"
							maxlength="2048"
						/>
					</label>
					<label class="block text-sm"
						>Token endpoint (HTTPS)
						<input
							class="setup-input mt-2"
							name="token"
							type="url"
							required
							pattern="https://.*"
							maxlength="2048"
						/>
					</label>
					<label class="block text-sm"
						>IMAP scopes
						<input
							class="setup-input mt-2"
							name="scopes"
							required
							maxlength="4096"
							aria-describedby="scope-help"
						/>
					</label>
					<p id="scope-help" class="text-xs text-[var(--secondary)]">
						Space-separated provider scopes. Include offline access if your provider supports it.
					</p>
					<label class="block text-sm"
						>Client ID
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
							? 'Client secret'
							: method === 'ProtonBridgeLocal'
								? 'Bridge password'
								: 'App password'}
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
						Start Proton Mail Bridge first and select its SSL/TLS IMAP mode. Only loopback hosts are
						allowed; local self-signed TLS is accepted.
					</p>
				{/if}
				<p class="text-xs leading-relaxed text-[var(--secondary)]">
					{oauth
						? 'Authorize contacts your configured provider endpoints. Tokens and client credentials stay in the native vault. Automatic token refresh is not available yet.'
						: 'Connect saves credentials in the native vault and starts inbound mail sync.'}
				</p>
				<button type="submit" class="setup-primary"
					>{controller.busy
						? 'Connecting…'
						: oauth
							? 'Authorize and connect'
							: 'Connect account'}</button
				>
			</fieldset>
		</form>
	{/key}
	{#if controller.authorizing}
		<p role="status" class="text-sm">
			Finish authorization in your browser. This attempt expires in five minutes.
		</p>
		<button class="setup-secondary" type="button" onclick={() => void controller.cancel()}
			>Cancel authorization</button
		>
	{/if}
</section>
