<script lang="ts">
	import { onMount } from 'svelte';
	import { Dialog } from 'bits-ui';
	import Clock from '@lucide/svelte/icons/clock';
	import { Drawer } from 'vaul-svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { TriageAction } from './triage';
	import { workspaceRegistry, type InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace?: InboxWorkspace | null;
		open?: boolean;
		onExecute?: (action: TriageAction) => void;
	};

	let { workspace, open = undefined, onExecute }: Props = $props();

	let deadline = $state('');
	let invalid = $state(false);
	let isMobile = $state(false);
	let deadlineInput = $state<HTMLInputElement | null>(null);

	const activeWorkspace = $derived(workspace ?? workspaceRegistry.active);
	const sheetOpen = $derived(open !== undefined ? open : (activeWorkspace?.snoozeOpen ?? false));
	const tMailboxes = $derived(i18n.t.mailboxes);

	onMount(() => {
		const check = () => {
			isMobile = window.innerWidth < 768;
		};
		check();
		window.addEventListener('resize', check);
		return () => window.removeEventListener('resize', check);
	});

	$effect(() => {
		if (sheetOpen) {
			invalid = false;
			deadline = '';
		}
	});

	function setOpen(next: boolean) {
		if (open !== undefined) {
			open = next;
		}
		if (activeWorkspace) {
			activeWorkspace.snoozeOpen = next;
		}
	}

	function presets() {
		const now = new Date();
		const laterToday = new Date(now);
		laterToday.setHours(18, 0, 0, 0);
		if (laterToday.getTime() <= now.getTime()) {
			laterToday.setTime(now.getTime() + 3 * 3600000);
		}
		const tomorrow = new Date(now);
		tomorrow.setDate(now.getDate() + 1);
		tomorrow.setHours(9, 0, 0, 0);

		const nextMonday = new Date(now);
		const day = nextMonday.getDay();
		const offset = day === 0 ? 1 : 8 - day;
		nextMonday.setDate(now.getDate() + offset);
		nextMonday.setHours(9, 0, 0, 0);

		return [
			{ id: 'later', label: 'Later Today', target: laterToday },
			{ id: 'tomorrow', label: 'Tomorrow 09:00', target: tomorrow },
			{ id: 'monday', label: 'Next Monday 09:00', target: nextMonday }
		];
	}

	function dispatchSnooze(until: number) {
		const now = Date.now();
		if (!Number.isFinite(until) || until <= now || until > now + 365 * 86400000) {
			invalid = true;
			return;
		}
		invalid = false;
		// INVARIANT: datetime-local is interpreted in the user's timezone; IPC always receives UTC milliseconds.
		const action: TriageAction = { Snooze: { until_epoch_ms: until } };
		setOpen(false);
		if (onExecute) {
			onExecute(action);
		} else if (activeWorkspace?.snoozeTargetId) {
			void activeWorkspace.mutate(activeWorkspace.snoozeTargetId, action);
		}
	}

	function formatPresetTime(date: Date): string {
		return date.toLocaleString(i18n.locale, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function submitCustom(event: SubmitEvent) {
		event.preventDefault();
		dispatchSnooze(new Date(deadline).getTime());
	}
</script>

{#snippet snoozeBody()}
	<div class="space-y-4">
		<div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
			{#each presets() as preset (preset.id)}
				<button
					type="button"
					onclick={() => dispatchSnooze(preset.target.getTime())}
					class="flex min-h-11 flex-col items-start justify-center rounded-lg border border-separator bg-field px-3 py-2 text-left transition-colors hover:bg-selection-soft md:min-h-0"
				>
					<span class="text-xs font-medium text-foreground">{preset.label}</span>
					<span class="font-mono text-[10px] whitespace-nowrap tabular-nums text-secondary">
						{formatPresetTime(preset.target)}
					</span>
				</button>
			{/each}
		</div>

		<form onsubmit={submitCustom} class="space-y-3 border-t border-separator pt-3">
			<label class="block text-xs font-medium text-foreground">
				{tMailboxes.deadline}
				<input
					type="datetime-local"
					bind:this={deadlineInput}
					bind:value={deadline}
					required
					class="mt-2 block h-9 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
				/>
			</label>
			{#if invalid}
				<p role="alert" class="text-xs text-status-danger">{tMailboxes.invalidDeadline}</p>
			{/if}
			<div class="flex justify-end gap-2">
				<button
					type="button"
					onclick={() => setOpen(false)}
					class="min-h-11 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground md:min-h-8"
				>
					{tMailboxes.close}
				</button>
				<button
					type="submit"
					class="min-h-11 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground md:min-h-8"
				>
					{tMailboxes.saveSnooze}
				</button>
			</div>
		</form>
	</div>
{/snippet}

{#if isMobile}
	<Drawer.Root open={sheetOpen} onOpenChange={setOpen}>
		<Drawer.Portal>
			<Drawer.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
			<Drawer.Content
				class="fixed inset-x-0 bottom-0 z-50 flex flex-col gap-3 rounded-t-xl border-t border-separator bg-surface p-5 text-foreground outline-none"
			>
				<div class="space-y-1">
					<div class="flex items-center gap-2">
						<Clock class="h-4 w-4 text-status-warning" aria-hidden="true" />
						<h2 class="font-display text-sm font-semibold">{tMailboxes.snooze}</h2>
					</div>
					<p class="text-xs text-secondary">{tMailboxes.snoozeHelp}</p>
				</div>
				{@render snoozeBody()}
			</Drawer.Content>
		</Drawer.Portal>
	</Drawer.Root>
{:else}
	<Dialog.Root open={sheetOpen} onOpenChange={setOpen}>
		<Dialog.Portal>
			<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
			<Dialog.Content
				onOpenAutoFocus={(event) => {
					event.preventDefault();
					deadlineInput?.focus();
				}}
				class="fixed top-1/2 left-1/2 z-50 w-[min(92vw,520px)] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
			>
				<Dialog.Title class="flex items-center gap-2 font-display text-sm font-semibold">
					<Clock class="h-4 w-4 text-status-warning" aria-hidden="true" />
					<span>{tMailboxes.snooze}</span>
				</Dialog.Title>
				<Dialog.Description class="mt-1 mb-4 text-xs text-secondary">
					{tMailboxes.snoozeHelp}
				</Dialog.Description>
				{@render snoozeBody()}
			</Dialog.Content>
		</Dialog.Portal>
	</Dialog.Root>
{/if}
