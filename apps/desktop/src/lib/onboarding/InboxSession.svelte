<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { nativeAvailable, nativeTransport } from '$lib/inbox/transport';
	import { shortcut } from '$lib/inbox/triage';
	import type { InboxTransport } from '$lib/inbox/types';
	import WdgInboxStatsBar from '$lib/inbox/WdgInboxStatsBar.svelte';
	import WdgMessageList from '$lib/inbox/WdgMessageList.svelte';
	import WdgMessageReader from '$lib/inbox/WdgMessageReader.svelte';
	import {
		createDemoWorkspace,
		createLiveWorkspace,
		toShellAccounts,
		workspaceRegistry
	} from '$lib/inbox/workspace.svelte';
	import { nativeRemindersTransport } from '$lib/reminders/transport';
	import { calculateDueDateStatus, type RemindersTransport } from '$lib/reminders/types';
	import DueDateDashboard from '$lib/reminders/DueDateDashboard.svelte';
	import SettingsDialog from '$lib/settings/SettingsDialog.svelte';
	import { nativeSettingsTransport } from '$lib/settings/transport';
	import type { SettingsTab, SettingsTransport } from '$lib/settings/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { AccountController } from './controller.svelte';

	let {
		connected,
		inboxTransport = nativeTransport,
		settingsTransport = nativeSettingsTransport,
		remindersTransport = nativeRemindersTransport,
		accountController
	}: {
		connected: boolean;
		inboxTransport?: InboxTransport;
		settingsTransport?: SettingsTransport;
		remindersTransport?: RemindersTransport;
		accountController?: AccountController;
	} = $props();

	let settingsOpen = $state(false);
	let dueDateDashboardOpen = $state(false);
	let settingsTab = $state<SettingsTab>('general');
	let ruleSeedMessageId = $state<string | undefined>();

	function openSettings() {
		settingsTab = 'general';
		settingsOpen = true;
	}

	function createRule(id: string) {
		ruleSeedMessageId = id;
		settingsTab = 'rules';
		settingsOpen = true;
	}

	const workspace = untrack(() =>
		connected
			? createLiveWorkspace(new InboxController(inboxTransport), {
					connected: true,
					settingsTransport,
					accounts: accountController?.accounts ?? [],
					createRule
				})
			: createDemoWorkspace()
	);

	const inbox = workspace.inbox;

	$effect(() => {
		const list = accountController?.accounts ?? [];
		workspace.accounts = list;
		if (connected && list.length) {
			appStore.accounts = toShellAccounts(list);
		}
		if (connected) {
			untrack(() => {
				const scope = inbox.scope;
				if (
					scope &&
					scope !== 'GlobalAllMail' &&
					!list.some((a) => a.account_id === scope.AccountView.account_id) &&
					list.length > 0
				) {
					void inbox.selectScope();
				}
			});
		}
	});

	$effect(() => {
		workspaceRegistry.setActive(workspace);
		appStore.registerShellHooks({
			onScopeChange: (scope) => void workspace.applyShellScope(scope),
			onTriggerSync: () => void inbox.refresh()
		});

		if (!connected) {
			untrack(() => {
				void workspace.applyShellScope(appStore.activeScope);
			});
			return () => {
				inbox.dispose();
				appStore.registerShellHooks({});
				if (workspaceRegistry.current === workspace) workspaceRegistry.setActive(null);
			};
		}

		untrack(() => {
			void inbox.connect();
			void settingsTransport
				.get()
				.then((settings) => {
					inbox.setExperimentalSemanticSearch(settings.experimental_semantic_search);
				})
				.catch(() => {});

			if (nativeAvailable()) {
				void inbox.savedTransport
					?.list()
					.then((views) => {
						appStore.savedViews = views.map((v) => ({
							id: v.id,
							name: v.name,
							query: v.query
						}));
					})
					.catch(() => {});

				void remindersTransport
					.listDueDates()
					.then((items) => {
						const active = items.filter((item) => {
							const st = calculateDueDateStatus(item);
							return st === 'Overdue' || st === 'DueToday';
						});
						workspace.dueCount = active.length;
						appStore.overdueAndDueTodayCount = active.length;
					})
					.catch(() => {});
			}
		});

		return () => {
			inbox.dispose();
			appStore.registerShellHooks({});
			if (workspaceRegistry.current === workspace) workspaceRegistry.setActive(null);
		};
	});

	$effect(() => {
		if (!connected || !inbox.savedView) return;
		const revision = inbox.badgeRevision;
		void revision;
		const now = Date.now();
		const timer = setTimeout(() => void inbox.refresh(), 86400000 - (now % 86400000) + 1);
		return () => clearTimeout(timer);
	});

	$effect(() => {
		const deadlines = inbox.rows
			.map((r) => r.snooze_until_epoch_ms ?? 0)
			.filter((n) => n > Date.now());
		if (!deadlines.length || !connected) return;
		const timer = setTimeout(
			() => void inbox.refresh(),
			Math.min(2147483647, deadlines.reduce((a, b) => Math.min(a, b)) - Date.now() + 1)
		);
		return () => clearTimeout(timer);
	});

	$effect(() => {
		inbox.rows;
		inbox.totalEmbeddingErrors;
		inbox.experimentalSemanticSearch;
		inbox.syncStatus;
		workspace.dueCount;
		untrack(() => workspace.syncShellMetrics());
	});

	function handleCloseSettings() {
		settingsOpen = false;
		ruleSeedMessageId = undefined;
		void settingsTransport
			.get()
			.then((settings) => {
				inbox.setExperimentalSemanticSearch(settings.experimental_semantic_search);
			})
			.catch(() => {});
	}

	async function handleBackToList() {
		workspace.closeMobileReader();
		await tick();
		workspace.listElement?.focus();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (
			settingsOpen ||
			dueDateDashboardOpen ||
			appStore.commandOpen ||
			workspace.snoozeOpen ||
			workspace.tagDialogOpen ||
			inbox.mutating ||
			inbox.loading
		) {
			return;
		}
		const selected = workspace.selected;
		if (!selected) return;
		const action = shortcut(event);
		if (!action || action === 'palette') return;
		event.preventDefault();
		if (action === 'tag') {
			workspace.openTagDialogFor(selected.id);
		} else {
			void workspace.mutate(selected.id, action);
		}
	}
</script>

<svelte:window
	onkeydown={handleWindowKeydown}
	onfocus={() => {
		if (connected) void inbox.refresh();
	}}
/>

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-background text-foreground">
	<WdgInboxStatsBar {workspace} {openSettings} />

	{#if inbox.mutationError}
		<div
			role="alert"
			class="border-b border-separator bg-field px-4 py-2 text-xs text-status-danger"
		>
			{inbox.mutationError}
		</div>
	{/if}
	{#if inbox.mutationNotice}
		<div role="status" class="border-b border-separator bg-field px-4 py-2 text-xs text-secondary">
			{inbox.mutationNotice}
		</div>
	{/if}

	<div class="flex min-h-0 flex-1 overflow-hidden">
		<div
			class="min-h-0 w-full shrink-0 border-separator md:flex md:w-[340px] md:flex-col md:border-r lg:w-[380px] {workspace.mobileReaderOpen
				? 'hidden'
				: 'flex flex-col'}"
		>
			<WdgMessageList {workspace} bind:listElement={workspace.listElement} />
		</div>

		<div
			class="min-w-0 flex-1 flex-col overflow-hidden md:flex {workspace.mobileReaderOpen
				? 'flex'
				: 'hidden'}"
		>
			<WdgMessageReader {workspace} onBackToList={() => void handleBackToList()} />
		</div>
	</div>
</div>

<DueDateDashboard
	open={dueDateDashboardOpen}
	close={() => (dueDateDashboardOpen = false)}
	transport={remindersTransport}
	accounts={accountController?.accounts ?? []}
	onSelectMessage={(msgId: string) => {
		dueDateDashboardOpen = false;
		workspace.selectMessage(msgId, true);
	}}
	onConfigureReminders={() => {
		dueDateDashboardOpen = false;
		settingsTab = 'reminders';
		settingsOpen = true;
	}}
/>

<SettingsDialog
	open={settingsOpen}
	initialTab={settingsTab}
	{ruleSeedMessageId}
	selectedMessageIds={inbox.selectedId ? [inbox.selectedId] : []}
	close={handleCloseSettings}
	transport={settingsTransport}
	{accountController}
/>
