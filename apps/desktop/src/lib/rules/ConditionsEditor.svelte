<script lang="ts">
	import type { AccountConfig } from '../onboarding/types';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import { FIELDS, condition, type Condition, type Field } from './types';
	let {
		title,
		values,
		change,
		disabled = false,
		accounts = []
	}: {
		title: string;
		values: Condition[];
		change: (values: Condition[]) => void;
		disabled?: boolean;
		accounts?: AccountConfig[];
	} = $props();
	const t = $derived(labels[i18n.locale]);
	function update(index: number, field: Field, value: string) {
		change(values.map((c, i) => (i === index ? condition(field, value) : c)));
	}
</script>

<fieldset {disabled} class="space-y-2 rounded border border-[var(--separator)] p-4">
	<legend>{title}</legend>
	{#each values as c, index}
		{@const field = Object.keys(c)[0] as Field}
		<div class="flex flex-wrap gap-2">
			<select
				aria-label={`${title} ${t.field} ${index + 1}`}
				value={field}
				onchange={(e) => update(index, e.currentTarget.value as Field, '')}
				class="rounded bg-[var(--surface)] p-2"
			>
				{#each FIELDS as key}<option value={key}>{t.fields[key]}</option>{/each}
			</select>
			{#if field === 'Account'}
				<select
					aria-label={`${title} ${t.value} ${index + 1}`}
					value={String(Object.values(c)[0])}
					onchange={(e) => update(index, field, e.currentTarget.value)}
					class="rounded bg-[var(--surface)] p-2"
				>
					<option value="">—</option>
					{#each accounts as account}<option value={account.account_id}
							>{account.email_display}</option
						>{/each}
				</select>
			{:else}
				<input
					aria-label={`${title} ${t.value} ${index + 1}`}
					value={String(Object.values(c)[0])}
					type={field.startsWith('Amount') ? 'number' : field.startsWith('Due') ? 'date' : 'text'}
					min={field.startsWith('Amount') ? '0' : undefined}
					max={field.startsWith('Amount') ? '9007199254740991' : undefined}
					step={field.startsWith('Amount') ? '1' : undefined}
					maxlength="512"
					oninput={(e) => update(index, field, e.currentTarget.value)}
					class="min-w-0 flex-1 rounded border border-[var(--separator)] bg-[var(--surface)] p-2"
				/>
			{/if}
			<button type="button" onclick={() => change(values.filter((_, i) => i !== index))}
				>{t.remove}</button
			>
		</div>
	{/each}
	<button
		type="button"
		disabled={disabled || values.length >= 64}
		onclick={() => change([...values, { Subject: '' }])}>+ {title}</button
	>
</fieldset>
