<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import type { AvatarTransport } from './avatar-types';
	import { nativeAvatarTransport } from './avatar-transport';
	import { AvatarController, avatarFallback } from './avatar-controller.svelte';

	let {
		accountId,
		label,
		transport = nativeAvatarTransport
	}: {
		accountId: string;
		label: string;
		transport?: AvatarTransport;
	} = $props();
	let controller = $state<AvatarController>();
	const t = $derived(i18n.t.accountPhoto);
	const fallback = $derived(avatarFallback(accountId, label));
	$effect(() => {
		const photo = new AvatarController(accountId, transport);
		controller = photo;
		untrack(() => void photo.load());
		return () => photo.dispose();
	});

	function select(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (file) void controller?.replace(file);
	}
</script>

<div role="group" class="flex flex-wrap items-center gap-4" aria-label={`${t.label}: ${label}`}>
	{#if controller?.src}
		<img
			src={controller.src}
			alt=""
			width="48"
			height="48"
			class="h-12 w-12 rounded-full object-cover"
		/>
	{:else}
		<span
			class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full font-semibold"
			class:bg-[var(--accent)]={fallback.color === 0}
			class:bg-[var(--surface)]={fallback.color === 1}
			class:bg-[var(--separator)]={fallback.color === 2}
			aria-hidden="true">{fallback.initials}</span
		>
	{/if}
	<div class="min-w-0 space-y-2">
		<label class="block text-sm text-[var(--secondary)]">
			{controller?.src ? t.replace : t.choose}
			<input
				type="file"
				accept="image/jpeg,image/png,image/webp"
				onchange={select}
				disabled={!controller || controller.loading || controller.busy}
				class="mt-2 block w-full max-w-xs text-sm file:mr-4 file:rounded-lg file:border-0 file:bg-[var(--separator)] file:px-4 file:py-2 file:text-[var(--text)] disabled:opacity-50"
			/>
		</label>
		{#if controller?.src}
			<button
				type="button"
				class="text-sm text-[var(--accent)] disabled:opacity-50"
				disabled={controller.busy}
				onclick={() => void controller?.remove()}>{t.remove}</button
			>
		{/if}
		<p class="text-xs text-[var(--secondary)]">{t.help}</p>
		{#if controller?.loading || controller?.busy}<p role="status" class="text-sm">
				{t.pending}
			</p>{/if}
		{#if controller?.error}<p role="alert" class="text-sm text-[var(--text)]">
				{t[controller.error]}
			</p>{/if}
	</div>
</div>
