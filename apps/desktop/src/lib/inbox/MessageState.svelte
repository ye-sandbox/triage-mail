<script lang="ts">
	import { i18n } from '../i18n/index.svelte';
	import type { MessageRowDto } from './types';
	let { message, tags, folder }: { message: MessageRowDto; tags: string[]; folder: string } =
		$props();
</script>

<div
	class="shrink-0 border-b border-[var(--separator)] px-8 py-2 text-xs text-[var(--secondary)]"
	aria-label="Message state"
>
	{message.is_read ? 'Read' : 'Unread'} · {folder}
	{#if message.is_starred}
		· {i18n.t.mailboxes.Starred}{/if}
	{#if (message.snooze_until_epoch_ms ?? 0) > Date.now()}
		· {i18n.t.mailboxes.snoozedUntil}
		<time class="tabular-nums" datetime={new Date(message.snooze_until_epoch_ms!).toISOString()}
			>{new Date(message.snooze_until_epoch_ms!).toLocaleString(i18n.locale)}</time
		>{/if}
	{#each tags as tag}<span class="ml-2 break-all">#{tag}</span>{/each}
</div>
