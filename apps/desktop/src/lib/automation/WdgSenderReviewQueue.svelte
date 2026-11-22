<script lang="ts">
	import { untrack } from 'svelte';
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
	import WdgSenderChoicesList, { type FilterTab } from './WdgSenderChoicesList.svelte';
	import WdgSenderPreferencesBar from './WdgSenderPreferencesBar.svelte';
	import WdgSenderReviewHeader from './WdgSenderReviewHeader.svelte';

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
		choices.filter((c) => (filterStatus === 'All' ? true : c.state === filterStatus))
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
			const [pref, rows] = await Promise.all([transport.preferences(account), transport.list(account, append ? choices.length : 0)]);
			if (!alive || current !== generation) return;
			value = pref;
			choices = append ? [...choices, ...rows] : rows;
			more = rows.length === 200;
			enabled = pref.enabled;
			hide = pref.hide_in_inbox;
			dirty = false;
			guard.markDirty(false);
			error = false;
			onQueueCountChange?.(choices.filter((c) => c.state === 'Pending' || c.state === 'Declined').length);
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
		const draft: ReviewPreferences = { ...value, enabled, hide_in_inbox: hide };
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
</script>

<section class="flex flex-col gap-4 rounded-lg border border-separator bg-surface p-4 text-xs" aria-label={reviewT.title}>
	<WdgSenderReviewHeader
		{accountId}
		{accounts}
		disabled={busy || dirty || loading}
		onSelectAccount={selectAccount}
		onReload={() => load()}
	/>

	{#if error}
		<p role="alert" class="rounded-md border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-xs text-status-danger">
			{reviewT.error}
		</p>
	{/if}

	{#if value}
		<WdgSenderPreferencesBar
			{enabled}
			{hide}
			{dirty}
			{busy}
			{loading}
			onToggleEnabled={(checked) => {
				enabled = checked;
				markPrefsDirty();
			}}
			onToggleHide={(checked) => {
				hide = checked;
				markPrefsDirty();
			}}
			onSave={savePreferences}
		/>

		{#if value.enabled}
			<p role="status" class="rounded-md border border-status-info/35 bg-status-info/10 px-3 py-2 text-[11px] text-foreground">
				{value.initializing ? reviewT.initializing : reviewT.ready}
			</p>
		{/if}

		<WdgSenderChoicesList
			{choices}
			{visibleChoices}
			bind:filterStatus
			{accountLabel}
			{senderMeta}
			{busy}
			{dirty}
			{loading}
			{more}
			{isConsented}
			onSetConsent={setConsent}
			onDecide={decide}
			onLoadMore={() => load(true)}
		/>
	{/if}
</section>
