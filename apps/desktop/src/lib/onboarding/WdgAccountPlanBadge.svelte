<script lang="ts">
	import { badgeTone, quotaSummary, type StorageQuota } from './subscriptions';

	type Props = { label: string | null; quota?: StorageQuota | null };
	let { label, quota = null }: Props = $props();
</script>

{#if label}
	<span
		data-testid="plan-badge"
		title={quota ? quotaSummary(quota) : undefined}
		class="shrink-0 rounded border bg-field px-2 py-1 font-mono text-[10px] font-semibold uppercase leading-none {badgeTone(
			label.toUpperCase()
		)}">{label}</span
	>
{/if}
