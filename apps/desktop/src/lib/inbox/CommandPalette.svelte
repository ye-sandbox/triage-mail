<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { i18n } from '../i18n/index.svelte';
	import SnoozeForm from './SnoozeForm.svelte';
	import { tick, untrack } from 'svelte';
	import { mailboxCommands, type PaletteCommand } from './commands';
	import { validTag, type TriageAction } from './triage';
	let {
		open = $bindable(false),
		tagging = $bindable(false),
		snoozing = $bindable(false),
		subject,
		canUnsubscribe,
		execute,
		restoreFocus
	}: {
		open?: boolean;
		tagging?: boolean;
		snoozing?: boolean;
		subject: string;
		canUnsubscribe: boolean;
		execute: (action: TriageAction) => void;
		restoreFocus: () => void;
	} = $props();
	let query = $state('');
	let tag = $state('');
	let index = $state(0);
	let input = $state<HTMLInputElement>();
	$effect(() => {
		if (open) untrack(reset);
	});
	const matches = $derived(
		mailboxCommands().filter((command) =>
			command.label.toLowerCase().includes(query.trim().toLowerCase())
		)
	);
	function choose(command: PaletteCommand | undefined) {
		if (!command) return;
		if (command.action === 'snooze') {
			snoozing = true;
			return;
		}
		if (command.action === 'tag') {
			tagging = true;
			void tick().then(() => input?.focus());
			return;
		}
		if (command.action === 'OneClickUnsubscribe' && !canUnsubscribe) return;
		open = false;
		execute(command.action);
	}
	function navigate(event: KeyboardEvent) {
		if (tagging || snoozing || event.isComposing) return;
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			index =
				(index + (event.key === 'ArrowDown' ? 1 : -1) + matches.length) % (matches.length || 1);
		} else if (event.key === 'Enter') {
			event.preventDefault();
			choose(matches[index]);
		}
	}
	function reset() {
		query = '';
		tag = '';
		index = 0;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-40 bg-black/40" />
		<Dialog.Content
			class="fixed top-[15%] left-1/2 z-50 max-h-[75dvh] w-[calc(100%_-_2rem)] max-w-lg -translate-x-1/2 overflow-y-auto rounded-2xl border border-[var(--separator)] bg-[var(--surface)] p-4 text-[var(--text)] shadow-xl"
			onOpenAutoFocus={(event) => {
				event.preventDefault();
				input?.focus();
			}}
			onCloseAutoFocus={(event) => {
				event.preventDefault();
				restoreFocus();
			}}
		>
			<div class="mb-2 flex items-center justify-between gap-4">
				<Dialog.Title class="font-semibold"
					>{snoozing
						? i18n.t.mailboxes.snooze
						: tagging
							? i18n.t.mailboxes.tag
							: i18n.t.mailboxes.actions}</Dialog.Title
				>
				<Dialog.Close class="rounded px-2 py-1 text-sm text-[var(--secondary)]"
					>{i18n.t.mailboxes.close}</Dialog.Close
				>
			</div>
			<Dialog.Description class="mb-4 truncate text-sm text-[var(--secondary)]"
				>{subject || '(No subject)'}</Dialog.Description
			>
			{#if snoozing}
				<SnoozeForm
					execute={(action) => {
						open = false;
						execute(action);
					}}
				/>
			{:else if tagging}
				<form
					onsubmit={(event) => {
						event.preventDefault();
						if (validTag(tag.trim())) {
							open = false;
							execute({ Tag: tag.trim() });
						}
					}}
				>
					<label class="text-sm" for="local-tag">Tag name</label>
					<input
						bind:this={input}
						bind:value={tag}
						id="local-tag"
						autocomplete="off"
						aria-describedby="tag-help"
						class="mt-2 w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-sm"
					/>
					<p id="tag-help" class="mt-2 text-xs text-[var(--secondary)]">
						Stored on this device. Up to 128 UTF-8 bytes, with no control characters.
					</p>
					<button
						type="submit"
						disabled={!validTag(tag.trim())}
						class="mt-4 rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-sm text-[var(--accent)] disabled:opacity-40"
						>Save tag</button
					>
				</form>
			{:else}
				<input
					bind:this={input}
					bind:value={query}
					oninput={() => (index = 0)}
					onkeydown={navigate}
					role="combobox"
					aria-label={i18n.t.mailboxes.actionSearch}
					aria-autocomplete="list"
					aria-expanded="true"
					aria-controls="triage-commands"
					aria-activedescendant={matches[index] ? `triage-command-${index}` : undefined}
					autocomplete="off"
					placeholder="Find an action…"
					class="mb-2 w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-sm"
				/>
				<ul id="triage-commands" role="listbox" aria-label="Message actions">
					{#each matches as command, i}
						{@const unavailable = command.action === 'OneClickUnsubscribe' && !canUnsubscribe}
						<li role="presentation">
							<button
								id={`triage-command-${i}`}
								type="button"
								role="option"
								aria-selected={index === i}
								aria-disabled={unavailable}
								onclick={() => choose(command)}
								onfocus={() => (index = i)}
								onkeydown={navigate}
								class="flex w-full items-center justify-between gap-4 rounded-lg px-3 py-3 text-left text-sm aria-selected:bg-[var(--selection-soft)] aria-disabled:opacity-40"
							>
								<span>{command.label}</span><kbd class="shrink-0 text-xs text-[var(--secondary)]"
									>{command.hint}</kbd
								>
							</button>
						</li>
					{/each}
				</ul>
				{#if !matches.length}<p role="status" class="px-3 py-3 text-sm">
						{i18n.t.mailboxes.noActions}
					</p>{/if}
				<p class="mt-4 text-xs text-[var(--secondary)]">
					Unsubscribe sends a request to the sender. Available only when this message supports
					one-click unsubscribe.
				</p>
			{/if}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
