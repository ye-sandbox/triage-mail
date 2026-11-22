<script lang="ts">
	import { untrack } from 'svelte';
	import ImagePlus from '@lucide/svelte/icons/image-plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { i18n } from '../i18n/index.svelte';
	import { AvatarController, avatarFallback } from './avatar-controller.svelte';
	import { nativeAvatarTransport } from './avatar-transport';
	import type { AvatarTransport } from './avatar-types';
	import { appStore } from '../stores/app.svelte';

	interface Props {
		accountId: string;
		label: string;
		transport?: AvatarTransport;
	}

	let { accountId, label, transport = nativeAvatarTransport }: Props = $props();
	let controller = $state<AvatarController>();
	let fileInput = $state<HTMLInputElement>();

	const t = $derived(i18n.t.accountPhoto);
	const fallback = $derived(avatarFallback(accountId, label));

	$effect(() => {
		const photo = new AvatarController(accountId, transport, (src) => {
			appStore.setAccountAvatarSrc(accountId, src);
		});
		controller = photo;
		untrack(() => void photo.load());
		return () => photo.dispose();
	});

	function handleSelect(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (file) void controller?.replace(file);
	}
</script>

<div class="flex flex-wrap items-center gap-3">
	<div
		class="relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-md border border-separator font-display text-xs font-semibold text-foreground"
		class:bg-accent={!controller?.src && fallback.color === 0}
		class:text-accent-foreground={!controller?.src && fallback.color === 0}
		class:bg-field={!controller?.src && fallback.color !== 0}
	>
		{#if controller?.src}
			<img
				src={controller.src}
				alt={`${t.label}: ${label}`}
				referrerpolicy="no-referrer"
				class="h-full w-full object-cover"
			/>
		{:else}
			<span aria-hidden="true">{fallback.initials}</span>
		{/if}
	</div>

	<div class="flex flex-col gap-1">
		<div class="flex flex-wrap items-center gap-2">
			<input
				bind:this={fileInput}
				type="file"
				accept="image/jpeg,image/png,image/webp"
				onchange={handleSelect}
				disabled={!controller || controller.loading || controller.busy}
				aria-label={`${controller?.src ? t.replace : t.choose}: ${label}`}
				class="hidden"
			/>
			<button
				type="button"
				onclick={() => fileInput?.click()}
				disabled={!controller || controller.loading || controller.busy}
				class="flex min-h-[34px] items-center gap-1 rounded-md border border-separator bg-field px-3 py-1 text-xs text-foreground whitespace-nowrap hover:bg-selection-soft disabled:opacity-50"
			>
				<ImagePlus class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{controller?.src ? t.replace : t.choose}</span>
			</button>

			{#if controller?.src}
				<button
					type="button"
					onclick={() => void controller?.remove()}
					disabled={controller.busy}
					aria-label={`${t.remove}: ${label}`}
					class="flex min-h-[34px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs text-secondary whitespace-nowrap hover:text-status-danger disabled:opacity-50"
				>
					<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{t.remove}</span>
				</button>
			{/if}
		</div>
		{#if controller?.loading || controller?.busy}
			<p role="status" class="text-[11px] text-secondary">{t.pending}</p>
		{/if}
		{#if controller?.error}
			<p role="alert" class="text-[11px] text-status-danger">{t[controller.error]}</p>
		{/if}
	</div>
</div>
