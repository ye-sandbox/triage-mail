<script lang="ts">
	import { untrack } from 'svelte';
	import { Switch } from 'bits-ui';
	import { CheckCircle2, Cpu, KeyRound, Lock } from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import { clearCredentialInputs } from '$lib/onboarding/forms';
	import type { SettingsFormGuard } from './guards';
	import { settingsLabels } from './labels';
	import type { AiSettings, Settings, SettingsTransport } from './types';

	interface Props {
		settings: Settings;
		transport: SettingsTransport;
		guard?: SettingsFormGuard;
		busy?: boolean;
		onReload?: () => void | Promise<void>;
	}

	let { settings, transport, guard, busy = false, onReload }: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].ai);

	let formEl = $state<HTMLFormElement>();
	let enabled = $state(untrack(() => settings.ai.enabled));
	let provider = $state<'Ollama' | 'Byok'>(untrack(() => settings.ai.provider));
	let endpoint = $state(untrack(() => settings.ai.endpoint));
	let model = $state(untrack(() => settings.ai.model));
	let hasTransientKey = $state(false);
	let saving = $state(false);
	let statusKind = $state<'ok' | 'error' | ''>('');
	let active = true;

	const isDirty = $derived(
		enabled !== settings.ai.enabled ||
			provider !== settings.ai.provider ||
			endpoint.trim() !== settings.ai.endpoint ||
			model.trim() !== settings.ai.model ||
			hasTransientKey
	);

	$effect(() => {
		const baseline = settings.ai;
		if (!saving && !isDirty) {
			enabled = baseline.enabled;
			provider = baseline.provider;
			endpoint = baseline.endpoint;
			model = baseline.model;
		}
	});

	$effect(() => {
		guard?.markDirty(isDirty);
	});

	$effect(() => {
		active = true;
		const currentForm = formEl;
		return () => {
			active = false;
			clearCredentialInputs(currentForm);
			guard?.reset();
		};
	});

	function selectProvider(next: 'Ollama' | 'Byok') {
		if (next === provider) return;
		provider = next;
		endpoint = next === 'Ollama' ? 'http://127.0.0.1:11434' : 'https://api.openai.com/v1';
		model = next === 'Ollama' ? 'qwen2.5:7b-instruct' : 'gpt-4o-mini';
		statusKind = '';
	}

	async function handleSave(event: SubmitEvent) {
		event.preventDefault();
		if (!formEl || saving || busy || guard?.isBlocked()) return;

		const keyInput = formEl.elements.namedItem('apiKey') as HTMLInputElement | null;
		// INVARIANT: API keys live only in the unbound DOM input and invoke arguments; clear before await.
		const key = keyInput?.value ?? '';
		const payload: AiSettings = {
			enabled,
			provider,
			endpoint: endpoint.trim(),
			model: model.trim()
		};

		clearCredentialInputs(formEl);
		hasTransientKey = false;
		saving = true;
		guard?.setBusy(true);
		statusKind = '';

		try {
			await transport.save(payload, key);
			if (active) {
				statusKind = 'ok';
				guard?.markDirty(false);
				toast.success(t.saveSuccess);
				await onReload?.();
			}
		} catch {
			if (active) {
				statusKind = 'error';
				toast.error(t.saveError);
			}
		} finally {
			clearCredentialInputs(formEl);
			hasTransientKey = false;
			if (active) {
				saving = false;
				guard?.setBusy(false);
			}
		}
	}
</script>

<section class="flex flex-col gap-5" aria-labelledby="ai-models-heading">
	<div class="flex flex-wrap items-center justify-between gap-4 border-b border-separator pb-3">
		<div>
			<h2 id="ai-models-heading" class="font-display text-base font-semibold text-foreground">
				{t.heading}
			</h2>
			<p class="mt-0.5 text-xs text-secondary">{t.subtitle}</p>
		</div>

		<div class="flex shrink-0 items-center gap-2.5">
			<span class="font-mono text-xs {enabled ? 'text-status-healthy' : 'text-secondary'}">
				{enabled ? t.enabledBadge : t.disabledBadge}
			</span>
			<Switch.Root
				checked={enabled}
				disabled={saving || busy}
				onCheckedChange={(checked) => (enabled = checked)}
				aria-label={t.enableAria}
				class="inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-separator p-0.5 transition-colors focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-accent data-[state=unchecked]:bg-field"
			>
				<Switch.Thumb class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-surface shadow-sm transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0" />
			</Switch.Root>
		</div>
	</div>

	<form
		bind:this={formEl}
		onsubmit={handleSave}
		aria-label={t.formAria}
		autocomplete="off"
		class="flex flex-col gap-5 rounded-lg border border-separator bg-surface p-4"
	>
		<p class="text-xs leading-relaxed text-secondary">{t.disclosure}</p>

		<div class="flex flex-col gap-2">
			<span class="text-xs font-medium text-foreground">{t.providerLabel}</span>
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
				<button
					type="button"
					disabled={saving || busy}
					aria-pressed={provider === 'Ollama'}
					onclick={() => selectProvider('Ollama')}
					class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:cursor-not-allowed {provider ===
					'Ollama'
						? 'border-accent bg-selection-soft'
						: 'border-separator bg-field hover:bg-selection-soft/40'}"
				>
					<div class="flex w-full items-center justify-between gap-2">
						<span class="flex items-center gap-1.5 text-xs font-semibold text-foreground">
							<Cpu class="h-3.5 w-3.5 text-secondary" />
							<span>{t.ollamaTitle}</span>
						</span>
						<span class="font-mono text-[11px] text-status-healthy">{t.ollamaBadge}</span>
					</div>
					<span class="font-mono text-[11px] text-secondary">{t.ollamaDesc}</span>
				</button>

				<button
					type="button"
					disabled={saving || busy}
					aria-pressed={provider === 'Byok'}
					onclick={() => selectProvider('Byok')}
					class="flex flex-col items-start gap-1 rounded-md border p-3 text-left transition-colors disabled:cursor-not-allowed {provider ===
					'Byok'
						? 'border-accent bg-selection-soft'
						: 'border-separator bg-field hover:bg-selection-soft/40'}"
				>
					<div class="flex w-full items-center justify-between gap-2">
						<span class="flex items-center gap-1.5 text-xs font-semibold text-foreground">
							<Lock class="h-3.5 w-3.5 text-secondary" />
							<span>{t.byokTitle}</span>
						</span>
						<span class="font-mono text-[11px] text-status-warning">{t.byokBadge}</span>
					</div>
					<span class="font-mono text-[11px] text-secondary">{t.byokDesc}</span>
				</button>
			</div>
		</div>

		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<label class="flex flex-col gap-1.5">
				<span class="text-xs font-medium text-foreground">{t.endpointLabel}</span>
				<input
					name="endpoint"
					type="url"
					required
					disabled={saving || busy}
					bind:value={endpoint}
					placeholder="http://127.0.0.1:11434"
					class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
				/>
			</label>

			<label class="flex flex-col gap-1.5">
				<span class="text-xs font-medium text-foreground">{t.modelLabel}</span>
				<input
					name="model"
					type="text"
					required
					maxlength="200"
					disabled={saving || busy}
					bind:value={model}
					placeholder="qwen2.5:7b-instruct"
					class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
				/>
			</label>
		</div>

		<p class="text-[11px] text-secondary">{t.endpointHelp}</p>

		<div class="flex flex-col gap-3 border-t border-separator pt-4">
			<div class="flex flex-wrap items-center justify-between gap-2">
				<label for="ai-vault-api-key" class="flex items-center gap-1.5 text-xs font-medium text-foreground">
					<KeyRound class="h-3.5 w-3.5 text-secondary" />
					<span>{t.apiKeyLabel}</span>
				</label>

				{#if settings.has_api_key}
					<span class="flex items-center gap-1.5 font-mono text-xs font-medium text-status-healthy">
						<CheckCircle2 class="h-3.5 w-3.5 shrink-0" />
						<span>{t.keyStored}</span>
					</span>
				{:else}
					<span class="font-mono text-xs text-secondary">{t.noKeyStored}</span>
				{/if}
			</div>

			<input
				id="ai-vault-api-key"
				name="apiKey"
				type="password"
				data-credential
				disabled={saving || busy}
				oninput={(e) => (hasTransientKey = e.currentTarget.value.length > 0)}
				autocomplete="off"
				spellcheck="false"
				placeholder={settings.has_api_key ? t.keyPlaceholderStored : t.keyPlaceholderEmpty}
				class="h-9 w-full rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground placeholder:font-sans placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-50"
			/>

			<p class="text-[11px] text-secondary">{t.keyHelp}</p>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-3 border-t border-separator pt-3">
			{#if statusKind === 'ok'}
				<p role="status" class="text-xs text-status-healthy">{t.saveSuccess}</p>
			{:else if statusKind === 'error'}
				<p role="alert" class="text-xs text-status-danger">{t.saveError}</p>
			{:else}
				<span></span>
			{/if}

			<button
				type="submit"
				disabled={saving || busy}
				class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
			>
				{saving ? t.savingBtn : t.saveBtn}
			</button>
		</div>
	</form>
</section>
