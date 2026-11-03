<script lang="ts">
	import { untrack } from 'svelte';
	import type { InboxController } from '../inbox/controller.svelte';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	let { inbox, disabled = false }: { inbox: InboxController; disabled?: boolean } = $props();
	const t = $derived(labels[i18n.locale]);
	let progress = $state<{
		indexed: number;
		total: number;
		ready: boolean;
	} | null>(null);
	$effect(() => {
		inbox.badgeRevision;
		const scope = inbox.scope;
		const accountId =
			scope && typeof scope === 'object' && !inbox.savedView
				? scope.AccountView.account_id
				: undefined;
		const transport = inbox.conversationTransport;
		if (!transport) return;
		let alive = true;
		let timer: ReturnType<typeof setTimeout>;
		async function load() {
			try {
				const value = await transport!.progress(accountId);
				if (alive) {
					progress = value;
					if (!value.ready) timer = setTimeout(() => void load(), 2000);
				}
			} catch {
				if (alive) progress = null;
			}
		}
		untrack(() => void load());
		return () => {
			alive = false;
			clearTimeout(timer);
		};
	});
</script>

{#if inbox.conversationTransport}
	<div class="flex items-center gap-4 border-b border-[var(--separator)] px-6 py-2 text-xs">
		<button
			type="button"
			class="text-[var(--accent)]"
			aria-pressed={inbox.conversations}
			disabled={disabled ||
				inbox.loading ||
				inbox.mutating ||
				(!inbox.conversations && !progress?.ready)}
			onclick={() => void inbox.toggleConversations()}>{t.title}</button
		>
		{#if progress && !progress.ready}<span role="status" class="tabular-nums"
				>{t.building}: {progress.indexed}/{progress.total}</span
			>{:else if inbox.conversations}<span>{t.help}</span>{/if}
	</div>
{/if}
