<script lang="ts">
	import { sanitizeEmailHtml } from '$lib/security/sanitize';

	let searchQuery = $state('');
	let allowRemoteImages = $state(false);

	const previewHtml = $derived(
		sanitizeEmailHtml(
			'<p>Welcome to <strong>triage-mail</strong> local-first unified inbox.</p>',
			{ allowRemoteImages }
		)
	);
</script>

<main class="min-h-screen bg-background text-foreground p-6">
	<header class="flex items-center justify-between border-b pb-4 mb-6">
		<div>
			<h1 class="text-xl font-semibold">triage-mail</h1>
			<p class="text-sm text-muted-foreground">
				Local-First Email Aggregator &amp; 3-Tier Triage Engine
			</p>
		</div>
		<div class="flex items-center gap-4">
			<input
				type="search"
				placeholder="Hybrid search (FTS5 + sqlite-vec)..."
				bind:value={searchQuery}
				class="px-4 py-2 rounded border bg-card text-foreground"
			/>
			<button
				type="button"
				onclick={() => (allowRemoteImages = !allowRemoteImages)}
				class="px-4 py-2 rounded border bg-card text-foreground font-mono tabular-nums"
			>
				Remote Images: {allowRemoteImages ? 'ON' : 'OFF'}
			</button>
		</div>
	</header>

	<section class="grid grid-cols-1 gap-4">
		<iframe
			title="Email Content Preview"
			sandbox=""
			referrerpolicy="no-referrer"
			srcdoc={previewHtml}
			class="w-full h-64 border rounded bg-card"
		></iframe>
	</section>
</main>
