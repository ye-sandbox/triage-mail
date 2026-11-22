<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { i18n } from '$lib/i18n/index.svelte';
	import { validTag } from './triage';
	import type { InboxWorkspace } from './workspace.svelte';

	type Props = {
		workspace: InboxWorkspace;
	};

	let { workspace }: Props = $props();

	let tagInput = $state('');
	let tagInputRef = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (workspace.tagDialogOpen) {
			tagInput = '';
		}
	});

	function submitTag(event: SubmitEvent) {
		event.preventDefault();
		const targetId = workspace.tagDialogTargetId;
		const trimmed = tagInput.trim();
		if (!targetId || !validTag(trimmed)) return;
		workspace.tagDialogOpen = false;
		void workspace.mutate(targetId, { Tag: trimmed });
	}
</script>

<Dialog.Root
	open={workspace.tagDialogOpen}
	onOpenChange={(next) => {
		workspace.tagDialogOpen = next;
		if (!next) tagInput = '';
	}}
>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			onOpenAutoFocus={(event) => {
				event.preventDefault();
				tagInputRef?.focus();
			}}
			class="fixed top-1/2 left-1/2 z-50 w-[min(92vw,420px)] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
		>
			<Dialog.Title class="font-display text-sm font-semibold">
				{i18n.t.mailboxes.tag}
			</Dialog.Title>
			<Dialog.Description class="mt-1 text-xs text-secondary">
				Stored on this device. Up to 128 UTF-8 bytes, with no control characters.
			</Dialog.Description>
			<form onsubmit={submitTag} class="mt-4 space-y-4">
				<div>
					<label for="local-tag" class="block text-xs font-medium text-foreground">
						Tag name
					</label>
					<input
						id="local-tag"
						bind:this={tagInputRef}
						bind:value={tagInput}
						autocomplete="off"
						aria-describedby="tag-help"
						placeholder="e.g. Receipts, Taxes, Follow-up"
						class="mt-2 h-9 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
					/>
					<p id="tag-help" class="mt-2 text-[11px] text-secondary">
						Up to 128 UTF-8 bytes, no control characters.
					</p>
				</div>
				<div class="flex justify-end gap-2">
					<button
						type="button"
						onclick={() => {
							workspace.tagDialogOpen = false;
						}}
						class="min-h-9 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground"
					>
						{i18n.t.mailboxes.close}
					</button>
					<button
						type="submit"
						disabled={!validTag(tagInput.trim())}
						class="min-h-9 rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground disabled:opacity-40"
					>
						Save tag
					</button>
				</div>
			</form>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
