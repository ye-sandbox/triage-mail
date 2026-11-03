<script lang="ts">
	import type { InboxController } from '../inbox/controller.svelte';
	import { i18n } from '../i18n/index.svelte';
	import { categories, type Category } from './types';
	import { labels } from './labels';
	let { inbox, connected }: { inbox: InboxController; connected: boolean } = $props();
	const t = $derived(labels[i18n.locale]);
</script>

{#if inbox.categoryTransport}
	<section
		class="mt-4 space-y-2 border-t border-[var(--separator)] pt-4 text-sm"
		aria-label={t.title}
	>
		<label class="block px-2"
			>{t.title}
			<select
				class="mt-2 w-full rounded border border-[var(--separator)] bg-[var(--surface)] p-2"
				disabled={!connected || inbox.mutating}
				value={inbox.category ?? ''}
				onchange={(e) =>
					void inbox.selectCategory((e.currentTarget.value || null) as Category | null)}
			>
				<option value="">{t.all}</option>{#each categories as category}<option value={category}
						>{t.category[category]}</option
					>{/each}
			</select></label
		>
		<p class="px-2 text-xs text-[var(--secondary)]">{t.local}</p>
	</section>
{/if}
