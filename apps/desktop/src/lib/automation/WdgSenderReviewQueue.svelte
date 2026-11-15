<script lang="ts">
	import { untrack } from 'svelte';
	import { Switch } from 'bits-ui';
	import { Check, RefreshCw, Shield, X } from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels as reviewLabelsMap } from '$lib/sender-review/labels';
	import { nativeReview } from '$lib/sender-review/transport';
	import type {
		ReviewPreferences,
		ReviewState,
		ReviewTransport,
		SenderChoice
	} from '$lib/sender-review/types';
	import { SettingsFormGuard } from '$lib/settings/guards';
	import type { SenderVisualMeta } from './automation.fixtures';
	import { automationLabels } from './labels';

	type FilterTab = 'All' | 'Pending' | 'Declined' | 'Accepted' | 'Known';

	interface Props {
		transport?: ReviewTransport;
		guard?: SettingsFormGuard;
		accounts?: AccountConfig[];
		senderMeta?: Record<string, SenderVisualMeta>;
		onQueueCountChange?: (count: number) => void;
	}

	let {
		transport = nativeReview,
		guard = new SettingsFormGuard(),
		accounts = [],
		senderMeta = {},
		onQueueCountChange
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].reviewPane);
	const reviewT = $derived(reviewLabelsMap[i18n.locale]);

	let accountId = $state('');
	let value = $state<ReviewPreferences | null>(null);
	let choices = $state.raw<SenderChoice[]>([]);
	let consentedRevisions = $state<Record<string, number>>({});
	let filterStatus = $state<FilterTab>('All');
	let more = $state(false);
	let enabled = $state(false);
	let hide = $state(false);
	let dirty = $state(false);
	let busy = $state(false);
	let loading = $state(false);
	let error = $state(false);
	let alive = true;
	let generation = 0;

	const accountLabel = $derived(
		accounts.find((a) => a.account_id === accountId)?.email_display ?? accountId
	);

	const visibleChoices = $derived(
		choices.filter((c) => {
			if (filterStatus === 'All') return true;
			return c.state === filterStatus;
		})
	);

	function rowKey(choice: SenderChoice): string {
		return `${choice.account_id}:${choice.sender}`;
	}

	function isConsented(choice: SenderChoice): boolean {
		return consentedRevisions[rowKey(choice)] === choice.revision;
	}

	function setConsent(choice: SenderChoice, checked: boolean) {
		const key = rowKey(choice);
		if (checked) {
			consentedRevisions = { ...consentedRevisions, [key]: choice.revision };
		} else {
			const next = { ...consentedRevisions };
			delete next[key];
			consentedRevisions = next;
		}
	}

	async function load(append = false) {
		const current = ++generation;
		const account = accountId;
		if (!account) return;
		loading = true;
		try {
			const [pref, rows] = await Promise.all([
				transport.preferences(account),
				transport.list(account, append ? choices.length : 0)
			]);
			if (alive && current === generation) {
				value = pref;
				choices = append ? [...choices, ...rows] : rows;
				more = rows.length === 200;
				enabled = pref.enabled;
				hide = pref.hide_in_inbox;
				dirty = false;
				guard.markDirty(false);
				error = false;
				onQueueCountChange?.(
					choices.filter((c) => c.state === 'Pending' || c.state === 'Declined').length
				);
			}
		} catch {
			if (alive && current === generation) error = true;
		} finally {
			if (alive && current === generation) loading = false;
		}
	}

	function selectAccount(id: string) {
		if (dirty || busy) return;
		generation++;
		accountId = id;
		value = null;
		choices = [];
		if (id) void load();
	}

	$effect(() => {
		const ids = accounts.map((a) => a.account_id);
		untrack(() => {
			if (!accountId && ids[0]) {
				selectAccount(ids[0]);
			} else if (accountId && !ids.includes(accountId)) {
				generation++;
				accountId = '';
				value = null;
				choices = [];
				dirty = false;
				guard.markDirty(false);
			}
		});
	});

	$effect(() => {
		alive = true;
		const timer = setInterval(() => {
			if (value?.initializing && !busy && !dirty && !loading) void load();
		}, 1500);
		return () => {
			clearInterval(timer);
			alive = false;
			generation++;
			choices = [];
			value = null;
			guard.reset();
		};
	});

	function markPrefsDirty() {
		generation++;
		dirty = true;
		guard.markDirty(true);
	}

	async function writeOperation(op: () => Promise<unknown>, onSuccess?: () => void) {
		if (busy || loading) return;
		busy = true;
		guard.setBusy(true);
		try {
			await op();
			if (alive) {
				await load();
				onSuccess?.();
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

	async function savePreferences() {
		if (!value) return;
		const draft: ReviewPreferences = {
			...value,
			enabled,
			hide_in_inbox: hide
		};
		await writeOperation(
			() => transport.savePreferences(draft),
			() => toast.success(t.preferencesSavedToast)
		);
	}

	async function decide(choice: SenderChoice, state: ReviewState) {
		if (!isConsented(choice)) return;
		const meta = senderMeta[rowKey(choice)];
		const displayName = meta?.sender_name ?? choice.sender;
		await writeOperation(
			() => transport.save({ ...choice, state }),
			() => toast.success(t.decisionToast(displayName, reviewT[state]))
		);
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

<section
	class="flex flex-col gap-4 rounded-lg border border-separator bg-surface p-4 text-xs"
	aria-label={reviewT.title}
>
	<div class="flex flex-wrap items-start justify-between gap-4 border-b border-separator pb-3">
		<div class="max-w-2xl space-y-1">
			<h2 class="flex items-center gap-2 font-display text-sm font-semibold text-foreground">
				<Shield class="h-4 w-4 text-accent" />
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
					disabled={busy || dirty || loading}
					onchange={(e) => selectAccount(e.currentTarget.value)}
					class="h-9 rounded-md border border-separator bg-field px-2.5 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-45"
				>
					<option value="">{reviewT.choose}</option>
					{#each accounts as acct (acct.account_id)}
						<option value={acct.account_id}>{acct.email_display}</option>
					{/each}
				</select>
			</label>
			<button
				type="button"
				disabled={busy || dirty || loading || !accountId}
				onclick={() => void load()}
				class="flex min-h-[36px] items-center gap-1.5 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
			>
				<RefreshCw class="h-3 w-3" />
				<span>{reviewT.reload}</span>
			</button>
		</div>
	</div>

	{#if error}
		<p
			role="alert"
			class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger"
		>
			{reviewT.error}
		</p>
	{/if}

	{#if value}
		<fieldset disabled={busy || loading} class="flex flex-col gap-3 border-0 p-0">
			<div
				class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-separator bg-field p-3"
			>
				<div class="flex flex-wrap items-center gap-4">
					<label class="flex cursor-pointer items-center gap-2">
						<Switch.Root
							checked={enabled}
							onCheckedChange={(checked) => {
								enabled = checked;
								markPrefsDirty();
							}}
							aria-label={t.toggleEnableAria}
							class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
						>
							<Switch.Thumb
								class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
							/>
						</Switch.Root>
						<span class="text-xs font-medium text-foreground">{reviewT.enable}</span>
					</label>

					<label class="flex cursor-pointer items-center gap-2">
						<Switch.Root
							checked={hide}
							onCheckedChange={(checked) => {
								hide = checked;
								markPrefsDirty();
							}}
							aria-label={t.toggleHideAria}
							class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
						>
							<Switch.Thumb
								class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
							/>
						</Switch.Root>
						<span class="text-xs font-medium text-foreground">{reviewT.hide}</span>
					</label>
				</div>

				<button
					type="button"
					disabled={!dirty || busy || loading}
					onclick={() => void savePreferences()}
					class="min-h-[34px] rounded-md bg-accent px-3.5 py-1 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-45"
				>
					{busy ? reviewT.saving : reviewT.save}
				</button>
			</div>

			<p class="text-[11px] text-secondary">{reviewT.hideHelp}</p>
		</fieldset>

		{#if value.enabled}
			<p
				role="status"
				class="rounded-md border border-status-info/35 bg-status-info/10 px-3 py-2 text-[11px] text-foreground"
			>
				{value.initializing ? reviewT.initializing : reviewT.ready}
			</p>
		{/if}

		<div class="flex flex-wrap items-center justify-between gap-2">
			<h3 class="font-display text-xs font-semibold text-foreground">{reviewT.choices}</h3>
			<div class="flex flex-wrap items-center gap-1.5">
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
						class="min-h-[32px] rounded-md border px-2.5 py-1 text-xs font-medium whitespace-nowrap transition-colors {filterStatus ===
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
					<div class="flex flex-col gap-2.5 p-3 hover:bg-field/40">
						<div class="flex flex-wrap items-start justify-between gap-3">
							<div class="min-w-0 flex-1 space-y-1">
								<div class="flex flex-wrap items-center gap-2">
									<span class="font-medium text-foreground">
										{meta?.sender_name ?? choice.sender}
									</span>
									<span
										class="rounded-md border px-2 py-0.5 font-mono text-[11px] {stateBadgeClass(
											choice.state
										)}"
									>
										{reviewT[choice.state]}
									</span>
								</div>
								<div class="flex flex-wrap items-center gap-2 font-mono text-[11px] text-secondary">
									<span>{choice.sender}</span>
									<span aria-hidden="true">·</span>
									<span>{accountLabel}</span>
									{#if typeof meta?.message_count === 'number'}
										<span aria-hidden="true">·</span>
										<span class="tabular-nums">{t.msgsCount(meta.message_count)}</span>
									{/if}
									{#if meta?.sample_subject}
										<span aria-hidden="true">·</span>
										<span class="truncate font-sans">{meta.sample_subject}</span>
									{/if}
								</div>
							</div>

							<div class="flex shrink-0 flex-wrap items-center gap-1.5">
								<button
									type="button"
									disabled={busy || dirty || loading || !consented || choice.state === 'Accepted'}
									onclick={() => void decide(choice, 'Accepted')}
									class="flex min-h-[34px] items-center gap-1 rounded-md border border-status-healthy/40 bg-status-healthy/10 px-3 py-1 text-xs font-medium whitespace-nowrap text-status-healthy hover:opacity-90 disabled:opacity-45"
								>
									<Check class="h-3.5 w-3.5" />
									<span>{reviewT.accept}</span>
								</button>
								<button
									type="button"
									disabled={busy || dirty || loading || !consented || choice.state === 'Declined'}
									onclick={() => void decide(choice, 'Declined')}
									class="flex min-h-[34px] items-center gap-1 rounded-md border border-separator bg-field px-3 py-1 text-xs font-medium whitespace-nowrap text-secondary hover:text-status-danger disabled:opacity-45"
								>
									<X class="h-3.5 w-3.5" />
									<span>{reviewT.decline}</span>
								</button>
								<button
									type="button"
									disabled={busy || dirty || loading || !consented || choice.state === 'Pending'}
									onclick={() => void decide(choice, 'Pending')}
									class="flex min-h-[34px] items-center gap-1 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium whitespace-nowrap text-secondary hover:text-foreground disabled:opacity-45"
								>
									<span>{reviewT.pending}</span>
								</button>
							</div>
						</div>

						<label class="flex cursor-pointer items-center gap-2 text-[11px] text-secondary">
							<input
								type="checkbox"
								checked={consented}
								onchange={(e) => setConsent(choice, e.currentTarget.checked)}
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
					onclick={() => void load(true)}
					class="min-h-[36px] rounded-md border border-separator bg-field px-4 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-45"
				>
					{reviewT.more}
				</button>
			</div>
		{/if}
	{/if}
</section>
