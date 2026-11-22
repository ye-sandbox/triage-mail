<script lang="ts">
	import X from '@lucide/svelte/icons/x';
	import { i18n } from '../i18n/index.svelte';
	import {
		MAX_ALIAS_ADDRESS_CHARS,
		MAX_ALIAS_LABEL_CHARS,
		validateAlias,
		type AccountAlias,
		type AliasIssue
	} from './aliases';

	type Props = {
		accountId: string;
		aliases?: AccountAlias[];
		failed?: boolean;
		onSave: (aliases: AccountAlias[]) => Promise<boolean>;
	};
	let { accountId, aliases = [], failed = false, onSave }: Props = $props();

	const t = $derived(i18n.t.onboarding);
	let address = $state('');
	let label = $state('');
	let issue = $state<AliasIssue | null>(null);
	let saving = $state(false);

	const issueText = $derived(
		issue === 'duplicate'
			? t.aliasDuplicate
			: issue === 'limit'
				? t.aliasLimit
				: issue === 'invalid'
					? t.aliasInvalid
					: ''
	);

	async function add(event: Event) {
		event.preventDefault();
		if (saving) return;
		const result = validateAlias(address, label, aliases);
		if ('issue' in result) {
			issue = result.issue;
			return;
		}
		issue = null;
		saving = true;
		try {
			if (await onSave([...aliases, result.alias])) {
				address = '';
				label = '';
			}
		} finally {
			saving = false;
		}
	}

	async function remove(target: string) {
		if (saving) return;
		saving = true;
		issue = null;
		try {
			await onSave(aliases.filter((a) => a.address !== target));
		} finally {
			saving = false;
		}
	}
</script>

<div class="flex flex-col gap-2 border-t border-separator px-4 py-3 text-xs">
	<div class="font-medium text-foreground">{t.aliasesTitle}</div>
	<p class="leading-relaxed text-secondary">{t.aliasesHelp}</p>
	{#if aliases.length === 0}
		<p class="text-secondary">{t.aliasEmpty}</p>
	{:else}
		<ul class="flex flex-col gap-1" data-testid="alias-list-{accountId}">
			{#each aliases as alias (alias.address)}
				<li class="flex items-center justify-between gap-2 rounded-md bg-field px-2 py-1">
					<span class="truncate text-foreground">
						{alias.address}
						{#if alias.label}<span class="text-secondary"> · {alias.label}</span>{/if}
					</span>
					<button
						type="button"
						aria-label="{t.aliasRemove} {alias.address}"
						disabled={saving}
						onclick={() => void remove(alias.address)}
						class="grid size-8 shrink-0 place-items-center rounded-md text-secondary hover:bg-selection-soft disabled:opacity-50"
					>
						<X class="size-4" aria-hidden="true" />
					</button>
				</li>
			{/each}
		</ul>
	{/if}
	<form class="flex flex-wrap items-center gap-2" onsubmit={add}>
		<input
			type="email"
			aria-label={t.aliasAddressLabel}
			placeholder={t.aliasAddressLabel}
			maxlength={MAX_ALIAS_ADDRESS_CHARS}
			bind:value={address}
			class="h-8 w-56 rounded-md border border-separator bg-field px-2 text-xs text-foreground"
		/>
		<input
			type="text"
			aria-label={t.aliasNameLabel}
			placeholder={t.aliasNameLabel}
			maxlength={MAX_ALIAS_LABEL_CHARS}
			bind:value={label}
			class="h-8 w-40 rounded-md border border-separator bg-field px-2 text-xs text-foreground"
		/>
		<button
			type="submit"
			disabled={saving || !address.trim()}
			class="h-8 rounded-md border border-separator bg-surface px-3 text-xs font-medium text-foreground hover:bg-field disabled:opacity-50"
			>{t.aliasAdd}</button
		>
	</form>
	{#if issueText}
		<p role="alert" class="text-status-warning">{issueText}</p>
	{:else if failed}
		<p role="alert" class="text-status-warning">{t.aliasSaveError}</p>
	{/if}
</div>
