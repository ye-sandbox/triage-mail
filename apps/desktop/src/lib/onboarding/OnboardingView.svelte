<script lang="ts">
	import type { AccountController } from './controller.svelte';
	import AccountForm from './AccountForm.svelte';
	import { clearCredentialInputs } from './forms';
	let { controller, enterInbox }: { controller: AccountController; enterInbox: () => void } =
		$props();
	let form = $state<HTMLFormElement>();
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
		<header class="space-y-2">
			<p class="text-sm font-semibold text-[var(--accent)]">Triage Mail</p>
			<h1 class="text-3xl font-semibold tracking-tight">
				{controller.phase === 'locked' ? 'Unlock your mail' : 'Your accounts'}
			</h1>
			<p class="text-sm text-[var(--secondary)]">
				Local storage. Private credentials. Your choice of provider.
			</p>
		</header>
		{#if controller.phase === 'loading'}
			<p role="status">Opening local mail…</p>
		{:else if controller.phase === 'locked'}
			<form
				bind:this={form}
				onsubmit={unlock}
				class="space-y-4 rounded-2xl border border-[var(--separator)] bg-[var(--surface)] p-6"
				autocomplete="off"
			>
				<p class="text-sm text-[var(--secondary)]">
					Your system keyring is unavailable or the vault is locked. Enter your local vault
					passphrase. On first use, this creates an encrypted fallback vault.
				</p>
				<label class="block text-sm"
					>Vault passphrase
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
					>{controller.busy ? 'Unlocking…' : 'Unlock vault'}</button
				>
				<button
					class="setup-secondary"
					type="button"
					disabled={controller.busy}
					onclick={() => void controller.load()}>Retry system keyring</button
				>
			</form>
		{:else if controller.phase === 'error'}
			<button class="setup-secondary" type="button" onclick={() => void controller.load()}
				>Try opening mail again</button
			>
		{:else if controller.phase === 'ready'}
			{#if controller.accounts.length}
				<section aria-label="Connected accounts" class="space-y-4">
					<ul
						class="divide-y divide-[var(--separator)] rounded-2xl border border-[var(--separator)] bg-[var(--surface)] px-6"
					>
						{#each controller.accounts as account (account.account_id)}
							<li class="py-4">
								<p class="break-all font-medium">{account.email_display}</p>
								<p class="mt-1 break-all text-sm text-[var(--secondary)]">
									{account.imap_host}:<span class="tabular-nums">{account.imap_port}</span>
									· {account.auth_method === 'AppPassword'
										? 'App password'
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
						onclick={enterInbox}>Open inbox</button
					>
				</section>
			{:else}
				<p class="text-sm text-[var(--secondary)]">
					Connect your first account to start bringing your mail together.
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
