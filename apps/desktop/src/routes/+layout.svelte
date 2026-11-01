<script lang="ts">
	import type { Snippet } from 'svelte';
	import TitleBar from '$lib/window/TitleBar.svelte';
	import type { WindowTransport } from '$lib/window/types';
	import '../app.css';

	type Props = {
		children: Snippet;
		transport?: WindowTransport;
	};

	let { children, transport }: Props = $props();
</script>

<div class="desktop-shell">
	<TitleBar {transport} />
	<div class="desktop-shell-content">
		{@render children()}
	</div>
</div>
