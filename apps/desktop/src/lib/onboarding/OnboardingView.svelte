<script lang="ts">
	import type { AccountController } from './controller.svelte';
	import AccountForm from './AccountForm.svelte';
	import LanguageSelector from '../ui/LanguageSelector.svelte';
	import { i18n } from '../i18n/index.svelte';
	import { clearCredentialInputs } from './forms';

	let { controller, enterInbox }: { controller: AccountController; enterInbox: () => void } =
		$props();
	let form = $state<HTMLFormElement>();

	const t = $derived(i18n.t.onboarding);
	const appT = $derived(i18n.t.app);

	$effect(() => {
		const current = form;
		return () => clearCredentialInputs(current);
	});
	function unlock(event: SubmitEvent) {
		event.preventDefault();
		if (!form || controller.busy) return;
		const input = form.elements.namedItem('passphrase') as HTMLInputElement;
		const pending = controller.unlock(input.value);
		input.value = '';
		void pending;
	}
</script>

<main class="min-h-dvh bg-[var(--background)] px-4 py-8 text-[var(--text)] sm:px-8">
	<div class="mx-auto max-w-xl space-y-8">
		<header class="flex items-start justify-between gap-4">
			<div class="space-y-2">
				<p class="text-sm font-semibold text-[var(--accent)]">{appT.title}</p>
				<h1 class="text-3xl font-semibold tracking-tight">
					{controller.phase === 'locked' ? t.unlockTitle : t.accountsTitle}
				</h1>
				<p class="text-sm text-[var(--secondary)]">
					{appT.tagline}
				</p>
			</div>
			<div class="pt-1">
				<LanguageSelector />
			</div>
		</header>
		{#if controller.phase === 'loading'}
			<p role="status">{t.openingMail}</p>
		{:else if controller.phase === 'locked'}
			<form
				bind:this={form}
				onsubmit={unlock}
				class="space-y-4 rounded-2xl border border-[var(--separator)] bg-[var(--surface)] p-6"
				autocomplete="off"
			>
				<p class="text-sm text-[var(--secondary)]">
					{t.vaultLockedHelp}
				</p>
				<label class="block text-sm"
					>{t.vaultPassphraseLabel}
					<input
						class="setup-input mt-2"
						name="passphrase"
						type="password"
						data-credential
						required
						maxlength="4096"
						autocomplete="off"
						disabled={controller.busy}
					/>
				</label>
				<button class="setup-primary" type="submit" disabled={controller.busy}
					>{controller.busy ? t.unlocking : t.unlockVault}</button
				>
				<button
					class="setup-secondary"
					type="button"
					disabled={controller.busy}
					onclick={() => void controller.load()}>{t.retryKeyring}</button
				>
			</form>
		{:else if controller.phase === 'error'}
			<button class="setup-secondary" type="button" onclick={() => void controller.load()}
				>{t.tryAgain}</button
			>
		{:else if controller.phase === 'ready'}
			{#if controller.accounts.length}
				<section aria-label={t.connectedAccounts} class="space-y-4">
					<ul
						class="divide-y divide-[var(--separator)] rounded-2xl border border-[var(--separator)] bg-[var(--surface)] px-6"
					>
						{#each controller.accounts as account (account.account_id)}
							<li class="py-4">
								<p class="break-all font-medium">{account.email_display}</p>
								<p class="mt-1 break-all text-sm text-[var(--secondary)]">
									{account.imap_host}:<span class="tabular-nums">{account.imap_port}</span>
									· {account.auth_method === 'AppPassword'
										? i18n.t.accountForm.methodAppPassword
										: account.auth_method === 'ProtonBridgeLocal'
											? 'Proton Bridge'
											: 'OAuth'}
								</p>
							</li>
						{/each}
					</ul>
					<button
						class="setup-secondary"
						type="button"
						disabled={controller.busy}
						onclick={enterInbox}>{t.openInbox}</button
					>
				</section>
			{:else}
				<p class="text-sm text-[var(--secondary)]">
					{t.connectFirstAccount}
				</p>
			{/if}
			<div class="rounded-2xl border border-[var(--separator)] bg-[var(--surface)] p-6">
				<AccountForm {controller} />
			</div>
		{/if}
		{#if controller.error}<p
				role="alert"
				class="rounded-lg border border-[var(--separator)] p-4 text-sm"
			>
				{controller.error}
			</p>{/if}
	</div>
</main>
