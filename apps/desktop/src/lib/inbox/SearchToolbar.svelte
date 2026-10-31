<script lang="ts">
	import Icon from './Icon.svelte';
	import type { SearchMode } from './types';
	let {
		query = $bindable(''),
		mode = $bindable<SearchMode>('Lexical'),
		toggleSidebar,
		refresh,
		sidebarOpen = true,
		busy = false,
		disabled = false,
		experimentalSemanticSearch = false
	}: {
		query?: string;
		mode?: SearchMode;
		toggleSidebar: () => void;
		refresh: () => void;
		sidebarOpen?: boolean;
		busy?: boolean;
		disabled?: boolean;
		experimentalSemanticSearch?: boolean;
	} = $props();
	let searchInput: HTMLInputElement;
	function shortcut(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'f') {
			event.preventDefault();
			searchInput?.focus();
		}
	}
	$effect(() => {
		if (!experimentalSemanticSearch && mode !== 'Lexical') {
			mode = 'Lexical';
		}
	});
</script>

<svelte:window onkeydown={shortcut} />
<header
	class="flex min-h-18 flex-wrap items-center gap-4 border-b border-[var(--separator)] bg-[var(--toolbar)] px-6 py-4"
>
	<button
		type="button"
		class="icon-button max-md:hidden"
		aria-label="Toggle sidebar"
		aria-expanded={sidebarOpen}
		title="Toggle sidebar"
		onclick={toggleSidebar}><Icon name="sidebar" /></button
	>
	<div class="mr-auto">
		<h1 class="text-lg font-semibold tracking-tight">All Mail</h1>
		<p class="text-xs text-[var(--secondary)]">Unified inbox</p>
	</div>
	<div
		class="search-field flex min-w-48 flex-1 items-center gap-2 rounded-lg border border-[var(--separator)] bg-[var(--field)] px-3 py-2 sm:max-w-96"
		role="search"
	>
		<span class="text-[var(--secondary)]"><Icon name="search" size={16} /></span>
		<input
			bind:this={searchInput}
			bind:value={query}
			type="search"
			aria-label="Search all mail"
			placeholder="Search all mail"
			class="min-w-0 flex-1 bg-transparent text-sm outline-none"
		/>
	</div>
	{#if experimentalSemanticSearch}
		<label class="flex items-center gap-2 text-xs text-[var(--secondary)]"
			>Match
			<select
				bind:value={mode}
				aria-label="Search matching"
				class="rounded-md border border-[var(--separator)] bg-[var(--field)] px-2 py-2 text-sm text-[var(--text)]"
			>
				<option value="Lexical">Words</option>
				<option value="Semantic">Semantic projection (Experimental)</option>
				<option value="Hybrid">Words + semantic projection (Experimental)</option>
			</select>
		</label>
	{/if}
	<button
		type="button"
		class="icon-button"
		aria-label="Refresh mail"
		title="Refresh mail"
		onclick={refresh}
		{disabled}
		><span class="refresh-symbol flex" class:is-spinning={busy}
			><Icon name="refresh" size={18} /></span
		></button
	>
</header>
