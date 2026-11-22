<script lang="ts">
	import { goto } from '$app/navigation';
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
	import type { RemindersTransport } from '$lib/reminders/types';
	import { nativeSettingsTransport } from '$lib/settings/transport';
	import type { SettingsTransport } from '$lib/settings/types';
	import { appStore } from '$lib/stores/app.svelte';
	import type { AccountController } from './controller.svelte';
	import {
		refreshDueDateBadgeCount,
		syncSavedViewsFromTransport,
		syncSettingsFromTransport
	} from './inbox-session';

	let {
		connected,
		inboxTransport = nativeTransport,
		settingsTransport = nativeSettingsTransport,
		remindersTransport = nativeRemindersTransport,
		accountController,
		onNavigate
	}: {
		connected: boolean;
		inboxTransport?: InboxTransport;
		settingsTransport?: SettingsTransport;
		remindersTransport?: RemindersTransport;
		accountController?: AccountController;
		onNavigate?: (path: string) => void | Promise<void>;
	} = $props();

	function navigateTo(path: string) {
		if (onNavigate) {
			void Promise.resolve(onNavigate(path)).catch(() => {});
			return;
		}
		try {
			void goto(path).catch(() => {});
		} catch {
			// Unit tests mount InboxSession without booting SvelteKit's client router.
		}
	}

	const workspace = untrack(() =>
		connected
			? createLiveWorkspace(new InboxController(inboxTransport), {
					connected: true,
					settingsTransport,
					accounts: accountController?.accounts ?? [],
					createRule: (id: string) => {
						appStore.requestCreateRule(id);
						navigateTo('/automation');
					}
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
				const pendingId = appStore.pendingSelectMessageId;
				if (pendingId) workspace.ensureDemoDueDateMessage(pendingId);
				void workspace.applyShellScope(appStore.activeScope).then(() => {
					const targetId = appStore.consumePendingSelectMessage();
					if (targetId) workspace.selectMessage(targetId, true);
				});
			});
			return () => {
				inbox.dispose();
				appStore.registerShellHooks({});
				if (workspaceRegistry.current === workspace) workspaceRegistry.setActive(null);
			};
		}

		untrack(() => {
			if (appStore.pendingSelectMessageId) {
				appStore.activeScope = { kind: 'all' };
				inbox.scope = 'GlobalAllMail';
			}
			void inbox.connect();
			syncSettingsFromTransport(connected, settingsTransport, workspace);
			syncSavedViewsFromTransport(connected, inboxTransport, workspace);
			if (nativeAvailable()) refreshDueDateBadgeCount(remindersTransport, workspace);
		});

		const onSettingsUpdated = () =>
			syncSettingsFromTransport(connected, settingsTransport, workspace);
		const onDefinitionsImported = () =>
			syncSavedViewsFromTransport(connected, inboxTransport, workspace);
		window.addEventListener('triage-settings-updated', onSettingsUpdated);
		window.addEventListener('triage-definitions-imported', onDefinitionsImported);

		return () => {
			window.removeEventListener('triage-settings-updated', onSettingsUpdated);
			window.removeEventListener('triage-definitions-imported', onDefinitionsImported);
			inbox.dispose();
			appStore.registerShellHooks({});
			if (workspaceRegistry.current === workspace) workspaceRegistry.setActive(null);
		};
	});

	$effect(() => {
		const pendingId = appStore.pendingSelectMessageId;
		const loaded = inbox.revision > 0 && !inbox.loading;
		const rows = inbox.rows;
		if (!connected || !pendingId || !loaded) return;
		untrack(() => {
			const targetId = appStore.consumePendingSelectMessage();
			if (targetId && rows.some((row) => row.id === targetId)) {
				workspace.selectMessage(targetId, true);
			}
		});
	});

	$effect(() => {
		if (!connected || inbox.badgeRevision === 0) return;
		untrack(() => refreshDueDateBadgeCount(remindersTransport, workspace));
	});

	$effect(() => {
		if (!connected || !inbox.savedView) return;
		void inbox.badgeRevision;
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

	async function handleBackToList() {
		workspace.closeMobileReader();
		await tick();
		workspace.listElement?.focus();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (appStore.commandOpen || workspace.snoozeOpen || workspace.tagDialogOpen || inbox.mutating || inbox.loading) return;
		const selected = workspace.selected;
		if (!selected) return;
		const action = shortcut(event);
		if (!action || action === 'palette') return;
		event.preventDefault();
		if (action === 'tag') workspace.openTagDialogFor(selected.id);
		else void workspace.mutate(selected.id, action);
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} onfocus={() => connected && void inbox.refresh()} />

<div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-background text-foreground">
	<WdgInboxStatsBar {workspace} openSettings={() => navigateTo('/settings')} />

	{#if inbox.mutationError}
		<div role="alert" class="border-b border-separator bg-field px-4 py-2 text-xs text-status-danger">
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
