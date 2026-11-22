<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { Dialog, Tabs } from 'bits-ui';
	import X from '@lucide/svelte/icons/x';
	import type { SchedulePolicy, ScheduleTransport } from '$lib/alerts/schedule';
	import type { AlertsTransport, Destination } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SettingsFormGuard } from '$lib/settings/guards';
	import { automationLabels } from './labels';
	import WdgAlertDestinationForm from './WdgAlertDestinationForm.svelte';
	import WdgAlertScheduleForm from './WdgAlertScheduleForm.svelte';

	interface Props {
		open: boolean;
		initialTab?: 'destination' | 'schedule';
		destination: Destination | null;
		alertsTransport: AlertsTransport;
		scheduleTransport: ScheduleTransport;
		guard?: SettingsFormGuard;
		onSaved: (saved: Destination, isNew: boolean) => void | Promise<void>;
		onScheduleSaved?: (saved: SchedulePolicy) => void | Promise<void>;
	}

	let {
		open = $bindable(),
		initialTab = 'destination',
		destination,
		alertsTransport,
		scheduleTransport,
		guard,
		onSaved,
		onScheduleSaved
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);

	let activeTab = $state<'destination' | 'schedule'>('destination');
	let busy = $state(false);

	onDestroy(() => {
		guard?.markDirty(false);
		guard?.setBusy(false);
	});

	$effect(() => {
		if (open) {
			untrack(() => {
				activeTab = initialTab;
				guard?.markDirty(true);
			});
		} else {
			untrack(() => {
				guard?.markDirty(false);
			});
		}
	});

	function closeDialog() {
		guard?.markDirty(false);
		open = false;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			class="fixed top-1/2 left-1/2 z-50 max-h-[90vh] w-[calc(100%-2rem)] max-w-xl -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border border-separator bg-surface p-5 shadow-xl outline-none"
		>
			<div class="mb-4 flex items-center justify-between border-b border-separator pb-3">
				<Dialog.Title class="font-display text-sm font-semibold text-foreground">
					{destination && destination.revision > 0
						? t.dialogTitleEdit(destination.revision)
						: t.dialogTitleNew}
				</Dialog.Title>
				<Dialog.Close
					onclick={closeDialog}
					class="rounded p-1 text-secondary hover:bg-field hover:text-foreground"
					aria-label={t.closeDialogAria}
				>
					<X class="h-4 w-4" aria-hidden="true" />
				</Dialog.Close>
			</div>

			<Tabs.Root bind:value={activeTab} class="flex flex-col gap-4">
				<Tabs.List class="grid w-full grid-cols-2 gap-1 rounded-md border border-separator bg-field p-1">
					<Tabs.Trigger
						value="destination"
						class="min-h-[36px] rounded px-3 py-1 text-xs font-medium text-secondary transition-colors data-[state=active]:bg-surface data-[state=active]:text-foreground data-[state=active]:shadow-xs"
					>
						{t.tabDestination}
					</Tabs.Trigger>
					<Tabs.Trigger
						value="schedule"
						class="min-h-[36px] rounded px-3 py-1 text-xs font-medium text-secondary transition-colors data-[state=active]:bg-surface data-[state=active]:text-foreground data-[state=active]:shadow-xs"
					>
						{t.tabSchedule}
					</Tabs.Trigger>
				</Tabs.List>

				<Tabs.Content value="destination" class="outline-none">
					<WdgAlertDestinationForm
						{open}
						{destination}
						{alertsTransport}
						{guard}
						bind:busy
						{onSaved}
						onClose={closeDialog}
					/>
				</Tabs.Content>

				<Tabs.Content value="schedule" class="outline-none">
					<WdgAlertScheduleForm
						{open}
						{destination}
						{scheduleTransport}
						{guard}
						bind:busy
						onSwitchToDestination={() => (activeTab = 'destination')}
						{onScheduleSaved}
						onClose={closeDialog}
					/>
				</Tabs.Content>
			</Tabs.Root>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
