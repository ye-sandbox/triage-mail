<script lang="ts">
	import Lock from '@lucide/svelte/icons/lock';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import { EXPORT_FIELDS, type Destination, type ExportField } from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import { automationLabels } from './labels';

	interface Props {
		draft: Destination;
		initialKind: Destination['kind'];
		urlInput?: HTMLInputElement;
		bearerInput?: HTMLInputElement;
		onUrlInput: (input: HTMLInputElement) => void;
		onBearerInput: () => void;
		onToggleField: (field: ExportField) => void;
	}

	let {
		draft,
		initialKind,
		urlInput = $bindable(),
		bearerInput = $bindable(),
		onUrlInput,
		onBearerInput,
		onToggleField
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);
</script>

{#if draft.kind === 'Telegram'}
	<p class="rounded-md border border-separator bg-field/60 p-3 text-[11px] text-secondary">
		{alertT.telegramHelp}
	</p>
{:else if draft.kind === 'Discord'}
	<p class="rounded-md border border-separator bg-field/60 p-3 text-[11px] text-secondary">
		{alertT.discordHelp}
	</p>
{/if}

<div class="space-y-2 rounded-md border border-separator bg-field/40 p-3">
	<div class="flex flex-wrap items-center justify-between gap-2 font-mono text-[11px] text-secondary">
		<span>{t.originLabel}: {draft.public_origin ?? '—'}</span>
	</div>

	<label class="flex flex-col gap-1">
		<span class="flex items-center gap-2 font-medium text-secondary">
			<Lock class="h-3 w-3" aria-hidden="true" />
			<span>{draft.kind === 'Telegram' ? alertT.token : alertT.url}</span>
		</span>
		<input
			bind:this={urlInput}
			type="password"
			autocomplete="off"
			required={draft.revision === 0 || draft.kind !== initialKind}
			oninput={(e) => onUrlInput(e.currentTarget)}
			placeholder={draft.has_credentials ? t.sealedPlaceholder : t.originPlaceholder}
			class="h-9 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
		/>
	</label>

	{#if draft.kind !== 'Discord'}
		<label class="flex flex-col gap-1">
			<span class="font-medium text-secondary">
				{draft.kind === 'Telegram' ? alertT.chat : alertT.bearer}
			</span>
			<input
				bind:this={bearerInput}
				type="password"
				autocomplete="off"
				required={draft.kind === 'Telegram' && (draft.revision === 0 || draft.kind !== initialKind)}
				oninput={(e) => {
					e.currentTarget.setCustomValidity('');
					onBearerInput();
				}}
				class="h-9 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
			/>
		</label>
	{/if}

	<p class="text-[11px] text-secondary">{alertT.preserve}</p>
	<p class="text-[11px] text-secondary">{alertT.revoke}</p>
	<p class="font-mono text-[11px] text-secondary">{t.originHint}</p>
</div>

<fieldset class="space-y-2 rounded-md border border-separator bg-field/60 p-3">
	<legend class="px-1 font-medium text-foreground">{alertT.fields}</legend>
	<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
		{#each EXPORT_FIELDS as field (field)}
			<label class="flex cursor-pointer items-center gap-2 rounded border border-separator bg-surface px-3 py-2">
				<input
					type="checkbox"
					checked={draft.fields.includes(field)}
					onchange={() => onToggleField(field)}
					class="h-3.5 w-3.5 accent-accent"
				/>
				<span class="text-[11px] text-foreground">{alertT.fieldNames[field]}</span>
			</label>
		{/each}
	</div>
	<p class="text-[11px] text-secondary">{alertT.metadata}</p>
	<p class="font-mono text-[11px] text-secondary">
		{alertT.preview}: {draft.fields.map((f) => alertT.fieldNames[f]).join(', ') || '—'}
	</p>
</fieldset>
