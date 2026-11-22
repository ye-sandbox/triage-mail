<script lang="ts">
	import type { Definitions, Mapping } from '$lib/backup/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { settingsLabels } from './labels';

	interface Props {
		nativePreview: Definitions;
		accounts: AccountConfig[];
		destinations: { id: string; name: string }[];
		mapping: Mapping;
		consent: boolean;
		busy: boolean;
		complete: boolean;
		onMappingChange?: (next: Mapping) => void;
		onConsentChange?: (next: boolean) => void;
		onCommitNative?: () => void;
		onClose: () => void;
	}

	let {
		nativePreview,
		accounts,
		destinations,
		mapping,
		consent,
		busy,
		complete,
		onMappingChange,
		onConsentChange,
		onCommitNative,
		onClose
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
</script>

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
				<label class="flex flex-col gap-2 text-xs">
					<span class="font-mono text-secondary">{placeholder}</span>
					<select
						value={mapping.accounts[placeholder] ?? ''}
						disabled={busy}
						onchange={(e) => updateAccountMapping(placeholder, e.currentTarget.value)}
						class="h-9 rounded-md border border-separator bg-surface px-3 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
					>
						<option value="">{t.backup.chooseTarget}</option>
						{#each accounts as account (account.account_id)}
							<option value={account.account_id}>{account.email_display}</option>
						{/each}
					</select>
				</label>
			{/each}

			{#each nativePreview.destinations as placeholder (placeholder)}
				<label class="flex flex-col gap-2 text-xs">
					<span class="font-mono text-secondary">{placeholder}</span>
					<select
						value={mapping.destinations[placeholder] ?? ''}
						disabled={busy}
						onchange={(e) => updateDestinationMapping(placeholder, e.currentTarget.value)}
						class="h-9 rounded-md border border-separator bg-surface px-3 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
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
			class="mt-1 h-3.5 w-3.5 accent-accent"
		/>
		<span>{t.backup.approveImport}</span>
	</label>
</div>

<div class="mt-4 flex justify-end gap-2 border-t border-separator pt-3">
	<button
		type="button"
		disabled={busy}
		onclick={onClose}
		class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-secondary hover:text-foreground disabled:opacity-50"
	>
		{t.importDialog.cancel}
	</button>
	<button
		type="button"
		disabled={busy || !complete || !consent}
		onclick={() => onCommitNative?.()}
		class="min-h-[36px] rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
	>
		{busy ? t.backup.busy : t.backup.applyImportBtn}
	</button>
</div>
