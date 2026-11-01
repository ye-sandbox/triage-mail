<script lang="ts">
	import { i18n, type SupportedLocale } from '../i18n/index.svelte';

	const options: { code: SupportedLocale; label: string; full: string }[] = [
		{ code: 'en', label: 'EN', full: 'English' },
		{ code: 'pt-BR', label: 'PT', full: 'Português' }
	];
</script>

<div
	role="radiogroup"
	aria-label={i18n.t.language.selectLabel}
	class="inline-flex items-center rounded-full border border-[var(--separator)] bg-[var(--surface)] p-0.5 shadow-xs backdrop-blur-md transition-colors"
>
	{#each options as opt}
		<button
			type="button"
			role="radio"
			aria-checked={i18n.locale === opt.code}
			aria-label={opt.full}
			class="relative flex h-6 min-w-[2.25rem] items-center justify-center rounded-full px-2.5 text-xs font-medium tracking-wide transition-all duration-150 active:scale-95 {i18n.locale ===
			opt.code
				? 'bg-[var(--accent)] font-semibold text-white shadow-xs'
				: 'text-[var(--secondary)] hover:text-[var(--text)]'}"
			onclick={() => i18n.setLocale(opt.code)}
		>
			{opt.label}
		</button>
	{/each}
</div>
