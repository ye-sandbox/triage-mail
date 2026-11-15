<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { X } from 'lucide-svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { Definitions, Mapping } from '$lib/backup/transport';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { settingsLabels } from './labels';
	import type { PortableImportPreview } from './settings.fixtures';

	interface Props {
		open: boolean;
		// Demo fixture preview mode
		preview?: PortableImportPreview | null;
		onResolveConflict?: (id: string, resolution: 'keep_local' | 'overwrite') => void;
		onCommit?: () => void;
		// Native TransferTransport mode
		nativePreview?: Definitions | null;
		accounts?: AccountConfig[];
		destinations?: { id: string; name: string }[];
		mapping?: Mapping;
		consent?: boolean;
		busy?: boolean;
		complete?: boolean;
		onMappingChange?: (next: Mapping) => void;
		onConsentChange?: (next: boolean) => void;
		onCommitNative?: () => void;
		onCancel?: () => void;
	}

	let {
		open = $bindable(false),
		preview = null,
		onResolveConflict,
		onCommit,
		nativePreview = null,
		accounts = [],
		destinations = [],
		mapping = { accounts: {}, destinations: {} },
		consent = false,
		busy = false,
		complete = false,
		onMappingChange,
		onConsentChange,
		onCommitNative,
		onCancel
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale]);

	function updateAccountMapping(placeholder: string, targetId: string) {
		onMappingChange?.({
			accounts: { ...mapping.accounts, [placeholder]: targetId },
			destinations: { ...mapping.destinations }
		});
		onConsentChange?.(false);
	}

	function updateDestinationMapping(placeholder: string, targetId: string) {
		onMappingChange?.({
			accounts: { ...mapping.accounts },
			destinations: { ...mapping.destinations, [placeholder]: targetId }
		});
		onConsentChange?.(false);
	}

	function closeDialog() {
		if (busy) return;
		open = false;
		onCancel?.();
	}
</script>

<Dialog.Root
	{open}
	onOpenChange={(next) => {
		if (!busy) {
			open = next;
			if (!next) onCancel?.();
		}
	}}
>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			class="fixed left-1/2 top-1/2 z-50 max-h-[88vh] w-[calc(100%-2rem)] max-w-2xl -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border border-separator bg-surface p-5 text-foreground shadow-xl outline-none"
		>
			<div class="flex items-start justify-between gap-3 border-b border-separator pb-3">
				<div>
					<Dialog.Title class="font-display text-sm font-semibold text-foreground">
						{nativePreview ? t.backup.reviewHeading : t.importDialog.title}
					</Dialog.Title>
					{#if preview && !nativePreview}
						<p class="mt-0.5 font-mono text-[11px] text-secondary">
							{preview.source_filename} · {preview.schema_version}
						</p>
					{:else if nativePreview}
						<p class="mt-0.5 font-mono text-[11px] text-secondary">
							schema v{nativePreview.version} · {t.importDialog.rulesCount}: {nativePreview.rules.length} · {t.importDialog.viewsCount}: {nativePreview.views.length}
						</p>
					{/if}
				</div>
				<Dialog.Close
					disabled={busy}
					aria-label={t.importDialog.closeAria}
					class="rounded p-1 text-secondary hover:bg-field hover:text-foreground disabled:opacity-40"
				>
					<X class="h-4 w-4" />
				</Dialog.Close>
			</div>

			{#if nativePreview}
				<div class="my-4 space-y-4">
					<pre
						class="max-h-56 overflow-auto whitespace-pre-wrap break-all rounded-md border border-separator bg-field p-3 font-mono text-[11px] text-foreground">{JSON.stringify(
							nativePreview,
							null,
							2
						)}</pre>

					{#if nativePreview.accounts.length > 0 || nativePreview.destinations.length > 0}
						<div class="space-y-3 rounded-md border border-separator bg-field p-3">
							<h3 class="text-xs font-semibold text-foreground">{t.backup.mappingHeading}</h3>

							{#each nativePreview.accounts as placeholder (placeholder)}
								<label class="flex flex-col gap-1.5 text-xs">
									<span class="font-mono text-secondary">{placeholder}</span>
									<select
										value={mapping.accounts[placeholder] ?? ''}
										disabled={busy}
										onchange={(e) =>
											updateAccountMapping(placeholder, e.currentTarget.value)}
										class="h-9 rounded-md border border-separator bg-surface px-2.5 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
									>
										<option value="">{t.backup.chooseTarget}</option>
										{#each accounts as account (account.account_id)}
											<option value={account.account_id}>{account.email_display}</option>
										{/each}
									</select>
								</label>
							{/each}

							{#each nativePreview.destinations as placeholder (placeholder)}
								<label class="flex flex-col gap-1.5 text-xs">
									<span class="font-mono text-secondary">{placeholder}</span>
									<select
										value={mapping.destinations[placeholder] ?? ''}
										disabled={busy}
										onchange={(e) =>
											updateDestinationMapping(placeholder, e.currentTarget.value)}
										class="h-9 rounded-md border border-separator bg-surface px-2.5 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
									>
										<option value="">{t.backup.chooseTarget}</option>
										{#each destinations as destination (destination.id)}
											<option value={destination.id}>{destination.name}</option>
										{/each}
									</select>
								</label>
							{/each}
						</div>
					{/if}

					<label class="flex cursor-pointer items-start gap-2 text-xs text-secondary">
						<input
							type="checkbox"
							checked={consent}
							disabled={busy}
							onchange={(e) => onConsentChange?.(e.currentTarget.checked)}
							class="mt-0.5 h-3.5 w-3.5 accent-accent"
						/>
						<span>{t.backup.approveImport}</span>
					</label>
				</div>

				<div class="mt-4 flex justify-end gap-2 border-t border-separator pt-3">
					<button
						type="button"
						disabled={busy}
						onclick={closeDialog}
						class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground disabled:opacity-50"
					>
						{t.importDialog.cancel}
					</button>
					<button
						type="button"
						disabled={busy || !complete || !consent}
						onclick={() => onCommitNative?.()}
						class="min-h-[36px] rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
					>
						{busy ? t.backup.busy : t.backup.applyImportBtn}
					</button>
				</div>
			{:else if preview}
				<div class="my-4 grid grid-cols-3 gap-3 rounded-md border border-separator bg-field p-3">
					<div>
						<div class="text-[11px] text-secondary">{t.importDialog.added}</div>
						<div class="font-mono text-base font-semibold tabular-nums text-status-healthy">
							{preview.counts.added}
						</div>
					</div>
					<div>
						<div class="text-[11px] text-secondary">{t.importDialog.updated}</div>
						<div class="font-mono text-base font-semibold tabular-nums text-status-info">
							{preview.counts.updated}
						</div>
					</div>
					<div>
						<div class="text-[11px] text-secondary">{t.importDialog.conflicting}</div>
						<div class="font-mono text-base font-semibold tabular-nums text-status-warning">
							{preview.counts.conflicting}
						</div>
					</div>
				</div>

				<div class="divide-y divide-separator rounded-md border border-separator">
					{#each preview.items as item (item.id)}
						<div class="flex flex-wrap items-center justify-between gap-3 p-3 text-xs">
							<div class="min-w-0 flex-1 space-y-0.5">
								<div class="flex items-center gap-2 font-medium text-foreground">
									<span class="font-mono text-[11px] text-secondary">{item.kind}</span>
									<span aria-hidden="true">·</span>
									<span class="truncate">{item.name}</span>
								</div>
								<p class="text-[11px] text-secondary">{item.summary}</p>
							</div>
							{#if item.action === 'conflicting'}
								<div class="flex shrink-0 items-center gap-1 rounded border border-separator bg-field p-0.5">
									<button
										type="button"
										onclick={() => onResolveConflict?.(item.id, 'keep_local')}
										class="rounded px-2 py-1 text-[11px] font-medium {item.resolution ===
										'keep_local'
											? 'bg-surface text-foreground'
											: 'text-secondary'}"
									>
										{t.importDialog.keepLocal}
									</button>
									<button
										type="button"
										onclick={() => onResolveConflict?.(item.id, 'overwrite')}
										class="rounded px-2 py-1 text-[11px] font-medium {item.resolution ===
										'overwrite'
											? 'bg-surface text-status-warning'
											: 'text-secondary'}"
									>
										{t.importDialog.overwrite}
									</button>
								</div>
							{:else}
								<span
									class="shrink-0 font-mono text-[11px] font-medium {item.action === 'added'
										? 'text-status-healthy'
										: 'text-status-info'}"
								>
									{item.action}
								</span>
							{/if}
						</div>
					{/each}
				</div>

				<div class="mt-4 flex justify-end gap-2 border-t border-separator pt-3">
					<button
						type="button"
						onclick={closeDialog}
						class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground"
					>
						{t.importDialog.cancel}
					</button>
					<button
						type="button"
						onclick={() => onCommit?.()}
						class="min-h-[36px] rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95"
					>
						{t.importDialog.commit}
					</button>
				</div>
			{/if}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
