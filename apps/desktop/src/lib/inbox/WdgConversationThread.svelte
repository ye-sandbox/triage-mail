<script lang="ts">
	import { untrack } from 'svelte';
	import { Layers } from 'lucide-svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels } from '../conversations/labels';
	import WdgConversationActions from '../conversations/WdgConversationActions.svelte';
	import type { ConversationMember, ConversationRow, MutationStatus } from '../conversations/types';
	import type { MessageRowDto } from './types';
	import type { InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace: InboxWorkspace;
		row: ConversationRow;
		activeMemberId: string;
		onSelectMember: (member: MessageRowDto) => void;
	};

	let { workspace, row, activeMemberId, onSelectMember }: Props = $props();

	const inbox = $derived(workspace.inbox);
	const tConv = $derived(labels[i18n.locale]);
	const accountLabel = $derived(
		workspace.accounts.find((a) => a.account_id === row.representative.account_id)
			?.email_display ?? row.representative.account_id
	);

	let members = $state.raw<ConversationMember[]>([]);
	let statuses = $state.raw<MutationStatus[]>([]);
	let total = $state(0);
	let loading = $state(false);
	let error = $state(false);
	let version = 0;

	async function load(reset = false) {
		const api = inbox.conversationTransport;
		if (!api || (loading && !reset)) return;
		const generation = ++version;
		if (reset) {
			members = [];
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
			if (generation === version) {
				members = [...members, ...page.rows];
				total = page.total;
				statuses = [...statuses, ...state];
			}
		} catch {
			if (generation === version) error = true;
		} finally {
			if (generation === version) loading = false;
		}
	}

	$effect(() => {
		inbox.conversationGeneration;
		row.conversation_id;
		inbox.badgeRevision;
		untrack(() => void load(true));
		return () => {
			version++;
		};
	});
</script>

<div class="mx-4 mt-3 rounded-lg border border-separator bg-surface p-3 text-xs">
	<div class="mb-2 flex flex-wrap items-center justify-between gap-2">
		<div class="flex items-center gap-2 font-medium text-foreground">
			<Layers class="h-3.5 w-3.5 text-accent" />
			<span>{accountLabel}</span>
			<span class="font-mono text-[11px] tabular-nums text-secondary">
				{tConv.matching}: {row.matching_count} · {tConv.cached}: {row.total_count} · {tConv.unread}: {row.unread_count}
			</span>
		</div>
	</div>

	<div class="mb-2">
		<WdgConversationActions
			{inbox}
			{row}
			{accountLabel}
			disabled={!workspace.connected || inbox.loading}
		/>
	</div>

	{#if members.length > 0}
		<div class="flex max-h-40 flex-col gap-1 overflow-y-auto pt-1">
			{#each members as member (member.message.id)}
				{@const state = statuses.find((s) => s.message_id === member.message.id)}
				{@const selected = member.message.id === activeMemberId}
				<button
					type="button"
					onclick={() => onSelectMember(member.message)}
					class="flex w-full items-center justify-between gap-2 rounded-md border px-2.5 py-1.5 text-left transition-colors {selected
						? 'border-accent bg-selection-soft text-foreground'
						: 'border-separator bg-field text-secondary hover:text-foreground'}"
				>
					<span class="min-w-0 truncate font-medium">
						{member.message.sender} · {member.message.subject}
					</span>
					<span class="shrink-0 font-mono text-[10px]">
						{member.matching ? tConv.matching : tConv.outside}
						{#if state?.pending}
							· {state.failed ? tConv.failed : tConv.pending} ({state.attempts})
						{/if}
					</span>
				</button>
			{/each}
		</div>
	{/if}

	{#if loading}
		<p role="status" class="mt-2 text-secondary">{tConv.building}</p>
	{/if}
	{#if error}
		<div class="mt-2 flex items-center gap-2 text-status-danger" role="alert">
			<span>{tConv.unavailable}</span>
			<button type="button" onclick={() => void load(true)} class="underline">{tConv.refresh}</button>
		</div>
	{/if}
	{#if members.length < total}
		<button
			type="button"
			disabled={loading}
			onclick={() => void load()}
			class="mt-2 font-medium text-accent underline disabled:opacity-50"
		>
			{tConv.more}
		</button>
	{/if}
</div>
