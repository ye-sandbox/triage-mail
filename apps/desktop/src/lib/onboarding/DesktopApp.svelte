<script lang="ts">
	import { untrack } from 'svelte';
	import BackupForm from '../backup/BackupForm.svelte';
	import { transfer } from '../backup/transport';
	import { labels } from '../backup/labels';
	import { i18n } from '../i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { nativeAccountTransport } from './transport';
	import type { AccountTransport } from './types';
	import { AccountController } from './controller.svelte';
	import OnboardingView from './OnboardingView.svelte';
	import InboxSession from './InboxSession.svelte';

	let {
		transport = nativeAccountTransport,
		available = nativeAvailable
	}: { transport?: AccountTransport; available?: () => boolean } = $props();

	const accounts = new AccountController(untrack(() => transport));

	let recovery = $state(false);
	let recoveryBusy = $state(false);
	$effect(() => {
		let active = true;
		async function start() {
			if (available() && transport === nativeAccountTransport) {
				try {
					if (await transfer.available()) {
						if (active) recovery = true;
						return;
					}
				} catch {
					/* Older native hosts retain normal startup. */
				}
			}
			if (active) await accounts.load(available());
		}
		void start();
		return () => {
			active = false;
			accounts.dispose();
		};
	});
</script>

{#if recovery}
	<main class="h-full overflow-auto bg-[var(--background)] p-8 text-[var(--text)]">
		<div class="mx-auto max-w-xl space-y-8">
			<h1 class="text-xl font-semibold">{labels[i18n.locale].welcome}</h1>
			<button
				type="button"
				class="setup-secondary"
				disabled={recoveryBusy}
				onclick={() => {
					recovery = false;
					void accounts.load();
				}}>{labels[i18n.locale].continue}</button
			>
			<BackupForm
				restore
				onbusy={(busy) => {
					recoveryBusy = busy;
				}}
				ondone={() => {
					recovery = false;
					void accounts.load();
				}}
			/>
		</div>
	</main>
{:else if accounts.phase === 'browser'}
	<InboxSession connected={false} />
{:else if accounts.phase === 'ready' && accounts.accounts.length}
	<InboxSession connected={true} accountController={accounts} />
{:else}
	<OnboardingView controller={accounts} />
{/if}
