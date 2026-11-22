<script lang="ts">
	import { untrack } from 'svelte';
	import { Dialog } from 'bits-ui';
	import Bell from '@lucide/svelte/icons/bell';
	import X from '@lucide/svelte/icons/x';
	import { toast } from 'svelte-sonner';
	import { Drawer } from 'vaul-svelte';
	import { nativeAlertsTransport } from '$lib/alerts/transport';
	import type { AlertsTransport, Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { labels } from './labels';
	import { nativeRemindersTransport } from './transport';
	import {
		newReminderConfig,
		type RemindersTransport,
		type ScheduledReminderConfig
	} from './types';
	import WdgReminderRuleForm from './WdgReminderRuleForm.svelte';
	import WdgReminderRulesList from './WdgReminderRulesList.svelte';

	interface Props {
		open: boolean;
		remindersTransport?: RemindersTransport;
		alertsTransport?: AlertsTransport;
		accounts?: AccountConfig[];
		onConfigsChanged?: (configs: ScheduledReminderConfig[]) => void;
	}

	let {
		open = $bindable(false),
		remindersTransport = nativeRemindersTransport,
		alertsTransport = nativeAlertsTransport,
		accounts = [],
		onConfigsChanged
	}: Props = $props();

	const t = $derived(labels[i18n.locale]);

	let isMobile = $state(false);
	let configs = $state<ScheduledReminderConfig[]>([]);
	let destinations = $state<Destination[]>([]);
	let draft = $state<ScheduledReminderConfig | null>(null);
	let loading = $state(false);
	let busy = $state(false);
	let error = $state('');
	let loadGeneration = 0;

	$effect(() => {
		if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
		const mq = window.matchMedia('(max-width: 767px)');
		isMobile = mq.matches;
		const onChange = (e: MediaQueryListEvent) => (isMobile = e.matches);
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});

	async function load() {
		const gen = ++loadGeneration;
		loading = true;
		error = '';
		try {
			const [cfgList, destList] = await Promise.all([
				remindersTransport.listConfigs(),
				alertsTransport.list()
			]);
			if (gen !== loadGeneration) return;
			configs = cfgList;
			destinations = destList;
			onConfigsChanged?.(cfgList);
		} catch {
			if (gen === loadGeneration) error = t.error;
		} finally {
			if (gen === loadGeneration) loading = false;
		}
	}

	$effect(() => {
		if (open) {
			untrack(() => {
				draft = null;
				void load();
			});
		} else {
			loadGeneration++;
			draft = null;
			error = '';
		}
	});

	function startCreate() {
		error = '';
		const fresh = newReminderConfig();
		if (destinations.length > 0) {
			fresh.destination_ids = [destinations[0].id];
		}
		draft = fresh;
	}

	async function handleSave(next: ScheduledReminderConfig) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			await remindersTransport.saveConfig(next);
			draft = null;
			await load();
			toast.success(t.ruleSavedToast, { description: next.name });
		} catch {
			error = t.error;
		} finally {
			busy = false;
		}
	}

	async function toggleEnabled(cfg: ScheduledReminderConfig) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			const plain = $state.snapshot(cfg);
			await remindersTransport.saveConfig({ ...plain, enabled: !plain.enabled });
			await load();
		} catch {
			error = t.error;
		} finally {
			busy = false;
		}
	}

	async function handleDelete(cfg: ScheduledReminderConfig) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			await remindersTransport.deleteConfig(cfg.id, cfg.revision);
			if (draft?.id === cfg.id) draft = null;
			await load();
			toast.success(t.ruleDeletedToast, { description: cfg.name });
		} catch {
			error = t.error;
		} finally {
			busy = false;
		}
	}
</script>

{#snippet drawerBody()}
	<div class="flex flex-col gap-4 text-xs">
		<p class="leading-relaxed text-[11px] text-secondary">{t.rulesNotice}</p>

		{#if error && !draft}
			<div
				role="alert"
				class="rounded-md border border-status-danger/30 bg-status-danger/10 px-3 py-2 text-xs text-status-danger"
			>
				{error}
			</div>
		{/if}

		{#if draft}
			<WdgReminderRuleForm
				initial={draft}
				{destinations}
				{accounts}
				{busy}
				{error}
				onSave={handleSave}
				onCancel={() => (draft = null)}
			/>
		{:else}
			<WdgReminderRulesList
				{configs}
				{loading}
				{busy}
				onCreate={startCreate}
				onEdit={(cfg) => {
					error = '';
					draft = structuredClone($state.snapshot(cfg));
				}}
				onToggleEnabled={toggleEnabled}
				onDelete={handleDelete}
			/>
		{/if}
	</div>
{/snippet}

{#if isMobile}
	<Drawer.Root bind:open>
		<Drawer.Portal>
			<Drawer.Overlay class="fixed inset-0 z-50 bg-foreground/30" />
			<Drawer.Content
				class="fixed inset-x-0 bottom-0 z-50 max-h-[85vh] overflow-y-auto rounded-t-xl border-t border-separator bg-surface p-4 outline-none"
			>
				<div class="mx-auto mb-3 h-1.5 w-10 rounded-full bg-separator"></div>
				<div class="mb-3 flex items-center justify-between">
					<Drawer.Title class="flex items-center gap-2 font-display text-sm font-semibold text-foreground">
						<Bell class="h-4 w-4 text-accent" aria-hidden="true" />
						<span>{t.configureSchedule}</span>
					</Drawer.Title>
					<button
						type="button"
						aria-label={t.close}
						onclick={() => {
							if (!busy) open = false;
						}}
						class="rounded-md p-1 text-secondary hover:text-foreground"
					>
						<X class="h-4 w-4" aria-hidden="true" />
					</button>
				</div>
				{@render drawerBody()}
			</Drawer.Content>
		</Drawer.Portal>
	</Drawer.Root>
{:else}
	<Dialog.Root bind:open>
		<Dialog.Portal>
			<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
			<Dialog.Content
				class="fixed top-1/2 left-1/2 z-50 max-h-[85vh] w-[calc(100%-2rem)] max-w-lg -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border border-separator bg-surface p-5 shadow-xl outline-none"
			>
				<div class="mb-4 flex items-center justify-between">
					<Dialog.Title class="flex items-center gap-2 font-display text-sm font-semibold text-foreground">
						<Bell class="h-4 w-4 text-accent" aria-hidden="true" />
						<span>{t.configureSchedule}</span>
					</Dialog.Title>
					<button
						type="button"
						aria-label={t.close}
						onclick={() => {
							if (!busy) open = false;
						}}
						class="rounded-md p-1 text-secondary hover:text-foreground"
					>
						<X class="h-4 w-4" aria-hidden="true" />
					</button>
				</div>
				{@render drawerBody()}
			</Dialog.Content>
		</Dialog.Portal>
	</Dialog.Root>
{/if}
