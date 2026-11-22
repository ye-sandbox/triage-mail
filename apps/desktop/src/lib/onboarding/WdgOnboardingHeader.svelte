<script lang="ts">
	import Lock from '@lucide/svelte/icons/lock';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import Unlock from '@lucide/svelte/icons/unlock';
	import { i18n } from '../i18n/index.svelte';
	import { appStore } from '../stores/app.svelte';
	import type { OnboardingPhase } from './onboarding.fixtures';

	interface Props {
		isNative: boolean;
		indexedCount: number;
		nativeUnlocked: boolean;
		recovery: boolean;
		hasAccounts: boolean;
		demoPhase: OnboardingPhase;
		onSelectDemoPhase: (phase: OnboardingPhase) => void;
		onOpenInbox: () => void;
	}

	let {
		isNative,
		indexedCount,
		nativeUnlocked,
		recovery,
		hasAccounts,
		demoPhase,
		onSelectDemoPhase,
		onOpenInbox
	}: Props = $props();

	const t = $derived(i18n.t.onboarding);

	const DEMO_PHASES: { id: OnboardingPhase; label: string }[] = [
		{ id: 'vault-locked', label: '1. Vault Locked Gate' },
		{ id: 'discovery', label: '2. Discovery & Auth' },
		{ id: 'directory', label: '3. Connected Accounts' }
	];
</script>

<div class="border-b border-separator bg-toolbar px-4 py-3 md:px-6">
	<div class="mx-auto flex w-full max-w-4xl flex-wrap items-center justify-between gap-3">
		<div>
			<h1 class="font-display text-lg font-semibold text-foreground">{t.pageTitle}</h1>
			<div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-secondary">
				<span class="font-mono tabular-nums">{indexedCount} {t.accountsIndexed}</span>
				<span aria-hidden="true">·</span>
				{#if isNative}
					<span
						class="inline-flex items-center gap-1 font-medium {nativeUnlocked
							? 'text-status-healthy'
							: 'text-status-warning'}"
					>
						{#if nativeUnlocked}
							<ShieldCheck class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t.vaultUnlockedBadge}</span>
						{:else}
							<Lock class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{recovery ? t.recoveryModeBadge : t.vaultLockedBadge}</span>
						{/if}
					</span>
				{:else}
					<span
						class="inline-flex items-center gap-1 font-medium {appStore.vaultUnlocked &&
						demoPhase !== 'vault-locked'
							? 'text-status-healthy'
							: 'text-status-warning'}"
					>
						{#if appStore.vaultUnlocked && demoPhase !== 'vault-locked'}
							<ShieldCheck class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t.vaultUnlockedBadge}</span>
						{:else}
							<Lock class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t.vaultLockedBadge}</span>
						{/if}
					</span>
				{/if}
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<div
				role="radiogroup"
				aria-label={i18n.t.language.selectLabel}
				class="inline-flex items-center rounded-md border border-separator bg-field p-1"
			>
				<button
					type="button"
					role="radio"
					aria-checked={i18n.locale === 'en'}
					aria-label="English"
					onclick={() => i18n.setLocale('en')}
					class="min-h-[32px] rounded px-3 text-xs font-medium transition-colors {i18n.locale === 'en'
						? 'bg-surface text-foreground shadow-xs'
						: 'text-secondary hover:text-foreground'}"
				>
					EN
				</button>
				<button
					type="button"
					role="radio"
					aria-checked={i18n.locale === 'pt-BR'}
					aria-label="Português"
					onclick={() => i18n.setLocale('pt-BR')}
					class="min-h-[32px] rounded px-3 text-xs font-medium transition-colors {i18n.locale === 'pt-BR'
						? 'bg-surface text-foreground shadow-xs'
						: 'text-secondary hover:text-foreground'}"
				>
					PT
				</button>
			</div>

			{#if !isNative}
				<div
					role="tablist"
					aria-label="Onboarding lifecycle phases"
					class="flex items-center gap-1 rounded-md border border-separator bg-field p-1"
				>
					{#each DEMO_PHASES as item (item.id)}
						<button
							type="button"
							role="tab"
							aria-selected={demoPhase === item.id}
							onclick={() => onSelectDemoPhase(item.id)}
							class="min-h-[36px] rounded px-3 py-1 text-xs font-medium whitespace-nowrap shrink-0 transition-colors {demoPhase ===
							item.id
								? 'bg-surface text-foreground shadow-xs'
								: 'text-secondary hover:text-foreground'}"
						>
							{item.label}
						</button>
					{/each}
				</div>

				{#if demoPhase !== 'vault-locked'}
					<button
						type="button"
						onclick={() => onSelectDemoPhase('vault-locked')}
						class="flex min-h-[38px] items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap shrink-0 hover:bg-field"
					>
						<Lock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
						<span>Lock Vault</span>
					</button>
				{:else}
					<button
						type="button"
						onclick={() => onSelectDemoPhase('directory')}
						class="flex min-h-[38px] items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap shrink-0 hover:bg-field"
					>
						<Unlock class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
						<span>Preview Directory</span>
					</button>
				{/if}
			{:else if nativeUnlocked && hasAccounts}
				<button
					type="button"
					onclick={onOpenInbox}
					class="flex min-h-[38px] items-center gap-2 rounded-md bg-accent px-3 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap hover:opacity-95"
				>
					<span>{t.openInbox}</span>
				</button>
			{/if}
		</div>
	</div>
</div>
