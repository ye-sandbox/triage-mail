<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import '@fontsource/plus-jakarta-sans/500.css';
	import '@fontsource/plus-jakarta-sans/600.css';
	import '@fontsource/plus-jakarta-sans/700.css';
	import '@fontsource/dm-sans/400.css';
	import '@fontsource/dm-sans/500.css';
	import '@fontsource/dm-sans/600.css';
	import '@fontsource/dm-sans/700.css';
	import '@fontsource/jetbrains-mono/400.css';
	import '@fontsource/jetbrains-mono/500.css';
	import '@fontsource/jetbrains-mono/600.css';
	import '../app.css';
	import { Toaster } from 'svelte-sonner';
	import { nativeAvailable } from '$lib/inbox/transport';
	import WdgCommandPalette from '$lib/inbox/WdgCommandPalette.svelte';
	import WdgSnoozeSheet from '$lib/inbox/WdgSnoozeSheet.svelte';
	import AppShell from '$lib/shell/AppShell.svelte';
	import { appStore } from '$lib/stores/app.svelte';
	import type { WindowTransport } from '$lib/window/types';

	type Props = {
		children: Snippet;
		transport?: WindowTransport;
	};

	let { children, transport }: Props = $props();

	$effect(() => {
		appStore.syncDocumentTheme();
	});

	$effect(() => {
		if (!nativeAvailable() || !appStore.vaultUnlocked) return;
		untrack(() => {
			void appStore.hydrateNativeShellCounts();
		});
	});
</script>

<Toaster richColors position="bottom-right" theme={appStore.theme} />

<AppShell {transport}>
	{@render children()}
</AppShell>

<WdgCommandPalette />
<WdgSnoozeSheet />
