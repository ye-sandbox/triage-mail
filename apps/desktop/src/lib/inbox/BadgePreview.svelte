<script lang="ts">
	import { untrack } from 'svelte';
	import { BadgeLoader } from './badges';
	import type { TriageBadge } from '../reader/types';
	import TriageBadges from './TriageBadges.svelte';
	let { id, loader, revision }: { id: string; loader: BadgeLoader; revision: number } = $props();
	let badges = $state<TriageBadge[]>([]);
	$effect(() => {
		const messageId = id;
		const source = loader;
		const epoch = revision;
		return untrack(() => {
			badges = [];
			return source.watch(
				messageId,
				(result) => {
					badges = result;
				},
				epoch
			);
		});
	});
</script>

<TriageBadges {badges} compact />
