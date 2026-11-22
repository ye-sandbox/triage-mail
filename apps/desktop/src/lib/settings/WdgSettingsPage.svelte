<script lang="ts">
	import { untrack } from 'svelte';
	import ArchiveRestore from '@lucide/svelte/icons/archive-restore';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Cpu from '@lucide/svelte/icons/cpu';
	import Database from '@lucide/svelte/icons/database';
	import Lock from '@lucide/svelte/icons/lock';
	import Settings2 from '@lucide/svelte/icons/settings-2';
	import { demoAutomationBackend } from '$lib/automation/automation.fixtures';
	import type { TransferTransport } from '$lib/backup/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { onboardingSession } from '$lib/onboarding/session.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { appStore } from '$lib/stores/app.svelte';
	import { SettingsFormGuard } from './guards';
	import { settingsLabels, type SettingsSectionId } from './labels';
	import { createDemoSettingsTransport } from './settings.fixtures';
	import { nativeSettingsTransport } from './transport';
	import type { Settings, SettingsTransport } from './types';
	import WdgAiModelsPanel from './WdgAiModelsPanel.svelte';
	import WdgBackupDefinitionsPanel from './WdgBackupDefinitionsPanel.svelte';
	import WdgPrivacyPanel from './WdgPrivacyPanel.svelte';
	import WdgSearchIndexPanel from './WdgSearchIndexPanel.svelte';
	import WdgUnsavedSettingsDialog from './WdgUnsavedSettingsDialog.svelte';

	interface Props {
		transport?: SettingsTransport;
		transferTransport?: TransferTransport;
		accounts?: AccountConfig[];
		guard?: SettingsFormGuard;
		initialSection?: SettingsSectionId;
		onBackToInbox?: () => void;
	}

	const defaultDemoTransport = createDemoSettingsTransport();

	let {
		transport = nativeAvailable() ? nativeSettingsTransport : defaultDemoTransport,
		transferTransport,
		accounts,
		guard = new SettingsFormGuard(),
		initialSection = 'general',
		onBackToInbox
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale]);

	const NAV_ITEMS: { id: SettingsSectionId; icon: typeof Settings2 }[] = [
		{ id: 'general', icon: Settings2 },
		{ id: 'search', icon: Database },
		{ id: 'ai', icon: Cpu },
		{ id: 'privacy', icon: Lock },
		{ id: 'backup', icon: ArchiveRestore }
	];

	let activeSection = $state<SettingsSectionId>(untrack(() => initialSection));
	let pendingAction = $state<(() => void) | null>(null);
	let unsavedDialogOpen = $state(false);
	let settings = $state<Settings | null>(null);
	let loadError = $state('');
	let active = true;
	let revision = 0;

	const resolvedAccounts = $derived.by<AccountConfig[]>(() => {
		if (accounts) return accounts;
		if (nativeAvailable()) return onboardingSession.accounts.accounts;
		return demoAutomationBackend.accounts;
	});

	async function loadSettings() {
		const current = ++revision;
		try {
			const next = await transport.get();
			if (active && current === revision) {
				settings = next;
				loadError = '';
				appStore.semanticSearchEnabled = next.experimental_semantic_search;
			}
		} catch {
			if (active && current === revision) loadError = t.loadError;
		}
	}

	async function handleReload() {
		await loadSettings();
		if (typeof window !== 'undefined') {
			window.dispatchEvent(new Event('triage-settings-updated'));
		}
	}

	$effect(() => {
		active = true;
		if (nativeAvailable() && !accounts && !onboardingSession.booted) {
			void onboardingSession.boot();
		}
		untrack(() => void loadSettings());
		return () => {
			active = false;
			revision++;
			guard.reset();
		};
	});

	function runGuarded(action: () => void) {
		if (guard.isBlocked()) return;
		if (guard.needsConfirmation()) {
			pendingAction = action;
			unsavedDialogOpen = true;
			return;
		}
		action();
	}

	function requestSectionSwitch(nextId: SettingsSectionId) {
		if (nextId === activeSection) return;
		runGuarded(() => {
			activeSection = nextId;
		});
	}

	function cancelSwitch() {
		pendingAction = null;
		unsavedDialogOpen = false;
	}

	function confirmDiscardAndSwitch() {
		const proceed = pendingAction;
		pendingAction = null;
		unsavedDialogOpen = false;
		guard.reset();
		proceed?.();
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-4 md:p-6">
	<div class="mx-auto flex w-full max-w-6xl flex-col gap-5">
		<header class="flex flex-wrap items-center justify-between gap-4 border-b border-separator pb-4">
			<div class="space-y-1">
				<h1 class="font-display text-lg font-semibold text-foreground">{t.pageTitle}</h1>
				<p class="text-xs text-secondary">{t.pageSubtitle}</p>
			</div>

			{#if onBackToInbox}
				<button
					type="button"
					onclick={() => onBackToInbox && runGuarded(onBackToInbox)}
					class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground hover:bg-field"
				>
					<ArrowLeft class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
					<span>{t.backToInbox}</span>
				</button>
			{/if}
		</header>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-[240px_minmax(0,1fr)] lg:items-start">
			<nav
				aria-label={t.navAria}
				class="flex flex-row gap-1 overflow-x-auto rounded-lg border border-separator bg-surface p-2 lg:flex-col"
			>
				{#each NAV_ITEMS as item (item.id)}
					{@const Icon = item.icon}
					{@const activeTab = activeSection === item.id}
					<button
						type="button"
						id={`tab-${item.id}`}
						aria-current={activeTab ? 'page' : undefined}
						onclick={() => requestSectionSwitch(item.id)}
						class="flex min-h-[40px] shrink-0 items-center justify-between gap-2 whitespace-nowrap rounded-md px-3 py-2 text-left text-xs font-medium transition-colors {activeTab
							? 'bg-selection text-foreground'
							: 'text-secondary hover:bg-field hover:text-foreground'}"
					>
						<span class="flex items-center gap-3">
							<Icon class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
							<span>{t.tabs[item.id]}</span>
						</span>
						{#if activeTab && guard.dirty}
							<span class="font-mono text-[10px] text-status-warning">{t.editedBadge}</span>
						{/if}
					</button>
				{/each}
			</nav>

			<div class="min-w-0">
				{#if activeSection === 'general'}
					<WdgPrivacyPanel section="general" {transport} />
				{:else if activeSection === 'privacy'}
					<WdgPrivacyPanel section="privacy" />
				{:else if activeSection === 'backup'}
					<WdgBackupDefinitionsPanel
						transport={transferTransport}
						accounts={resolvedAccounts}
						{guard}
					/>
				{:else if loadError}
					<div class="space-y-3 rounded-lg border border-separator bg-surface p-5">
						<p role="alert" class="text-xs font-medium text-status-danger">{loadError}</p>
						<button
							type="button"
							aria-label={t.retryLoading}
							onclick={() => void loadSettings()}
							class="min-h-[36px] rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-accent hover:bg-selection-soft"
						>
							{t.retryLoading}
						</button>
					</div>
				{:else if settings}
					{#if activeSection === 'search'}
						<WdgSearchIndexPanel {settings} {transport} onReload={handleReload} />
					{:else if activeSection === 'ai'}
						<WdgAiModelsPanel {settings} {transport} {guard} onReload={handleReload} />
					{/if}
				{:else}
					<div class="rounded-lg border border-separator bg-surface p-5">
						<p role="status" class="text-xs text-secondary">{t.loading}</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>

<WdgUnsavedSettingsDialog
	bind:open={unsavedDialogOpen}
	onStay={cancelSwitch}
	onDiscard={confirmDiscardAndSwitch}
/>
