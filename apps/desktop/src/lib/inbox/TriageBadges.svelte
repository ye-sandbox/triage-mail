<script lang="ts">
	import type { TriageBadge } from '../reader/types';
	import { badgeText } from './badges';
	let { badges, compact = false }: { badges: TriageBadge[]; compact?: boolean } = $props();
</script>

{#if badges.length}
	<ul
		aria-label="Triage badges"
		class="flex min-w-0 gap-2"
		class:flex-wrap={!compact}
		class:overflow-hidden={compact}
	>
		{#each badges as badge}
			{@const text = badgeText(badge)}
			<li
				class="min-w-0 rounded bg-[var(--selection-soft)] px-2 text-xs text-[var(--accent)] tabular-nums"
				class:shrink-0={compact}
			>
				<span
					class="font-medium"
					title={text.value}
					aria-label={compact ? `${text.label}: ${text.value}` : undefined}>{text.label}</span
				>
				{#if !compact}<span class="ml-2 break-all select-text">{text.value}</span>{/if}
			</li>
		{/each}
	</ul>
{/if}
