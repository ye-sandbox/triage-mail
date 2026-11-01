<script lang="ts">
	import type { AccountController } from '../../onboarding/controller.svelte';
	import AccountList from '../../onboarding/AccountList.svelte';
	import AccountForm from '../../onboarding/AccountForm.svelte';
	import { i18n } from '../../i18n/index.svelte';

	let {
		controller
	}: {
		controller: AccountController;
	} = $props();

	let adding = $state(false);

	const t = $derived(i18n.t.onboarding);
</script>

<div class="space-y-6" aria-label="Accounts settings">
	<div class="flex items-center justify-between gap-4">
		<div>
			<h2 class="text-lg font-semibold text-[var(--text)]">Accounts</h2>
			<p class="text-sm text-[var(--secondary)]">
				Manage your connected email accounts and authentication methods.
			</p>
		</div>
		{#if !adding && controller.accounts.length > 0}
			<button
				type="button"
				class="rounded-lg bg-[var(--accent)] px-3 py-1.5 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
				disabled={controller.busy}
				onclick={() => (adding = true)}
			>
				+ Add account
			</button>
		{/if}
	</div>

	{#if adding}
		<div class="space-y-4">
			<button
				type="button"
				class="flex items-center gap-2 text-sm text-[var(--accent)] hover:underline disabled:opacity-50"
				disabled={controller.busy}
				onclick={() => (adding = false)}
			>
				← Back to accounts
			</button>
			<div class="rounded-2xl border border-[var(--separator)] bg-[var(--surface)] p-6">
				<AccountForm {controller} />
			</div>
		</div>
	{:else if controller.accounts.length > 0}
		<section aria-label="Connected accounts list" class="space-y-4">
			<AccountList accounts={controller.accounts} />
		</section>
	{:else}
		<div class="space-y-4">
			<p class="text-sm text-[var(--secondary)]">
				{t.connectFirstAccount}
			</p>
			<div class="rounded-2xl border border-[var(--separator)] bg-[var(--surface)] p-6">
				<AccountForm {controller} />
			</div>
		</div>
	{/if}

	{#if controller.error}
		<p role="alert" class="rounded-lg border border-[var(--separator)] p-4 text-sm text-red-500">
			{controller.error}
		</p>
	{/if}
</div>
