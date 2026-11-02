<script lang="ts">
	import { untrack } from 'svelte';
	import { AvatarController, avatarFallback } from '../onboarding/avatar-controller.svelte';
	import { nativeAvatarTransport } from '../onboarding/avatar-transport';
	import type { AvatarTransport } from '../onboarding/avatar-types';
	let {
		accountId,
		label,
		revision = 0,
		transport = nativeAvatarTransport
	}: {
		accountId: string;
		label: string;
		revision?: number;
		transport?: AvatarTransport;
	} = $props();
	let photo = $state<AvatarController>();
	const fallback = $derived(avatarFallback(accountId, label));
	$effect(() => {
		revision;
		const controller = new AvatarController(accountId, transport);
		photo = controller;
		untrack(() => void controller.load());
		return () => controller.dispose();
	});
</script>

{#if photo?.src}
	<img
		src={photo.src}
		alt=""
		width="32"
		height="32"
		class="size-8 shrink-0 rounded-full object-cover"
	/>
{:else}
	<span
		aria-hidden="true"
		class="flex size-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold"
		class:bg-[var(--accent)]={fallback.color === 0}
		class:bg-[var(--surface)]={fallback.color === 1}
		class:bg-[var(--separator)]={fallback.color === 2}>{fallback.initials}</span
	>
{/if}
