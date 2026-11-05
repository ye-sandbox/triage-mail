<script lang="ts">
	import { transfer, type TransferTransport } from './transport';
	import { labels } from './labels';
	import { i18n } from '../i18n/index.svelte';
	import { SettingsFormGuard } from '../settings/guards';
	import { clearCredentialInputs } from '../onboarding/forms';
	let {
		restore = false,
		transport = transfer,
		guard = new SettingsFormGuard(),
		ondone = () => {},
		onbusy = (_busy: boolean) => {}
	}: {
		restore?: boolean;
		transport?: TransferTransport;
		guard?: SettingsFormGuard;
		ondone?: () => void;
		onbusy?: (busy: boolean) => void;
	} = $props();
	let form = $state<HTMLFormElement>();
	let busy = $state(false);
	let consent = $state(false);
	let message = $state('');
	let finished = $state(false);
	let active = true;
	const t = $derived(labels[i18n.locale]);
	$effect(() => {
		active = true;
		const current = form;
		return () => {
			active = false;
			clearCredentialInputs(current);
			guard.reset();
		};
	});
	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!form || busy || guard.isBlocked() || !consent || finished) return;
		const secret = form.elements.namedItem('passphrase') as HTMLInputElement;
		const confirm = form.elements.namedItem('confirmation') as HTMLInputElement;
		if (secret.value.length < 12 || secret.value !== confirm.value) {
			message = t.mismatch;
			return;
		}
		busy = true;
		onbusy(true);
		guard.setBusy(true);
		message = '';
		try {
			const path = await transport.choose(!restore);
			if (!active || !path) return;
			const pending = restore
				? transport.restore(path, secret.value, consent)
				: transport.backup(path, secret.value);
			clearCredentialInputs(form);
			await pending;
			if (active) {
				message = restore ? t.restored : t.done;
				finished = restore;
				consent = false;
				guard.markDirty(false);
			}
		} catch {
			if (active) message = t.error;
		} finally {
			clearCredentialInputs(form);
			if (active) {
				busy = false;
				onbusy(false);
				guard.setBusy(false);
				guard.markDirty(false);
			}
		}
	}
</script>

<form
	bind:this={form}
	onsubmit={submit}
	oninput={() => guard.markDirty()}
	autocomplete="off"
	class="space-y-4"
>
	<h3 class="font-semibold">{restore ? t.restore : t.backup}</h3>
	<p class="text-sm text-[var(--secondary)]">{t.help}</p>
	<p class="text-sm text-[var(--secondary)]">{t.recovery}</p>
	<p class="text-sm text-[var(--secondary)]">{t.newFile}</p>
	{#if !finished}
		<label class="block text-sm"
			>{t.pass}<input
				name="passphrase"
				type="password"
				data-credential
				required
				minlength="12"
				maxlength="4096"
				autocomplete="off"
				disabled={busy}
				class="setup-input mt-2"
			/></label
		>
		<label class="block text-sm"
			>{t.confirm}<input
				name="confirmation"
				type="password"
				data-credential
				required
				minlength="12"
				maxlength="4096"
				autocomplete="off"
				disabled={busy}
				class="setup-input mt-2"
			/></label
		>
		<label class="flex gap-2 text-sm"
			><input type="checkbox" bind:checked={consent} disabled={busy} />{t.consent}</label
		>
		<button class="setup-primary" type="submit" disabled={busy || !consent}
			>{busy ? t.busy : restore ? t.restore : t.backup}</button
		>
	{:else}
		<button class="setup-primary" type="button" onclick={ondone}>{t.continue}</button>
	{/if}
	{#if message}<p role="status" class="text-sm">{message}</p>{/if}
</form>
