<script lang="ts">
	import { AlertDialog } from 'bits-ui';
	import { i18n } from '$lib/i18n/index.svelte';
	import { settingsLabels } from './labels';

	interface Props {
		open: boolean;
		onStay: () => void;
		onDiscard: () => void;
	}

	let { open = $bindable(), onStay, onDiscard }: Props = $props();

	const t = $derived(settingsLabels[i18n.locale]);
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Portal>
		<AlertDialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<AlertDialog.Content
			class="fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-lg border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
		>
			<AlertDialog.Title class="font-display text-sm font-semibold text-foreground">
				{t.unsavedTitle}
			</AlertDialog.Title>
			<AlertDialog.Description class="mt-2 text-xs leading-relaxed text-secondary">
				{t.unsavedDescription}
			</AlertDialog.Description>
			<div class="mt-5 flex justify-end gap-2 border-t border-separator pt-3">
				<AlertDialog.Cancel
					onclick={onStay}
					class="min-h-[36px] rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground hover:bg-field"
				>
					{t.stay}
				</AlertDialog.Cancel>
				<AlertDialog.Action
					onclick={onDiscard}
					class="min-h-[36px] rounded-md bg-status-danger px-4 py-2 text-xs font-medium text-white hover:opacity-95"
				>
					{t.discard}
				</AlertDialog.Action>
			</div>
		</AlertDialog.Content>
	</AlertDialog.Portal>
</AlertDialog.Root>
