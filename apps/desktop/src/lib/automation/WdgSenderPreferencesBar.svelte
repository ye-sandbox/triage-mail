<script lang="ts">
	import { Switch } from 'bits-ui';
	import { i18n } from '$lib/i18n/index.svelte';
	import { labels as reviewLabelsMap } from '$lib/sender-review/labels';
	import { automationLabels } from './labels';

	interface Props {
		enabled: boolean;
		hide: boolean;
		dirty: boolean;
		busy: boolean;
		loading: boolean;
		onToggleEnabled: (checked: boolean) => void;
		onToggleHide: (checked: boolean) => void;
		onSave: () => void | Promise<void>;
	}

	let {
		enabled,
		hide,
		dirty,
		busy,
		loading,
		onToggleEnabled,
		onToggleHide,
		onSave
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].reviewPane);
	const reviewT = $derived(reviewLabelsMap[i18n.locale]);
</script>

<fieldset disabled={busy || loading} class="flex min-w-0 max-w-full flex-col gap-3 border-0 p-0">
	<div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-separator bg-field p-3">
		<div class="flex flex-wrap items-center gap-4">
			<label class="flex cursor-pointer items-center gap-2">
				<Switch.Root
					checked={enabled}
					onCheckedChange={onToggleEnabled}
					aria-label={t.toggleEnableAria}
					class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
				>
					<Switch.Thumb
						class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
					/>
				</Switch.Root>
				<span class="text-xs font-medium text-foreground">{reviewT.enable}</span>
			</label>

			<label class="flex cursor-pointer items-center gap-2">
				<Switch.Root
					checked={hide}
					onCheckedChange={onToggleHide}
					aria-label={t.toggleHideAria}
					class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-1 transition-colors data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface"
				>
					<Switch.Thumb
						class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0"
					/>
				</Switch.Root>
				<span class="text-xs font-medium text-foreground">{reviewT.hide}</span>
			</label>
		</div>

		<button
			type="button"
			disabled={!dirty || busy || loading}
			onclick={() => void onSave()}
			class="min-h-[34px] rounded-md bg-accent px-4 py-1 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-45"
		>
			{busy ? reviewT.saving : reviewT.save}
		</button>
	</div>

	<p class="text-[11px] text-secondary">{reviewT.hideHelp}</p>
</fieldset>
