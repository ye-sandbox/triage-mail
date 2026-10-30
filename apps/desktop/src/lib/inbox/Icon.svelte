<script lang="ts">
	let {
		name,
		size = 20
	}: {
		name: 'inbox' | 'search' | 'shield' | 'chevron' | 'refresh' | 'mail' | 'paperclip' | 'sidebar';
		size?: number;
	} = $props();
	const paths = {
		inbox: 'M4 4h16l2 11v5H2v-5L4 4Zm-2 11h6l2 3h4l2-3h6',
		search: 'm21 21-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z',
		shield: 'm12 2 8 3v6c0 5-8 11-8 11S4 16 4 11V5l8-3Zm-4 9 3 3 5-6',
		chevron: 'm9 5 7 7-7 7',
		refresh: 'M20 8a9 9 0 1 0 1 8M20 2v6h-6',
		mail: 'M3 5h18v14H3V5Zm0 0 9 8 9-8',
		paperclip: 'm8 12 7-7a4 4 0 0 1 6 6L10 22a6 6 0 0 1-8-8L13 3m-8 14 11-11',
		sidebar: 'M3 4h18v16H3V4Zm6 0v16'
	};
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.6"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"><path d={paths[name]} /></svg
>
