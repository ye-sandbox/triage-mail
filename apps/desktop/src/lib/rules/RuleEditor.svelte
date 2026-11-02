<script lang="ts">
	import type { AccountConfig } from '../onboarding/types';
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import { labels } from './labels';
	import type { Rule, RuleAction } from './types';
	import ConditionsEditor from './ConditionsEditor.svelte';
	let {
		draft,
		busy,
		change,
		save,
		cancel,
		accounts = []
	}: {
		draft: Rule;
		busy: boolean;
		change: (rule: Rule) => void;
		save: () => void;
		cancel: () => void;
		accounts?: AccountConfig[];
	} = $props();
	const t = $derived(labels[i18n.locale]);
	let tags = $state(
		untrack(() => draft.actions.flatMap((a) => (typeof a === 'object' ? [a.Tag] : [])).join('\n'))
	);
	function toggle(action: RuleAction, enabled: boolean) {
		change({
			...draft,
			actions: enabled ? [...draft.actions, action] : draft.actions.filter((a) => a !== action)
		});
	}
	function setTags(value: string) {
		tags = value;
		change({
			...draft,
			actions: [
				...draft.actions.filter((a) => typeof a === 'string'),
				...value
					.split('\n')
					.filter((v) => v.trim())
					.map((Tag) => ({ Tag }))
			]
		});
	}
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		save();
	}}
	class="space-y-4"
>
	<fieldset disabled={busy} class="space-y-4">
		<label class="block"
			>{t.name}<input
				required
				maxlength="120"
				value={draft.name}
				oninput={(e) => change({ ...draft, name: e.currentTarget.value })}
				class="block w-full rounded border border-[var(--separator)] bg-[var(--surface)] p-2"
			/></label
		>
		<label class="flex gap-2"
			><input
				type="checkbox"
				checked={draft.enabled}
				onchange={(e) => change({ ...draft, enabled: e.currentTarget.checked })}
			/>{t.enabled}</label
		>
		<p class="text-sm text-[var(--secondary)]">{t.cents}</p>
		<ConditionsEditor
			{accounts}
			title={t.all}
			values={draft.all}
			disabled={busy}
			change={(all) => change({ ...draft, all })}
		/>
		<ConditionsEditor
			{accounts}
			title={t.any}
			values={draft.any}
			disabled={busy}
			change={(any) => change({ ...draft, any })}
		/>
		<ConditionsEditor
			{accounts}
			title={t.except}
			values={draft.except}
			disabled={busy}
			change={(except) => change({ ...draft, except })}
		/>
		<label class="block"
			>{t.tags}<textarea
				value={tags}
				oninput={(e) => setTags(e.currentTarget.value)}
				class="block w-full rounded border border-[var(--separator)] bg-[var(--surface)] p-2"
			></textarea></label
		>
		<label class="flex gap-2"
			><input
				type="checkbox"
				checked={draft.actions.includes('Star')}
				onchange={(e) => toggle('Star', e.currentTarget.checked)}
			/>{t.star}</label
		>
		<label class="flex gap-2"
			><input
				type="checkbox"
				checked={draft.actions.includes('Archive')}
				onchange={(e) => toggle('Archive', e.currentTarget.checked)}
			/>{t.archive}</label
		>
		<label class="flex gap-2"
			><input
				type="checkbox"
				checked={draft.stop_processing}
				onchange={(e) => change({ ...draft, stop_processing: e.currentTarget.checked })}
			/>{t.stop}</label
		>
		<div class="flex gap-4">
			<button type="submit">{t.save}</button><button type="button" onclick={cancel}
				>{t.cancel}</button
			>
		</div>
	</fieldset>
</form>
