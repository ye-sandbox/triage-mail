<script lang="ts">
	import { untrack } from 'svelte';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import Tags from '@lucide/svelte/icons/tags';
	import { toast } from 'svelte-sonner';
	import { labels as catLabelsMap } from '$lib/categories/labels';
	import { nativeCategories } from '$lib/categories/transport';
	import {
		categories,
		type Category,
		type CategoryTransport,
		type SenderOverride
	} from '$lib/categories/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { SettingsFormGuard } from '$lib/settings/guards';
	import type { OverrideVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';

	interface Props {
		transport?: CategoryTransport;
		guard?: SettingsFormGuard;
		accounts?: AccountConfig[];
		overrideMeta?: Record<string, OverrideVisualMeta>;
	}

	let {
		transport = nativeCategories,
		guard = new SettingsFormGuard(),
		accounts = [],
		overrideMeta = {}
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].categoriesPane);
	const catT = $derived(catLabelsMap[i18n.locale]);

	let values = $state.raw<SenderOverride[]>([]);
	let more = $state(false);
	let busy = $state(false);
	let error = $state(false);
	let alive = true;
	let generation = 0;

	const activeOverrides = $derived(values.filter((v) => v.category !== null));

	function metaFor(entry: SenderOverride): OverrideVisualMeta | undefined {
		return overrideMeta[`${entry.account_id}:${entry.sender}`];
	}

	function accountLabelFor(accountId: string): string {
		return accounts.find((a) => a.account_id === accountId)?.email_display ?? catT.unavailable;
	}

	async function load(append = false) {
		const current = ++generation;
		try {
			const result = await transport.list(append ? values.length : 0);
			if (alive && current === generation) {
				values = append ? [...values, ...result] : result;
				more = result.length === 200;
				error = false;
			}
		} catch {
			if (alive && current === generation) error = true;
		}
	}

	$effect(() => {
		alive = true;
		untrack(() => void load());
		return () => {
			alive = false;
			generation++;
			values = [];
			guard.reset();
		};
	});

	async function updateCategory(entry: SenderOverride, nextCategory: Category) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		const snapshot = $state.snapshot(entry);
		try {
			await transport.save({ ...snapshot, category: nextCategory });
			if (alive) {
				await load();
				toast.success(t.updatedToast(entry.sender, catT.category[nextCategory]));
			}
		} catch {
			if (alive) error = true;
		} finally {
			if (alive) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}

	async function resetOverride(entry: SenderOverride) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		const snapshot = $state.snapshot(entry);
		try {
			await transport.save({ ...snapshot, category: null });
			if (alive) {
				await load();
				toast.info(t.removedToast(entry.sender));
			}
		} catch {
			if (alive) error = true;
		} finally {
			if (alive) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}
</script>

<section
	class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4 text-xs"
	aria-label={catT.title}
>
	<div class="flex flex-wrap items-start justify-between gap-3 border-b border-separator pb-3">
		<div class="max-w-2xl space-y-1">
			<h2 class="flex items-center gap-2 font-display text-sm font-semibold text-foreground">
				<Tags class="h-4 w-4 text-accent" aria-hidden="true" />
				<span>{t.title}</span>
			</h2>
			<p class="text-xs text-secondary">{t.subtitle}</p>
			<p class="text-[11px] text-secondary">{catT.help} {catT.local}</p>
			<p class="text-[11px] text-secondary">{catT.consent}</p>
		</div>

		<button
			type="button"
			disabled={busy}
			onclick={() => void load()}
			class="flex min-h-[36px] items-center gap-2 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
		>
			<RefreshCw class="h-3 w-3" aria-hidden="true" />
			<span>{catT.reload}</span>
		</button>
	</div>

	{#if error}
		<p
			role="alert"
			class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger"
		>
			{catT.error}
		</p>
	{/if}

	{#if activeOverrides.length === 0}
		<div class="rounded-md border border-separator bg-field p-4 text-center text-xs text-secondary">
			{catT.empty}
		</div>
	{:else}
		<div class="overflow-x-auto">
			<table class="w-full border-collapse text-left text-xs">
				<thead>
					<tr class="border-b border-separator text-[11px] font-medium text-secondary">
						<th class="py-2 pr-3">{t.colSender}</th>
						<th class="py-2 pr-3">{t.colReason}</th>
						<th class="py-2 pr-3">{t.colAuto}</th>
						<th class="py-2 pr-3">{t.colOverride}</th>
						<th class="py-2 text-right">{t.colAction}</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-separator">
					{#each activeOverrides as entry (`${entry.account_id}:${entry.sender}`)}
						{@const meta = metaFor(entry)}
						<tr class="hover:bg-field/40">
							<td class="py-3 pr-3">
								{#if meta?.sender_name}
									<div class="font-medium text-foreground">{meta.sender_name}</div>
								{/if}
								<div class="font-mono text-[11px] break-all text-secondary">{entry.sender}</div>
								<div class="text-[11px] text-secondary">{accountLabelFor(entry.account_id)}</div>
							</td>
							<td class="py-3 pr-3">
								<span
									class="rounded-md border border-separator bg-field px-2 py-1 font-mono text-[11px] text-foreground"
								>
									{meta?.reason ? catT.reason[meta.reason] : t.provenanceSender}
								</span>
							</td>
							<td class="py-3 pr-3 text-secondary">
								{meta?.auto_category ? catT.category[meta.auto_category] : '—'}
							</td>
							<td class="py-3 pr-3">
								<select
									disabled={busy}
									value={entry.category ?? ''}
									onchange={(e) =>
										void updateCategory(entry, e.currentTarget.value as Category)}
									aria-label={t.overrideSelectAria(entry.sender)}
									class="h-8 rounded-md border border-separator bg-field px-2 text-xs font-medium text-foreground focus:border-accent focus:outline-none disabled:opacity-45"
								>
									{#each categories as cat (cat)}
										<option value={cat}>{catT.category[cat]}</option>
									{/each}
								</select>
							</td>
							<td class="py-3 text-right">
								<button
									type="button"
									disabled={busy}
									onclick={() => void resetOverride(entry)}
									aria-label={`${catT.resetSender}: ${entry.sender}`}
									class="inline-flex min-h-[30px] items-center gap-1 rounded border border-separator bg-surface px-3 py-1 text-[11px] font-medium text-secondary hover:text-foreground disabled:opacity-45"
								>
									<RotateCcw class="h-3 w-3" aria-hidden="true" />
									<span>{t.resetButton}</span>
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if more}
		<div class="flex justify-center">
			<button
				type="button"
				disabled={busy}
				onclick={() => void load(true)}
				class="min-h-[36px] rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
			>
				{catT.more}
			</button>
		</div>
	{/if}
</section>
