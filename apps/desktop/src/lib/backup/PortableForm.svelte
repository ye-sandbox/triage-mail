<script lang="ts">
	import { transfer, type Definitions, type Mapping, type TransferTransport } from './transport';
	import { labels } from './labels';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountConfig } from '../onboarding/types';
	import { SettingsFormGuard } from '../settings/guards';
	let {
		accounts = [],
		guard = new SettingsFormGuard(),
		transport = transfer,
		onbusy = (_busy: boolean) => {}
	}: {
		accounts?: AccountConfig[];
		guard?: SettingsFormGuard;
		transport?: TransferTransport;
		onbusy?: (busy: boolean) => void;
	} = $props();
	let preview = $state<Definitions | null>(null);
	let mapping = $state<Mapping>({ accounts: {}, destinations: {} });
	let destinations = $state<{ id: string; name: string }[]>([]);
	let path = '';
	let consent = $state(false);
	let busy = $state(false);
	let message = $state('');
	let active = true;
	const t = $derived(labels[i18n.locale]);
	const complete = $derived(
		preview &&
			preview.accounts.every((id) => mapping.accounts[id]) &&
			preview.destinations.every((id) => mapping.destinations[id])
	);
	$effect(() => () => {
		active = false;
		preview = null;
		path = '';
		mapping = { accounts: {}, destinations: {} };
		guard.reset();
	});
	async function run(action: 'export' | 'preview' | 'import') {
		if (busy || guard.isBlocked()) return;
		busy = true;
		onbusy(true);
		guard.setBusy(true);
		message = '';
		try {
			if (action === 'import') {
				if (!preview || !consent || !complete) return;
				await transport.import(path, $state.snapshot(mapping), $state.snapshot(preview), consent);
				window.dispatchEvent(new Event('triage-definitions-imported'));
				if (active) {
					preview = null;
					path = '';
					consent = false;
					guard.markDirty(false);
					message = t.done;
				}
			} else {
				const selected = await transport.choose(action === 'export');
				if (!active || !selected) return;
				if (action === 'export') {
					await transport.export(selected);
					if (active) message = t.done;
				} else {
					const value = await transport.preview(selected);
					const targets = await transport.destinations();
					if (active) {
						preview = value;
						destinations = targets;
						path = selected;
						mapping = { accounts: {}, destinations: {} };
						consent = false;
						guard.markDirty(true);
					}
				}
			}
		} catch {
			if (active) message = t.error;
		} finally {
			if (active) {
				busy = false;
				onbusy(false);
				guard.setBusy(false);
			}
		}
	}
</script>

<section class="space-y-4">
	<p class="text-sm text-[var(--secondary)]">{t.portable}</p>
	<div class="flex flex-wrap gap-4">
		<button class="setup-secondary" type="button" disabled={busy} onclick={() => void run('export')}
			>{t.export}</button
		>
		<button
			class="setup-secondary"
			type="button"
			disabled={busy}
			onclick={() => void run('preview')}>{t.import}</button
		>
	</div>
	{#if preview}
		<h3 class="font-semibold">{t.review}</h3>
		<pre
			class="max-h-64 overflow-auto whitespace-pre-wrap break-all rounded border border-[var(--separator)] p-4 text-xs">{JSON.stringify(
				preview,
				null,
				2
			)}</pre>
		<h3 class="font-semibold">{t.mapping}</h3>
		{#each preview.accounts as id}
			<label class="block text-sm"
				>{id}<select
					class="setup-input mt-2"
					bind:value={mapping.accounts[id]}
					disabled={busy}
					onchange={() => {
						consent = false;
						guard.markDirty(true);
					}}
					><option value="">{t.choose}</option>{#each accounts as account}<option
							value={account.account_id}>{account.email_display}</option
						>{/each}</select
				></label
			>
		{/each}
		{#each preview.destinations as id}
			<label class="block text-sm"
				>{id}<select
					class="setup-input mt-2"
					bind:value={mapping.destinations[id]}
					disabled={busy}
					onchange={() => {
						consent = false;
						guard.markDirty(true);
					}}
					><option value="">{t.choose}</option>{#each destinations as destination}<option
							value={destination.id}>{destination.name}</option
						>{/each}</select
				></label
			>
		{/each}
		<label class="flex gap-2 text-sm"
			><input type="checkbox" bind:checked={consent} disabled={busy} />{t.approve}</label
		>
		<button
			class="setup-primary"
			type="button"
			disabled={busy || !complete || !consent}
			onclick={() => void run('import')}>{busy ? t.busy : t.apply}</button
		>
	{/if}
	{#if message}<p role="status" class="text-sm">{message}</p>{/if}
</section>
