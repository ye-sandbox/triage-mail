<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import X from '@lucide/svelte/icons/x';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as reviewLabelsMap } from '$lib/sender-review/labels';
	import type { ReviewState, SenderChoice } from '$lib/sender-review/types';
	import type { SenderVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';

	export type FilterTab = 'All' | 'Pending' | 'Declined' | 'Accepted' | 'Known';

	interface Props {
		choices: SenderChoice[];
		visibleChoices: SenderChoice[];
		filterStatus: FilterTab;
		accountLabel: string;
		senderMeta: Record<string, SenderVisualMeta>;
		busy: boolean;
		dirty: boolean;
		loading: boolean;
		more: boolean;
		isConsented: (choice: SenderChoice) => boolean;
		onSetConsent: (choice: SenderChoice, checked: boolean) => void;
		onDecide: (choice: SenderChoice, state: ReviewState) => void | Promise<void>;
		onLoadMore: () => void | Promise<void>;
	}

	let {
		choices,
		visibleChoices,
		filterStatus = $bindable(),
		accountLabel,
		senderMeta,
		busy,
		dirty,
		loading,
		more,
		isConsented,
		onSetConsent,
		onDecide,
		onLoadMore
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].reviewPane);
	const reviewT = $derived(reviewLabelsMap[i18n.locale]);

	function rowKey(choice: SenderChoice): string {
		return `${choice.account_id}:${choice.sender}`;
	}

	function stateBadgeClass(state: ReviewState): string {
		switch (state) {
			case 'Accepted':
				return 'border-status-healthy/40 bg-status-healthy/10 text-status-healthy';
			case 'Pending':
				return 'border-status-warning/40 bg-status-warning/10 text-status-warning';
			case 'Declined':
				return 'border-status-danger/40 bg-status-danger/10 text-status-danger';
			case 'Known':
				return 'border-separator bg-field text-secondary';
		}
	}
</script>

<div class="flex flex-wrap items-center justify-between gap-2">
	<h3 class="font-display text-xs font-semibold text-foreground">{reviewT.choices}</h3>
	<div class="flex flex-wrap items-center gap-2">
		{#each [
			{ key: 'All', label: t.filterAll },
			{ key: 'Pending', label: t.filterPending },
			{ key: 'Declined', label: t.filterDeclined },
			{ key: 'Accepted', label: t.filterAccepted },
			{ key: 'Known', label: t.filterKnown }
		] as tab (tab.key)}
			<button
				type="button"
				onclick={() => (filterStatus = tab.key as FilterTab)}
				class="min-h-[32px] rounded-md border px-3 py-1 text-xs font-medium whitespace-nowrap transition-colors {filterStatus ===
				tab.key
					? 'border-accent bg-selection-soft text-foreground'
					: 'border-separator bg-field text-secondary hover:text-foreground'}"
			>
				{tab.label}
			</button>
		{/each}
	</div>
</div>

{#if visibleChoices.length === 0}
	<div class="rounded-md border border-separator bg-field p-4 text-center text-xs text-secondary">
		{choices.length === 0 ? reviewT.empty : t.noSendersMatch}
	</div>
{:else}
	<div class="flex flex-col divide-y divide-separator rounded-md border border-separator">
		{#each visibleChoices as choice (rowKey(choice))}
			{@const meta = senderMeta[rowKey(choice)]}
			{@const consented = isConsented(choice)}
			<div class="flex flex-col gap-3 p-3 hover:bg-field/40">
				<div class="flex flex-wrap items-start justify-between gap-3">
					<div class="min-w-0 flex-1 space-y-1">
						<div class="flex flex-wrap items-center gap-2">
							<span class="font-medium text-foreground">
								{meta?.sender_name ?? choice.sender}
							</span>
							<span
								class="rounded-md border px-2 py-1 font-mono text-[11px] {stateBadgeClass(choice.state)}"
							>
								{reviewT[choice.state]}
							</span>
						</div>
						<div class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 font-mono text-[11px] text-secondary">
							<span class="truncate">{choice.sender}</span>
							<span class="truncate">{accountLabel}</span>
							{#if typeof meta?.message_count === 'number'}
								<span class="whitespace-nowrap tabular-nums">{t.msgsCount(meta.message_count)}</span>
							{/if}
							{#if meta?.sample_subject}
								<span class="min-w-0 max-w-full truncate font-sans">{meta.sample_subject}</span>
							{/if}
						</div>
					</div>

					<div class="flex shrink-0 flex-wrap items-center gap-2">
						<button
							type="button"
							disabled={busy || dirty || loading || !consented || choice.state === 'Accepted'}
							onclick={() => void onDecide(choice, 'Accepted')}
							class="flex min-h-[34px] items-center gap-1 rounded-md border border-status-healthy/40 bg-status-healthy/10 px-3 py-1 text-xs font-medium whitespace-nowrap text-status-healthy hover:opacity-90 disabled:opacity-45"
						>
							<Check class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{reviewT.accept}</span>
						</button>
						<button
							type="button"
							disabled={busy || dirty || loading || !consented || choice.state === 'Declined'}
							onclick={() => void onDecide(choice, 'Declined')}
							class="flex min-h-[34px] items-center gap-1 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium whitespace-nowrap text-secondary hover:text-status-danger disabled:opacity-45"
						>
							<X class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{reviewT.decline}</span>
						</button>
						<button
							type="button"
							disabled={busy || dirty || loading || !consented || choice.state === 'Pending'}
							onclick={() => void onDecide(choice, 'Pending')}
							class="flex min-h-[34px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium whitespace-nowrap text-secondary hover:text-foreground disabled:opacity-45"
						>
							<span>{reviewT.pending}</span>
						</button>
					</div>
				</div>

				<label class="flex cursor-pointer items-center gap-2 text-[11px] text-secondary">
					<input
						type="checkbox"
						checked={consented}
						onchange={(e) => onSetConsent(choice, e.currentTarget.checked)}
						class="h-3.5 w-3.5 accent-accent"
					/>
					<span>{reviewT.consent}</span>
				</label>
			</div>
		{/each}
	</div>
{/if}

{#if more}
	<div class="flex justify-center">
		<button
			type="button"
			disabled={busy || dirty || loading}
			onclick={() => void onLoadMore()}
			class="min-h-[36px] rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
		>
			{reviewT.more}
		</button>
	</div>
{/if}
