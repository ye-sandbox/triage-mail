<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { InboxController } from '../inbox/controller.svelte';
	import type { ReaderTransport } from '../reader/types';
	import type { ConversationMember, ConversationRow, MutationStatus } from './types';
	import type { AccountConfig } from '../onboarding/types';
	import SelectionOverview from '../inbox/SelectionOverview.svelte';
	import ConversationActions from './ConversationActions.svelte';
	import { labels } from './labels';
	let {
		inbox,
		row,
		transport,
		accounts,
		disabled = false
	}: {
		inbox: InboxController;
		row: ConversationRow;
		transport?: ReaderTransport;
		accounts: AccountConfig[];
		disabled?: boolean;
	} = $props();
	const t = $derived(labels[i18n.locale]);
	const accountLabel = $derived(
		accounts.find((a) => a.account_id === row.representative.account_id)?.email_display ??
			row.representative.account_id
	);
	let members = $state.raw<ConversationMember[]>([]);
	let statuses = $state.raw<MutationStatus[]>([]);
	let total = $state(0);
	let selectedId = $state<string | null>(null);
	let loading = $state(false);
	let error = $state(false);
	let version = 0;
	let alive = true;
	const selected = $derived(members.find((m) => m.message.id === selectedId)?.message);
	async function load(reset = false) {
		const api = inbox.conversationTransport;
		if (!api || (loading && !reset)) return;
		const generation = ++version;
		if (reset) {
			members = [];
			selectedId = null;
			statuses = [];
		}
		loading = true;
		error = false;
		try {
			const page = await api.members(
				inbox.conversationQuery(),
				inbox.conversationContext(),
				row.conversation_id,
				members.length
			);
			const state = await api.status(page.rows.map((m) => m.message.id));
			if (alive && generation === version) {
				members = [...members, ...page.rows];
				total = page.total;
				statuses = [...statuses, ...state];
			}
		} catch {
			if (alive && generation === version) error = true;
		} finally {
			if (alive && generation === version) loading = false;
		}
	}
	$effect(() => {
		inbox.conversationGeneration;
		row.conversation_id;
		inbox.badgeRevision;
		untrack(() => void load(true));
	});
	$effect(() => {
		const api = inbox.conversationTransport;
		const ids = members.map((m) => m.message.id);
		if (!api || !ids.length) return;
		let active = true;
		const timer = setInterval(() => {
			void api
				.status(ids)
				.then((rows) => {
					if (active) statuses = rows;
				})
				.catch(() => {});
		}, 5000);
		return () => {
			active = false;
			clearInterval(timer);
		};
	});
	$effect(() => () => {
		alive = false;
		version++;
		members = [];
	});
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="shrink-0 border-b border-[var(--separator)] px-6 py-2 text-xs tabular-nums">
		{accountLabel} · {t.matching}: {row.matching_count} · {t.cached}: {row.total_count}
		· {t.unread}: {row.unread_count}
	</div>
	<ConversationActions {inbox} {row} {accountLabel} {disabled} />
	<div class="max-h-64 shrink-0 overflow-auto border-b border-[var(--separator)] p-4 text-sm">
		{#each members as member (member.message.id)}
			{@const state = statuses.find((s) => s.message_id === member.message.id)}
			<div class="border-b border-[var(--separator)] py-2">
				<button
					type="button"
					class="w-full text-left"
					aria-expanded={selectedId === member.message.id}
					{disabled}
					onclick={() => (selectedId = selectedId === member.message.id ? null : member.message.id)}
				>
					<span class="block">{member.message.subject} · {member.message.sender}</span>
					<span class="text-xs text-[var(--secondary)]"
						>{member.message.folder_override ?? member.message.folder} · {new Date(
							member.message.received_at_epoch_ms
						).toLocaleString()} · {member.matching ? t.matching : t.outside} · {selectedId ===
						member.message.id
							? t.close
							: t.open}</span
					>
				</button>
				{#if state?.pending}<p class="text-xs tabular-nums" role="status">
						{state.failed ? t.failed : t.pending} · {t.attempts}: {state.attempts}
					</p>{/if}
			</div>
		{/each}
		{#if loading}<p role="status">{t.building}</p>{/if}
		{#if error}<p role="alert">{t.unavailable}</p>
			<button type="button" onclick={() => void load(true)}>{t.refresh}</button>{/if}
		{#if members.length < total}<button
				type="button"
				disabled={loading || disabled || error}
				class="py-2 text-[var(--accent)]"
				onclick={() => void load()}>{t.more}</button
			>{/if}
	</div>
	{#if selected}
		<div
			class="flex shrink-0 items-center gap-4 border-b border-[var(--separator)] px-6 py-2 text-xs"
		>
			<span>{t.current}</span><button
				type="button"
				disabled={disabled || inbox.mutating || loading}
				onclick={() =>
					void inbox.mutateMember(selected.id, selected.is_read ? 'MarkUnread' : 'MarkRead')}
				>{selected.is_read ? t.unreadAction : t.read}</button
			><button
				type="button"
				disabled={disabled || inbox.mutating || loading}
				onclick={() => void inbox.mutateMember(selected.id, 'Archive')}>{t.archive}</button
			>
		</div>
	{/if}
	<SelectionOverview message={selected} {transport} />
</div>
