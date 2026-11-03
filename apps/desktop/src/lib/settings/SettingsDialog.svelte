<script lang="ts">
	import { untrack } from 'svelte';
	import type { Settings, SettingsTab, SettingsTransport } from './types';
	import { SETTINGS_TABS } from './types';
	import { nativeSettingsTransport } from './transport';
	import type { AccountController } from '../onboarding/controller.svelte';
	import { SettingsFormGuard } from './guards';
	import { i18n } from '../i18n/index.svelte';
	import GeneralModule from './modules/GeneralModule.svelte';
	import AccountsModule from './modules/AccountsModule.svelte';
	import SearchModule from './modules/SearchModule.svelte';
	import AiModule from './modules/AiModule.svelte';
	import AlertsModule from '../alerts/AlertsModule.svelte';
	import RulesModule from '../rules/RulesModule.svelte';
	import RemindersModule from '../reminders/RemindersModule.svelte';
	import PrivacyModule from './modules/PrivacyModule.svelte';

	let {
		open = false,
		close,
		initialTab = 'general',
		transport = nativeSettingsTransport,
		accountController,
		guard = new SettingsFormGuard(),
		selectedMessageIds = [],
		ruleSeedMessageId
	}: {
		open?: boolean;
		close: () => void;
		initialTab?: SettingsTab;
		transport?: SettingsTransport;
		accountController?: AccountController;
		guard?: SettingsFormGuard;
		selectedMessageIds?: string[];
		ruleSeedMessageId?: string;
	} = $props();

	let activeTab = $state<SettingsTab>('general');
	let settings = $state<Settings | null>(null);
	let error = $state('');
	let promptVisible = $state(false);
	let pendingProceed = $state<(() => void) | null>(null);
	let active = true;
	let revision = 0;

	const t = $derived(i18n.t.settingsModal);

	$effect(() => {
		if (open) {
			activeTab = initialTab;
		}
	});

	async function load() {
		const current = ++revision;
		try {
			const result = await transport.get();
			if (active && current === revision) {
				settings = result;
				error = '';
			}
		} catch {
			if (active && current === revision) {
				error = 'Settings could not be loaded.';
			}
		}
	}

	$effect(() => {
		if (open) {
			active = true;
			untrack(() => void load());
		}
		return () => {
			active = false;
			revision++;
		};
	});

	function requestAction(action: () => void) {
		if (guard.isBlocked()) {
			return;
		}
		if (guard.needsConfirmation()) {
			pendingProceed = action;
			promptVisible = true;
			return;
		}
		action();
	}

	function requestClose() {
		requestAction(() => close());
	}

	function requestTabChange(tabId: SettingsTab) {
		requestAction(() => {
			activeTab = tabId;
		});
	}

	function handleStay() {
		promptVisible = false;
		pendingProceed = null;
	}

	function handleDiscard() {
		promptVisible = false;
		guard.reset();
		const proceed = pendingProceed;
		pendingProceed = null;
		proceed?.();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			requestClose();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4"
		onclick={(event) => {
			if (event.target === event.currentTarget) requestClose();
		}}
		role="presentation"
	>
		<div
			role="dialog"
			aria-modal="true"
			aria-labelledby="settings-dialog-title"
			class="relative flex h-[80vh] max-h-[700px] w-full max-w-4xl flex-col rounded-xl border border-[var(--separator)] bg-[var(--reader)] shadow-2xl overflow-hidden text-[var(--text)]"
		>
			{#if promptVisible}
				<div
					class="absolute inset-0 z-50 flex items-center justify-center bg-black/60 p-6"
					role="alertdialog"
					aria-modal="true"
					aria-labelledby="unsaved-prompt-title"
				>
					<div
						class="max-w-md space-y-4 rounded-xl border border-[var(--separator)] bg-[var(--surface)] p-6 shadow-2xl text-[var(--text)]"
					>
						<h3 id="unsaved-prompt-title" class="text-base font-semibold">
							{t.unsavedChanges}
						</h3>
						<div class="flex justify-end gap-3 pt-2">
							<button
								type="button"
								class="rounded-lg border border-[var(--separator)] px-4 py-2 text-sm font-medium hover:bg-[var(--reader)]"
								onclick={handleStay}
							>
								{t.stay}
							</button>
							<button
								type="button"
								class="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700"
								onclick={handleDiscard}
							>
								{t.discard}
							</button>
						</div>
					</div>
				</div>
			{/if}
			<header
				class="flex items-center justify-between border-b border-[var(--separator)] px-6 py-4"
			>
				<h1 id="settings-dialog-title" class="text-xl font-semibold">
					{t.title}
				</h1>
				<button
					type="button"
					aria-label={t.close}
					onclick={requestClose}
					class="rounded-lg p-1.5 text-[var(--secondary)] hover:bg-[var(--surface)] hover:text-[var(--text)]"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-5 w-5"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
						aria-hidden="true"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M6 18L18 6M6 6l12 12"
						/>
					</svg>
				</button>
			</header>

			<div class="flex flex-1 min-h-0">
				<div
					role="tablist"
					aria-label={t.navAria}
					class="w-48 shrink-0 border-r border-[var(--separator)] bg-[var(--surface)] p-3 space-y-1"
				>
					{#each SETTINGS_TABS as tab}
						<button
							role="tab"
							id={`tab-${tab.id}`}
							aria-selected={activeTab === tab.id}
							aria-controls={`panel-${tab.id}`}
							type="button"
							onclick={() => requestTabChange(tab.id)}
							class="w-full text-left rounded-lg px-3 py-2 text-sm font-medium transition-colors {activeTab ===
							tab.id
								? 'bg-[var(--selection-soft)] text-[var(--accent)]'
								: 'text-[var(--secondary)] hover:bg-[var(--reader)] hover:text-[var(--text)]'}"
						>
							{t.tabs[tab.id] ?? tab.label}
						</button>
					{/each}
				</div>

				<div
					role="tabpanel"
					id={`panel-${activeTab}`}
					aria-labelledby={`tab-${activeTab}`}
					class="flex-1 overflow-y-auto p-6"
				>
					{#if activeTab === 'accounts'}
						{#if accountController}
							<AccountsModule controller={accountController} />
						{:else}
							<p class="text-sm text-[var(--secondary)]">Account manager unavailable.</p>
						{/if}
					{:else if activeTab === 'alerts'}
						<AlertsModule {guard} selectedIds={selectedMessageIds} />
					{:else if activeTab === 'reminders'}
						<RemindersModule {guard} accounts={accountController?.accounts ?? []} />
					{:else if activeTab === 'rules'}
						<RulesModule
							accounts={accountController?.accounts ?? []}
							{guard}
							selectedIds={selectedMessageIds}
							seedMessageId={ruleSeedMessageId}
						/>
					{:else if error}
						<div class="space-y-4">
							<p role="alert" class="text-sm text-red-500">{error}</p>
							<button
								type="button"
								aria-label="Retry loading settings"
								onclick={() => void load()}
								class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-sm text-[var(--accent)]"
							>
								Retry loading settings
							</button>
						</div>
					{:else if settings}
						{#if activeTab === 'general'}
							<GeneralModule {settings} {transport} onReload={load} />
						{:else if activeTab === 'search'}
							<SearchModule {settings} {transport} onReload={load} />
						{:else if activeTab === 'ai'}
							<AiModule {settings} {transport} onReload={load} />
						{:else if activeTab === 'privacy'}
							<PrivacyModule {settings} {transport} onReload={load} />
						{/if}
					{:else}
						<p role="status" class="text-sm text-[var(--secondary)]">Loading settings…</p>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
