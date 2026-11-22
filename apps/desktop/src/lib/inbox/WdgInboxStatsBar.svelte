<script lang="ts">
	import { untrack } from 'svelte';
	import { Select } from 'bits-ui';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Layers from '@lucide/svelte/icons/layers';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import PanelLeftClose from '@lucide/svelte/icons/panel-left-close';
	import PanelLeftOpen from '@lucide/svelte/icons/panel-left-open';
	import Search from '@lucide/svelte/icons/search';
	import Settings from '@lucide/svelte/icons/settings';
	import X from '@lucide/svelte/icons/x';
	import { t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import { resolveInboxScopeLabel } from './scope-label';
	import type { SearchMode } from './types';
	import WdgInboxTelemetryStrip from './WdgInboxTelemetryStrip.svelte';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace?: InboxWorkspace;
		openSettings?: () => void;
	};

	let { workspace = workspaceRegistry.active!, openSettings }: Props = $props();

	const SEARCH_MODES: { value: SearchMode; labelKey: 'lexical' | 'meaningLocal' | 'hybridLocal' }[] = [
		{ value: 'Lexical', labelKey: 'lexical' },
		{ value: 'Semantic', labelKey: 'meaningLocal' },
		{ value: 'Hybrid', labelKey: 'hybridLocal' }
	];

	let query = $state(untrack(() => workspace.inbox.query));
	let mode = $state<SearchMode>(untrack(() => workspace.inbox.mode));

	$effect(() => {
		const view = workspace.inbox.savedView;
		if (view) {
			untrack(() => {
				query = view.query;
				mode = view.mode;
			});
		}
	});

	$effect(() => {
		const text = query.trim();
		const matching = mode;
		const inbox = workspace.inbox;
		if (!workspace.connected || inbox.savedView) return;
		if (
			!text &&
			matching === 'Lexical' &&
			!untrack(() => inbox.query) &&
			untrack(() => inbox.mode) === 'Lexical'
		) {
			return;
		}
		const timer = setTimeout(() => void inbox.search(text, matching), 200);
		return () => clearTimeout(timer);
	});

	const scopeLabel = $derived(resolveInboxScopeLabel(workspace));

	const showOnnxHint = $derived(
		!workspace.inbox.experimentalSemanticSearch && mode !== 'Lexical'
	);

	function clearSearch() {
		query = '';
		void workspace.inbox.search('', mode);
	}
</script>

<div class="flex shrink-0 flex-col border-b border-separator bg-toolbar">
	<div class="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
		<div class="flex min-w-0 items-center gap-2">
			<button
				type="button"
				aria-expanded={appStore.sidebarOpen}
				aria-label="Toggle sidebar"
				onclick={() => appStore.toggleSidebar()}
				class="hidden min-h-9 min-w-9 items-center justify-center rounded-md border border-separator bg-surface text-secondary transition-transform duration-100 ease-out hover:text-foreground active:scale-[0.98] md:flex"
			>
				{#if appStore.sidebarOpen}
					<PanelLeftClose class="h-4 w-4" aria-hidden="true" />
				{:else}
					<PanelLeftOpen class="h-4 w-4" aria-hidden="true" />
				{/if}
			</button>

			<div class="flex items-center gap-2 truncate text-xs font-medium text-foreground">
				<span class="text-secondary">{t(appStore.locale, 'navInbox')}</span>
				<span aria-hidden="true" class="text-secondary">/</span>
				<span class="truncate font-display font-semibold">{scopeLabel}</span>
			</div>
		</div>

		<div class="flex w-full min-w-0 flex-none flex-wrap items-center justify-end gap-2 sm:w-auto sm:min-w-[240px] sm:flex-1">
			<div class="relative flex w-full min-w-0 basis-full items-center sm:w-auto sm:min-w-[180px] sm:max-w-md sm:flex-1 sm:basis-auto">
				<Search class="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<input
					type="search"
					bind:value={query}
					disabled={!workspace.connected || workspace.inbox.loading}
					placeholder={t(appStore.locale, 'searchPlaceholder')}
					aria-label={t(appStore.locale, 'searchPlaceholder')}
					class="h-9 w-full rounded-md border border-separator bg-field pl-8 pr-8 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-50"
				/>
				{#if query}
					<button
						type="button"
						onclick={clearSearch}
						aria-label="Clear search"
						class="absolute right-2 rounded p-1 text-secondary hover:text-foreground"
					>
						<X class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
				{/if}
			</div>

			<Select.Root
				type="single"
				value={mode}
				onValueChange={(val) => {
					if (val === 'Lexical' || val === 'Semantic' || val === 'Hybrid') {
						mode = val;
						void workspace.inbox.search(query.trim(), val);
					}
				}}
			>
				<Select.Trigger
					aria-label="Search mode"
					disabled={!workspace.connected || workspace.inbox.loading}
					class="flex h-9 min-w-[140px] shrink-0 items-center justify-between gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 text-xs text-foreground hover:bg-field disabled:opacity-50"
				>
					<span class="truncate">
						{t(
							appStore.locale,
							SEARCH_MODES.find((m) => m.value === mode)?.labelKey ?? 'lexical'
						)}
					</span>
					<ChevronDown class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
				</Select.Trigger>
				<Select.Portal>
					<Select.Content
						class="z-50 min-w-[200px] rounded-md border border-separator bg-surface p-1 shadow-md"
						sideOffset={4}
					>
						{#each SEARCH_MODES as option (option.value)}
							<Select.Item
								value={option.value}
								label={t(appStore.locale, option.labelKey)}
								class="flex cursor-pointer items-center justify-between rounded px-3 py-2 text-xs text-foreground outline-none data-[highlighted]:bg-selection-soft"
							>
								<span>{t(appStore.locale, option.labelKey)}</span>
								{#if option.value !== 'Lexical' && !workspace.inbox.experimentalSemanticSearch}
									<span class="text-[10px] text-status-warning">OFF</span>
								{/if}
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Portal>
			</Select.Root>

			<button
				type="button"
				onclick={() => void workspace.inbox.toggleConversations()}
				aria-pressed={workspace.inbox.conversations}
				disabled={!workspace.connected}
				class="flex min-h-9 shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98] disabled:opacity-50"
			>
				{#if workspace.inbox.conversations}
					<Layers class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
					<span class="hidden lg:inline">{t(appStore.locale, 'threadedConversations')}</span>
				{:else}
					<MessageSquare class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span class="hidden lg:inline">{t(appStore.locale, 'individualMessages')}</span>
				{/if}
			</button>

			{#if openSettings}
				<button
					type="button"
					onclick={openSettings}
					aria-label="Settings"
					title={t(appStore.locale, 'navSettings')}
					class="flex min-h-9 shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-field active:scale-[0.98]"
				>
					<Settings class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span class="hidden lg:inline">{t(appStore.locale, 'navSettings')}</span>
				</button>
			{/if}
		</div>
	</div>

	{#if showOnnxHint}
		<div
			role="status"
			class="flex items-center justify-between gap-2 border-t border-separator bg-selection-soft px-3 py-2 text-xs text-status-warning"
		>
			<span class="flex items-center gap-2">
				<AlertCircle class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<span>{t(appStore.locale, 'onnxDisabledHint')}</span>
			</span>
			{#if openSettings}
				<button type="button" onclick={openSettings} class="whitespace-nowrap font-medium underline">
					Open {t(appStore.locale, 'navSettings')}
				</button>
			{:else}
				<a href="/settings" class="whitespace-nowrap font-medium underline">
					{t(appStore.locale, 'navSettings')}
				</a>
			{/if}
		</div>
	{/if}

	<WdgInboxTelemetryStrip {workspace} />
</div>
