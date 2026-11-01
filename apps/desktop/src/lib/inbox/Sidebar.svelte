<script lang="ts">
	import Icon from './Icon.svelte';
	let {
		status,
		openSettings
	}: {
		status: string;
		openSettings?: () => void;
	} = $props();
</script>

<aside
	class="flex h-full flex-col border-r border-[var(--separator)] bg-[var(--sidebar)] px-4 py-6"
	aria-label="Mail navigation"
>
	<a
		href="/"
		class="mb-12 flex items-center gap-3 px-2 text-base font-semibold tracking-tight text-[var(--text)]"
	>
		<span class="flex size-9 items-center justify-center rounded-xl bg-[var(--accent)] text-white"
			><Icon name="inbox" /></span
		>Triage Mail
	</a>
	<p class="mb-2 px-3 text-xs font-semibold text-[var(--secondary)]">Mailboxes</p>
	<nav aria-label="Mailboxes">
		<a
			href="/"
			aria-current="page"
			class="flex items-center gap-3 rounded-lg bg-[var(--selection-soft)] px-3 py-2 text-sm font-medium text-[var(--text)]"
			><span class="text-[var(--accent)]"><Icon name="inbox" size={18} /></span>All Mail</a
		>
	</nav>
	<div class="mt-8 px-3">
		<p class="text-xs font-semibold text-[var(--secondary)]">Your mail, together</p>
		<p class="mt-3 text-xs leading-relaxed text-[var(--secondary)]">
			Search across your connected accounts. Everything stays on this device.
		</p>
	</div>
	<div class="mt-auto space-y-4 px-3">
		<div class="flex items-center gap-2 text-xs text-[var(--secondary)]" role="status">
			<span class="size-1.5 rounded-full bg-[var(--accent)]"></span>{status}
		</div>
		{#if openSettings}
			<button
				type="button"
				class="flex w-full items-center gap-2 rounded-lg py-1.5 px-2 -mx-2 text-left text-xs text-[var(--secondary)] transition-colors hover:bg-[var(--surface)] hover:text-[var(--text)]"
				aria-label="Settings"
				aria-haspopup="dialog"
				onclick={openSettings}
			>
				<Icon name="settings" size={16} />
				<span>Settings</span>
			</button>
		{/if}
		<div
			class="flex items-center gap-2 border-t border-[var(--separator)] pt-4 text-xs text-[var(--secondary)]"
		>
			<Icon name="shield" size={16} />Private by design
		</div>
	</div>
</aside>
