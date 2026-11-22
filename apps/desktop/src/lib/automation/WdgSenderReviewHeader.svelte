<script lang="ts">
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Shield from '@lucide/svelte/icons/shield';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels as reviewLabelsMap } from '$lib/sender-review/labels';
	import { automationLabels } from './labels';

	interface Props {
		accountId: string;
		accounts: AccountConfig[];
		disabled: boolean;
		onSelectAccount: (id: string) => void;
		onReload: () => void | Promise<void>;
	}

	let { accountId, accounts, disabled, onSelectAccount, onReload }: Props = $props();

	const t = $derived(automationLabels[i18n.locale].reviewPane);
	const reviewT = $derived(reviewLabelsMap[i18n.locale]);
</script>

<div class="flex flex-wrap items-start justify-between gap-4 border-b border-separator pb-3">
	<div class="max-w-2xl space-y-1">
		<h2 class="flex items-center gap-2 font-display text-sm font-semibold text-foreground">
			<Shield class="h-4 w-4 text-accent" aria-hidden="true" />
			<span>{t.title}</span>
		</h2>
		<p class="text-xs text-secondary">{t.subtitle}</p>
		<p class="text-[11px] text-secondary">{reviewT.help}</p>
		<p class="text-[11px] text-secondary">{reviewT.history}</p>
	</div>

	<div class="flex flex-wrap items-center gap-2">
		<label class="flex items-center gap-2">
			<span class="text-[11px] font-medium text-secondary">{reviewT.account}</span>
			<select
				value={accountId}
				{disabled}
				onchange={(e) => onSelectAccount(e.currentTarget.value)}
				class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-45"
			>
				<option value="">{reviewT.choose}</option>
				{#each accounts as acct (acct.account_id)}
					<option value={acct.account_id}>{acct.email_display}</option>
				{/each}
			</select>
		</label>
		<button
			type="button"
			disabled={disabled || !accountId}
			onclick={() => void onReload()}
			class="flex min-h-[36px] items-center gap-2 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
		>
			<RefreshCw class="h-3 w-3" aria-hidden="true" />
			<span>{reviewT.reload}</span>
		</button>
	</div>
</div>
