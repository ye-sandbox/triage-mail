<script lang="ts">
	import { untrack } from 'svelte';
	import { Dialog } from 'bits-ui';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import { blankView, type SavedView } from './types';
	import type { InboxController } from '../inbox/controller.svelte';
	import type { AccountConfig } from '../onboarding/types';
	import ViewEditor from './ViewEditor.svelte';
	let {
		inbox,
		accounts,
		connected
	}: { inbox: InboxController; accounts: AccountConfig[]; connected: boolean } = $props();
	const t = $derived(labels[i18n.locale]);
	let views = $state.raw<SavedView[]>([]);
	let draft = $state<SavedView | null>(null);
	let baseline = '';
	let open = $state(false);
	let busy = $state(false);
	let error = $state('');
	let disposed = false;
	let generation = 0;
	async function reload() {
		const revision = ++generation;
		try {
			const list = await inbox.savedTransport?.list();
			if (!disposed && revision === generation) {
				views = list ?? [];
				error = '';
			}
		} catch {
			if (!disposed && revision === generation) error = t.error;
		}
	}
	$effect(() => {
		if (connected && inbox.savedTransport) untrack(() => void reload());
	});
	$effect(() => () => {
		disposed = true;
		generation++;
		views = [];
		draft = null;
	});
	function edit(view: SavedView) {
		draft = structuredClone(view);
		baseline = JSON.stringify(draft);
		open = true;
		error = '';
	}
	function create(starter?: 'unread' | 'invoices' | 'bills' | 'current') {
		const view =
			starter === 'current'
				? inbox.savedView
					? { ...structuredClone(inbox.savedView), id: crypto.randomUUID(), revision: 0, name: '' }
					: blankView(inbox.query, inbox.mode, inbox.scope)
				: blankView();
		if (starter && starter !== 'current') {
			view.name = t[starter];
			view.query = '';
			view.mode = 'Lexical';
			view.scope = null;
			if (starter === 'unread') view.filters.is_read = false;
			if (starter === 'invoices') view.filters.extraction = 'Invoice';
			if (starter === 'bills') {
				view.filters.extraction = 'Boleto';
				view.filters.upcoming_days = 30;
			}
		}
		if (starter === 'current' && inbox.category) view.filters.category = inbox.category;
		edit(view);
	}
	function close() {
		if (busy || (draft && JSON.stringify(draft) !== baseline && !window.confirm(t.discard))) return;
		open = false;
		draft = null;
	}
	async function save() {
		if (!draft || busy || !inbox.savedTransport) return;
		busy = true;
		const value = JSON.parse(JSON.stringify(draft)) as SavedView;
		try {
			const saved = await inbox.savedTransport.save(value);
			if (disposed) return;
			open = false;
			draft = null;
			if (inbox.savedView?.id === saved.id) await inbox.selectSavedView(saved);
			await reload();
		} catch {
			if (!disposed) error = t.error;
		} finally {
			if (!disposed) busy = false;
		}
	}
	async function remove(view: SavedView) {
		if (busy || !inbox.savedTransport || !window.confirm(t.confirmDelete)) return;
		busy = true;
		try {
			await inbox.savedTransport.remove(view.id, view.revision);
			if (disposed) return;
			if (inbox.savedView?.id === view.id) await inbox.selectScope();
			await reload();
		} catch {
			if (!disposed) error = t.error;
		} finally {
			if (!disposed) busy = false;
		}
	}
</script>

{#if connected && inbox.savedTransport}
	<section
		aria-label={t.title}
		class="mt-4 space-y-2 border-t border-[var(--separator)] pt-4 text-sm"
	>
		<h2 class="px-2 font-semibold">{t.title}</h2>
		<button class="px-2 py-2 text-[var(--accent)]" disabled={busy} onclick={() => create('current')}
			>{t.current}</button
		>
		<button class="px-2 py-2" disabled={busy} onclick={() => create()}>{t.add}</button>
		<details>
			<summary class="px-2">{t.starter}</summary>
			<div class="flex flex-col items-start">
				{#each ['unread', 'invoices', 'bills'] as starter}<button
						class="px-2 py-2"
						disabled={busy}
						onclick={() => create(starter as 'unread' | 'invoices' | 'bills')}
						>{t[starter as 'unread' | 'invoices' | 'bills']}</button
					>{/each}
				<p class="px-2 text-xs">{t.clients}</p>
			</div>
		</details>
		{#each views as view (view.id)}<div
				class="space-y-2 rounded px-2 py-2"
				class:active={inbox.savedView?.id === view.id}
			>
				<button
					class="w-full truncate text-left"
					aria-current={inbox.savedView?.id === view.id ? 'page' : undefined}
					disabled={busy}
					onclick={() => void inbox.selectSavedView(view)}>{view.name}</button
				>
				<div class="flex gap-4 text-xs text-[var(--secondary)]">
					<button disabled={busy} onclick={() => edit(view)} aria-label={`${t.edit}: ${view.name}`}
						>{t.edit}</button
					><button
						disabled={busy}
						onclick={() => void remove(view)}
						aria-label={`${t.remove}: ${view.name}`}>{t.remove}</button
					>
				</div>
			</div>{/each}
		{#if error && !open}<p role="alert">{error}</p>
			<button disabled={busy} onclick={() => void reload()}>{t.reload}</button>{/if}
	</section>
	<Dialog.Root
		{open}
		onOpenChange={(value) => {
			if (!value) close();
		}}
	>
		<Dialog.Portal
			><Dialog.Overlay class="fixed inset-0 z-50 bg-black/50" />
			<Dialog.Content
				onkeydown={(e) => e.stopPropagation()}
				class="fixed inset-x-4 top-8 z-50 mx-auto max-h-[90vh] max-w-xl overflow-y-auto rounded-xl border border-[var(--separator)] bg-[var(--background)] p-6 text-[var(--text)]"
				onEscapeKeydown={(e) => {
					e.preventDefault();
					close();
				}}
				onInteractOutside={(e) => {
					e.preventDefault();
					close();
				}}
			>
				<Dialog.Title class="mb-4 text-lg font-semibold">{t.title}</Dialog.Title>
				<Dialog.Description class="mb-4 text-sm">{t.help}</Dialog.Description>
				{#if error}<p role="alert">{error}</p>{/if}
				{#if draft}<ViewEditor
						{draft}
						{accounts}
						{busy}
						save={() => void save()}
						cancel={close}
						semantic={inbox.experimentalSemanticSearch}
					/>{/if}
			</Dialog.Content></Dialog.Portal
		></Dialog.Root
	>
{/if}

<style>
	.active {
		background: var(--selection-soft);
	}
</style>
