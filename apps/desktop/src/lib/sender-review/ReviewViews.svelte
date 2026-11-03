<script lang="ts">
	import type { InboxController } from '../inbox/controller.svelte';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import type { ReviewFilter } from './types';
	let { inbox, connected }: { inbox: InboxController; connected: boolean } = $props();
	const t = $derived(labels[i18n.locale]);
</script>

{#if inbox.reviewTransport}
	<section
		class="mt-4 space-y-2 border-t border-[var(--separator)] pt-4 text-sm"
		aria-label={t.title}
	>
		<label class="block px-2"
			>{t.title}<select
				class="mt-2 w-full rounded border bg-[var(--surface)] p-2"
				value={inbox.review ?? ''}
				disabled={!connected || inbox.mutating}
				onchange={(e) =>
					void inbox.selectReview((e.currentTarget.value || null) as ReviewFilter | null)}
			>
				<option value="">{t.all}</option><option value="Pending">{t.Pending}</option><option
					value="Declined">{t.Declined}</option
				>
			</select></label
		>
	</section>
{/if}
