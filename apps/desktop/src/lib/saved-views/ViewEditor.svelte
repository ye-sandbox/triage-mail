<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import type { SavedView, ExtractionKind } from './types';
	import type { AccountConfig } from '../onboarding/types';
	let {
		draft: initial,
		accounts,
		busy,
		save,
		cancel,
		semantic
	}: {
		draft: SavedView;
		accounts: AccountConfig[];
		busy: boolean;
		save: () => void;
		cancel: () => void;
		semantic: boolean;
	} = $props();
	// INVARIANT: Deferred input callbacks retain this object when parent teardown clears its draft.
	const draft = untrack(() => initial);
	const t = $derived(labels[i18n.locale]);
	const boolFields = ['is_read', 'is_starred', 'has_attachments'] as const;
	const boolLabels = $derived([t.read, t.starred, t.attachments]);
	const kinds: ExtractionKind[] = ['Boleto', 'Invoice', 'Pix', 'Tracking', 'Otp', 'Unsubscribe'];
	function toggleAccount(id: string, checked: boolean) {
		draft.accounts = checked ? [...draft.accounts, id] : draft.accounts.filter((a) => a !== id);
	}
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		save();
	}}
	class="space-y-4"
>
	<fieldset disabled={busy} class="space-y-4">
		<label class="block"
			>{t.name}<input required maxlength="120" bind:value={draft.name} class="field" /></label
		>
		<label class="block"
			>{t.query}<input maxlength="4096" bind:value={draft.query} class="field" /></label
		>
		<label class="block"
			>{t.mode}<select bind:value={draft.mode} class="field">
				<option value="Lexical">{t.words}</option>
				<option value="Semantic" disabled={!semantic}>{t.semantic}</option>
				<option value="Hybrid" disabled={!semantic}>{t.hybrid}</option>
			</select></label
		>
		{#if draft.scope && draft.scope !== 'GlobalAllMail'}<label class="flex gap-2"
				><input
					type="checkbox"
					checked
					onchange={(e) => {
						if (!e.currentTarget.checked) draft.scope = null;
					}}
				/>{t.scope}</label
			>{/if}
		<fieldset class="space-y-2">
			<legend>{t.accounts} — {draft.accounts.length === 0 ? t.all : draft.accounts.length}</legend>
			{#each accounts as account (account.account_id)}<label class="flex gap-2"
					><input
						type="checkbox"
						checked={draft.accounts.includes(account.account_id)}
						onchange={(e) => toggleAccount(account.account_id, e.currentTarget.checked)}
					/>{account.email_display}</label
				>{/each}
			{#each draft.accounts.filter((id) => !accounts.some((a) => a.account_id === id)) as id (id)}<label
					class="flex gap-2"
					><input
						type="checkbox"
						checked
						onchange={() => toggleAccount(id, false)}
					/>{t.missing}</label
				>{/each}
		</fieldset>
		{#each boolFields as field, index}<label class="block"
				>{boolLabels[index]}<select
					value={draft.filters[field] === null ? '' : String(draft.filters[field])}
					onchange={(e) =>
						(draft.filters[field] =
							e.currentTarget.value === '' ? null : e.currentTarget.value === 'true')}
					class="field"
					><option value="">{t.either}</option><option value="true">{t.yes}</option><option
						value="false">{t.no}</option
					></select
				></label
			>{/each}
		<label class="block"
			>{t.tags}<textarea
				value={draft.filters.tags.join('\n')}
				oninput={(e) =>
					(draft.filters.tags = e.currentTarget.value.split('\n').filter((v) => v.trim()))}
				class="field"
			></textarea></label
		>
		<label class="block"
			>{t.sender}<input
				type="email"
				value={draft.filters.sender ?? ''}
				oninput={(e) => (draft.filters.sender = e.currentTarget.value.trim().toLowerCase() || null)}
				class="field"
			/></label
		>
		<label class="block"
			>{t.domain}<input
				value={draft.filters.sender_domain ?? ''}
				oninput={(e) =>
					(draft.filters.sender_domain = e.currentTarget.value.trim().toLowerCase() || null)}
				class="field"
			/></label
		>
		<label class="block"
			>{t.extraction}<select
				value={draft.filters.extraction ?? ''}
				onchange={(e) =>
					(draft.filters.extraction = (e.currentTarget.value || null) as ExtractionKind | null)}
				class="field"
				><option value="">{t.either}</option>{#each kinds as kind}<option value={kind}
						>{t.kind[kind]}</option
					>{/each}</select
			></label
		>
		<label class="block"
			>{t.from}<input
				type="date"
				value={draft.filters.due_from ?? ''}
				onchange={(e) => (draft.filters.due_from = e.currentTarget.value || null)}
				class="field"
			/></label
		>
		<label class="block"
			>{t.to}<input
				type="date"
				value={draft.filters.due_to ?? ''}
				onchange={(e) => (draft.filters.due_to = e.currentTarget.value || null)}
				class="field"
			/></label
		>
		<label class="flex gap-2"
			><input
				type="checkbox"
				checked={draft.filters.upcoming_days !== null}
				onchange={(e) => (draft.filters.upcoming_days = e.currentTarget.checked ? 30 : null)}
			/>{t.upcoming}</label
		>
		<p class="text-sm text-[var(--secondary)]">{t.help}</p>
		<div class="flex gap-4">
			<button type="submit" class="rounded bg-[var(--accent)] px-4 py-2 text-white">{t.save}</button
			><button type="button" onclick={cancel}>{t.cancel}</button>
		</div>
	</fieldset>
</form>

<style>
	.field {
		display: block;
		width: 100%;
		border: 1px solid var(--separator);
		border-radius: 0.25rem;
		background: var(--surface);
		padding: 0.5rem;
	}
</style>
