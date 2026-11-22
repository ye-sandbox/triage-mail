<script lang="ts">
	import { untrack } from 'svelte';
	import FileUp from '@lucide/svelte/icons/file-up';
	import GlobeLock from '@lucide/svelte/icons/globe-lock';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountController } from './controller.svelte';
	import { nativeDiscoveryTransport, type DiscoveryTransport } from './discovery';
	import { importGoogleClientJsonFile, performDiscoveryLookup } from './discovery-actions';
	import { DiscoveryController } from './discovery-controller.svelte';
	import { runDiscoveryTestLogin, submitDiscoveryAccount } from './discovery-submit';
	import { clearCredentialInputs } from './forms';
	import { probeTlsEndpoint, type DemoConnectedAccountDto, type IspdbPreset, type TlsProbeResult } from './onboarding.fixtures';
	import { oauthDefaultsForHost, providerPresets, type ProviderPreset } from './presets';
	import { providerKindFor, type RecommendedAction } from './recommendation';
	import { looksLikeRegularGmailPassword } from './secret';
	import type { AuthMethod, LocalArchiveFormat } from './types';
	import WdgDiscoveryAdvancedFields from './WdgDiscoveryAdvancedFields.svelte';
	import WdgDiscoveryArchiveSection from './WdgDiscoveryArchiveSection.svelte';
	import WdgDiscoveryAuthTabs from './WdgDiscoveryAuthTabs.svelte';
	import WdgDiscoveryCredentialsSection from './WdgDiscoveryCredentialsSection.svelte';
	import WdgDiscoveryFeedback from './WdgDiscoveryFeedback.svelte';
	import WdgDiscoveryLookupBar from './WdgDiscoveryLookupBar.svelte';
	import WdgDiscoverySubmitBar from './WdgDiscoverySubmitBar.svelte';
	import WdgMethodRecommendationCard from './WdgMethodRecommendationCard.svelte';

	interface Props {
		controller: AccountController;
		discoveryTransport?: DiscoveryTransport;
		demoMode?: boolean;
		onAccountAdded?: (account: DemoConnectedAccountDto) => void;
		onSuccess?: () => void;
	}

	let { controller, discoveryTransport = nativeDiscoveryTransport, demoMode = false, onAccountAdded, onSuccess }: Props = $props();

	let form = $state<HTMLFormElement>();
	let emailInput = $state<HTMLInputElement>();
	let googleJsonInput = $state<HTMLInputElement>();
	let archivePath = $state('');
	let archiveFormat = $state<LocalArchiveFormat>('Mbox');
	let advancedExpanded = $state(false);
	let showAllMethods = $state(false);
	let showAllPresets = $state(false);
	let guideOpen = $state(false);
	let guideTab = $state<AuthMethod | 'EmailRouting'>('AppPassword');
	let gmailSecretSuspect = $state(false);
	const discovery = new DiscoveryController(untrack(() => discoveryTransport), () => clearCredentialInputs(form));

	let demoIspdb = $state<IspdbPreset | null>(null);
	let demoTls = $state<TlsProbeResult | null>(null);
	let demoPkceState = $state<'idle' | 'binding' | 'waiting' | 'authorized'>('idle');
	let demoPkceReady = $state(false);
	let demoPkceSeconds = $state(0);
	let readyTimer: ReturnType<typeof setTimeout> | undefined;
	let tickTimer: ReturnType<typeof setInterval> | undefined;

	const t = $derived(i18n.t.accountForm);
	const obT = $derived(i18n.t.onboarding);
	const method = $derived(discovery.settings.method);
	const oauth = $derived(method === 'OAuth2PublicPkce' || method === 'OAuth2ByoCredentials');
	const isBridge = $derived(method === 'ProtonBridgeLocal');
	const isArchive = $derived(method === 'LocalArchive');
	const providerKind = $derived(
		providerKindFor({ preset: discovery.selectedPreset, host: discovery.settings.host, method, status: discovery.status, providerName: discovery.provider?.provider_name })
	);
	const showAdvanced = $derived(
		advancedExpanded || (!isArchive && !isBridge && (discovery.selectedPreset === 'Other IMAP' || discovery.status === 'unresolved' || !discovery.settings.host.trim() || (oauth && (!discovery.settings.authorize.trim() || !discovery.settings.token.trim() || !discovery.settings.scopes.trim()))))
	);
	const INPUT_CLS = 'h-10 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-60';

	function stopDemoTimers() {
		clearTimeout(readyTimer);
		clearInterval(tickTimer);
		readyTimer = undefined;
		tickTimer = undefined;
	}

	$effect(() => {
		const currentMethod = method;
		const inputs = form?.querySelectorAll<HTMLInputElement>('[data-credential]');
		// INVARIANT: Capture actual DOM inputs including ones detached on auth method change.
		return () => {
			if (currentMethod) inputs?.forEach((input) => (input.value = ''));
		};
	});

	$effect(() => () => {
		stopDemoTimers();
		discovery.dispose();
		archivePath = '';
		clearCredentialInputs(form);
		if (controller.authorizing) void controller.cancel();
	});

	function applyPreset(preset: ProviderPreset) {
		clearCredentialInputs(form);
		controller.clearDiagnostics();
		archivePath = '';
		discovery.preset(preset);
		advancedExpanded = preset.name === 'Other IMAP';
		demoTls = null;
	}

	function switchMethod(next: AuthMethod) {
		if (next === method) return;
		clearCredentialInputs(form);
		controller.clearDiagnostics();
		if (next !== 'LocalArchive') archivePath = '';
		discovery.edit();
		discovery.settings.method = next;
		if (next === 'ProtonBridgeLocal') { discovery.settings.host = '127.0.0.1'; discovery.settings.port = 1143; }
		else if (discovery.settings.host === '127.0.0.1' && discovery.settings.port === 1143) {
			discovery.settings.host = discovery.provider?.imap_host ?? demoIspdb?.imap_host ?? '';
			discovery.settings.port = discovery.provider?.imap_port ?? demoIspdb?.imap_port ?? 993;
		}
		const oauthDefaults = (next === 'OAuth2PublicPkce' || next === 'OAuth2ByoCredentials') ? oauthDefaultsForHost(discovery.settings.host) : null;
		if (oauthDefaults) {
			discovery.settings.authorize = discovery.settings.authorize.trim() || oauthDefaults.authEndpoint;
			discovery.settings.token = discovery.settings.token.trim() || oauthDefaults.tokenEndpoint;
			discovery.settings.scopes = discovery.settings.scopes.trim() || oauthDefaults.scopes;
		}
		demoTls = null;
	}

	function applyRecommendation(action: RecommendedAction) {
		if (action.type === 'preset') { const found = providerPresets.find((p) => p.name === action.name); if (found) applyPreset(found); }
		else if (action.type === 'method') switchMethod(action.method);
		else { switchMethod('AppPassword'); advancedExpanded = true; }
	}

	function handleFormSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (controller.busy || controller.testing || discovery.busy || !form) return;
		void submitDiscoveryAccount({
			form, method, oauth, isBridge, isArchive, demoMode,
			demoPkceAuthorized: demoPkceState === 'authorized', archivePath, archiveFormat,
			controller, discovery, archiveErrorMsg: t.localArchiveImportError,
			archiveSuccessPrefix: t.localArchiveSuccess, onAccountAdded,
			onArchiveImported: () => { archivePath = ''; if (emailInput) emailInput.value = ''; },
			onDemoReset: () => { if (emailInput) emailInput.value = ''; demoIspdb = null; demoTls = null; demoPkceState = 'idle'; },
			onResetHint: () => (gmailSecretSuspect = false),
			onNativeSuccess: () => { if (emailInput) emailInput.value = ''; onSuccess?.(); }
		});
	}
</script>

<section aria-labelledby="add-account-title" class="mx-auto w-full max-w-3xl space-y-5 rounded-lg border border-separator bg-surface p-5 md:p-6">
	<div class="flex flex-wrap items-start justify-between gap-3 border-b border-separator pb-4">
		<div>
			<h2 id="add-account-title" class="font-display text-base font-semibold text-foreground">{t.title}</h2>
			<p class="mt-1 text-xs text-secondary">{t.discoveryIntro}</p>
		</div>
		<span class="shrink-0 whitespace-nowrap font-mono text-[11px] text-secondary">{obT.manualNetworkOnly}</span>
	</div>

	<form bind:this={form} onsubmit={handleFormSubmit} class="flex flex-col gap-5" autocomplete="off">
		<fieldset disabled={controller.busy || controller.testing} class="flex flex-col gap-5">
			<div class="flex flex-col gap-3">
				<label for="discovery-email" class="text-xs font-medium text-foreground">
					{isArchive ? t.localArchiveEmailLabel : isBridge ? t.emailLabelBridge : t.emailLabel}
				</label>
				<input
					id="discovery-email" bind:this={emailInput} name="email" type="text" required maxlength="320" autocomplete="off" placeholder="alex.work@example.com"
					oninput={(event) => { clearCredentialInputs(form); controller.clearDiagnostics(); if (!isArchive) discovery.input(demoMode ? undefined : event.currentTarget.value); }}
					onblur={(event) => !demoMode && !isArchive && discovery.blur(event.currentTarget.value)}
					class="{INPUT_CLS} w-full"
				/>
				{#if !isArchive}
					<WdgDiscoveryLookupBar
						{discovery} {demoMode} {isBridge} {showAllPresets} {demoIspdb} {demoTls}
						onLookup={() => { advancedExpanded = false; performDiscoveryLookup({ emailValue: emailInput?.value ?? '', demoMode, discovery, invalidMsg: t.discoveryInvalid, onDemoIspdb: (f) => { demoIspdb = f; demoTls = null; } }); }}
						onDetectMx={() => !demoMode && void discovery.detectMx(emailInput?.value ?? '')}
						onProbe={() => {
							if (!demoMode) return void discovery.probe();
							const host = isBridge ? '127.0.0.1' : discovery.settings.host.trim();
							if (!host) return void toast.error(t.discoveryUnresolved);
							demoTls = probeTlsEndpoint(host, isBridge ? 1143 : Number(discovery.settings.port) || 993);
							toast.success(`${host}:${isBridge ? 1143 : Number(discovery.settings.port) || 993}`);
						}}
						onApplyPreset={applyPreset} onToggleAllPresets={() => (showAllPresets = !showAllPresets)}
						onOpenAliasGuide={() => { guideTab = 'EmailRouting'; guideOpen = true; document.getElementById('account-help-guide')?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }); }}
					/>
				{/if}
			</div>

			<div class="flex flex-col gap-4">
				{#if !isArchive}
					<WdgMethodRecommendationCard kind={providerKind} {method} onAction={applyRecommendation} onOffline={() => switchMethod('LocalArchive')} />
				{/if}
				<WdgDiscoveryAuthTabs {providerKind} {method} {showAllMethods} onSelectMethod={switchMethod} onToggleAllMethods={() => (showAllMethods = !showAllMethods)} />

				{#if isArchive}
					<WdgDiscoveryArchiveSection
						{demoMode} {archivePath} {archiveFormat} importProgress={controller.importProgress}
						onFormatChange={(fmt) => (archiveFormat = fmt)}
						onChooseNative={async () => {
							const selected = await controller.transport.chooseArchiveFile?.();
							if (selected) { archivePath = selected; archiveFormat = selected.trim().toLowerCase().endsWith('.eml') ? 'Eml' : 'Mbox'; }
						}}
						onDemoFileChange={(event) => {
							const inputEl = event.currentTarget as HTMLInputElement;
							const file = inputEl.files?.[0];
							inputEl.value = '';
							if (file) { archivePath = file.name; archiveFormat = file.name.trim().toLowerCase().endsWith('.eml') ? 'Eml' : 'Mbox'; }
						}}
					/>
				{:else}
					{#if isBridge}
						<div class="flex items-start gap-3 rounded-md border border-separator bg-field px-4 py-3 text-xs text-secondary">
							<GlobeLock class="mt-1 h-4 w-4 shrink-0 text-accent" aria-hidden="true" />
							<p class="leading-relaxed">{t.protonNotice}</p>
						</div>
					{/if}
					{#if method === 'OAuth2ByoCredentials'}
						<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-4 py-3">
							<span class="text-xs text-secondary">{t.importGoogleJsonHelp}</span>
							<input bind:this={googleJsonInput} type="file" accept=".json,application/json" aria-label={t.importGoogleJson} onchange={(event) => void importGoogleClientJsonFile({ event, form, controller, discovery, messages: { invalid: t.importGoogleJsonInvalidError, webUnsupported: t.importGoogleJsonWebError, success: t.importGoogleJsonSuccess }, onImported: () => { advancedExpanded = false; demoTls = null; } })} class="sr-only" />
							<button type="button" onclick={() => googleJsonInput?.click()} class="flex min-h-[34px] shrink-0 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-selection-soft">
								<FileUp class="h-3.5 w-3.5 shrink-0 text-accent" aria-hidden="true" />
								<span>{t.importGoogleJson}</span>
							</button>
						</div>
					{/if}
					<WdgDiscoveryAdvancedFields {discovery} {isBridge} {oauth} {showAdvanced} inputCls={INPUT_CLS} onToggleAdvanced={() => (advancedExpanded = !showAdvanced)} onEdit={() => { advancedExpanded = true; discovery.edit(); }} />
					<WdgDiscoveryCredentialsSection
						{method} {oauth} {isBridge} {demoMode} selectedPreset={discovery.selectedPreset} {gmailSecretSuspect} inputCls={INPUT_CLS} {demoPkceState} {demoPkceReady} {demoPkceSeconds}
						onProtectCredentials={() => discovery.protectCredentials()}
						onRefreshGmailHint={() => (gmailSecretSuspect = looksLikeRegularGmailPassword(discovery.settings.host, method, form?.querySelector<HTMLInputElement>('input[name="secret"]')?.value ?? ''))}
						onStartDemoPkce={() => { stopDemoTimers(); demoPkceState = 'binding'; demoPkceReady = false; demoPkceSeconds = 0; readyTimer = setTimeout(() => { demoPkceState = 'waiting'; demoPkceReady = true; tickTimer = setInterval(() => (demoPkceSeconds += 1), 1000); }, 250); }}
						onCancelDemoPkce={() => { if (demoPkceReady) { stopDemoTimers(); demoPkceState = 'idle'; demoPkceReady = false; demoPkceSeconds = 0; } }}
						onCompleteDemoPkce={() => { stopDemoTimers(); demoPkceState = 'authorized'; demoPkceReady = false; }}
						onOpenGoogleAppPasswords={() => void controller.transport.openGoogleAppPasswords?.()}
					/>
				{/if}
			</div>

			<WdgDiscoverySubmitBar
				{demoMode} {isArchive} {oauth} hasTestConnection={Boolean(controller.transport.testConnection)}
				discoveryBusy={discovery.busy} controllerBusy={controller.busy} controllerTesting={controller.testing} {archivePath}
				onTestLogin={() => form && !controller.busy && !controller.testing && !discovery.busy && void runDiscoveryTestLogin({ form, method, isBridge, controller, onResetHint: () => (gmailSecretSuspect = false) })}
			/>
		</fieldset>
	</form>

	<WdgDiscoveryFeedback {controller} host={discovery.settings.host} {method} selectedPreset={discovery.selectedPreset} routing={providerKind === 'cloudflare'} bind:guideOpen bind:guideTab />
</section>
