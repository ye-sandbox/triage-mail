<script lang="ts">
	import { tick, untrack } from 'svelte';
	import {
		CheckCircle2,
		ChevronDown,
		FileArchive,
		FileUp,
		GlobeLock,
		KeyRound,
		MailPlus,
		Search,
		ShieldCheck,
		Timer,
		X
	} from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { AccountController } from './controller.svelte';
	import { nativeDiscoveryTransport, type DiscoveryTransport } from './discovery';
	import { DiscoveryController } from './discovery-controller.svelte';
	import { clearCredentialInputs } from './forms';
	import { MAX_GOOGLE_CLIENT_JSON_BYTES, parseGoogleOAuthClientJson } from './google-oauth-json';
	import {
		createDemoAccountDto,
		extractDomainOnly,
		formatPkceTimer,
		lookupIspdbDomain,
		probeTlsEndpoint,
		validateDemoDiscoveryInput,
		type DemoConnectedAccountDto,
		type IspdbPreset,
		type TlsProbeResult
	} from './onboarding.fixtures';
	import { oauthDefaultsForHost, providerPresets, type ProviderPreset } from './presets';
	import type { AccountConfig, AccountSubmission, AuthMethod, LocalArchiveFormat } from './types';
	import WdgAccountHelpGuide from './WdgAccountHelpGuide.svelte';

	interface Props {
		controller: AccountController;
		discoveryTransport?: DiscoveryTransport;
		demoMode?: boolean;
		onAccountAdded?: (account: DemoConnectedAccountDto) => void;
		onSuccess?: () => void;
	}

	let {
		controller,
		discoveryTransport = nativeDiscoveryTransport,
		demoMode = false,
		onAccountAdded,
		onSuccess
	}: Props = $props();

	let form = $state<HTMLFormElement>();
	let emailInput = $state<HTMLInputElement>();
	let googleJsonInput = $state<HTMLInputElement>();
	let demoArchiveInput = $state<HTMLInputElement>();
	let archivePath = $state('');
	let archiveFormat = $state<LocalArchiveFormat>('Mbox');
	let advancedExpanded = $state(false);
	let showAllMethods = $state(false);

	const discovery = new DiscoveryController(
		untrack(() => discoveryTransport),
		() => clearCredentialInputs(form)
	);

	// Demo-only preview state (used strictly when demoMode === true outside Tauri)
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

	const providerKind = $derived.by<'google' | 'microsoft' | 'proton' | 'custom' | 'appPassword'>(() => {
		const host = discovery.settings.host.trim().toLowerCase();
		const presetName = discovery.selectedPreset;
		if (
			presetName === 'Gmail' ||
			presetName === 'Gmail OAuth' ||
			presetName === 'Google Workspace' ||
			host === 'imap.gmail.com' ||
			host === 'imap.googlemail.com'
		) {
			return 'google';
		}
		if (presetName === 'Outlook' || host === 'outlook.office365.com') {
			return 'microsoft';
		}
		if (presetName === 'Proton' || method === 'ProtonBridgeLocal' || host === '127.0.0.1') {
			return 'proton';
		}
		if (presetName === 'Other IMAP' || discovery.status === 'unresolved' || !host) {
			return 'custom';
		}
		return 'appPassword';
	});

	const needsManualServer = $derived(
		!isArchive &&
			!isBridge &&
			(discovery.selectedPreset === 'Other IMAP' ||
				discovery.status === 'unresolved' ||
				!discovery.settings.host.trim() ||
				(oauth &&
					(!discovery.settings.authorize.trim() ||
						!discovery.settings.token.trim() ||
						!discovery.settings.scopes.trim())))
	);
	const showAdvanced = $derived(advancedExpanded || needsManualServer);

	const authTabs = $derived<{ id: AuthMethod; label: string }[]>([
		{ id: 'AppPassword', label: t.methodAppPassword },
		{ id: 'OAuth2PublicPkce', label: t.methodOAuthPublic },
		{ id: 'OAuth2ByoCredentials', label: t.methodOAuthByo },
		{ id: 'ProtonBridgeLocal', label: t.methodProton },
		{ id: 'LocalArchive', label: t.methodLocalArchive }
	]);

	function isTabPrimary(tabId: AuthMethod): boolean {
		if (tabId === method || tabId === 'LocalArchive' || providerKind === 'custom' || showAllMethods) {
			return true;
		}
		switch (providerKind) {
			case 'google':
				return tabId === 'AppPassword' || tabId === 'OAuth2ByoCredentials';
			case 'microsoft':
				return tabId === 'OAuth2PublicPkce' || tabId === 'AppPassword';
			case 'proton':
				return tabId === 'ProtonBridgeLocal';
			default:
				return tabId === 'AppPassword';
		}
	}

	const INPUT_CLS =
		'h-10 w-full rounded-md border border-separator bg-field px-3 text-xs text-foreground placeholder:text-secondary focus:border-accent focus:outline-none disabled:opacity-60';

	function stopDemoTimers() {
		clearTimeout(readyTimer);
		clearInterval(tickTimer);
		readyTimer = undefined;
		tickTimer = undefined;
	}

	$effect(() => {
		const current = form;
		const currentMethod = method;
		const inputs = current?.querySelectorAll<HTMLInputElement>('[data-credential]');
		// INVARIANT: Capture actual DOM inputs including ones detached on auth method change.
		return () => {
			if (currentMethod) {
				inputs?.forEach((input) => {
					input.value = '';
				});
			}
		};
	});

	$effect(() => {
		return () => {
			stopDemoTimers();
			discovery.dispose();
			archivePath = '';
			clearCredentialInputs(form);
			if (controller.authorizing) void controller.cancel();
		};
	});

	function applyPreset(preset: ProviderPreset) {
		clearCredentialInputs(form);
		archivePath = '';
		discovery.preset(preset);
		advancedExpanded = preset.name === 'Other IMAP';
		demoTls = null;
	}

	function selectPresetByName(name: string) {
		const found = providerPresets.find((preset) => preset.name === name);
		if (found) applyPreset(found);
	}

	function inferArchiveFormat(pathOrName: string): LocalArchiveFormat {
		return pathOrName.trim().toLowerCase().endsWith('.eml') ? 'Eml' : 'Mbox';
	}

	async function chooseNativeArchiveFile() {
		if (!controller.transport.chooseArchiveFile) return;
		const selected = await controller.transport.chooseArchiveFile();
		if (selected) {
			archivePath = selected;
			archiveFormat = inferArchiveFormat(selected);
		}
	}

	function handleDemoArchiveChange(event: Event) {
		const inputEl = event.currentTarget as HTMLInputElement;
		const file = inputEl.files?.[0];
		inputEl.value = '';
		if (!file) return;
		archivePath = file.name;
		archiveFormat = inferArchiveFormat(file.name);
	}

	function switchMethod(next: AuthMethod) {
		if (next === method) return;
		clearCredentialInputs(form);
		if (next !== 'LocalArchive') {
			archivePath = '';
		}
		discovery.edit();
		discovery.settings.method = next;
		if (next === 'ProtonBridgeLocal') {
			discovery.settings.host = '127.0.0.1';
			discovery.settings.port = 1143;
		} else if (discovery.settings.host === '127.0.0.1' && discovery.settings.port === 1143) {
			discovery.settings.host = discovery.provider?.imap_host ?? demoIspdb?.imap_host ?? '';
			discovery.settings.port = discovery.provider?.imap_port ?? demoIspdb?.imap_port ?? 993;
		}
		if (next === 'OAuth2PublicPkce' || next === 'OAuth2ByoCredentials') {
			const defaults = oauthDefaultsForHost(discovery.settings.host);
			if (defaults) {
				if (!discovery.settings.authorize.trim()) {
					discovery.settings.authorize = defaults.authEndpoint;
				}
				if (!discovery.settings.token.trim()) {
					discovery.settings.token = defaults.tokenEndpoint;
				}
				if (!discovery.settings.scopes.trim()) {
					discovery.settings.scopes = defaults.scopes;
				}
			}
		}
		demoTls = null;
	}

	function readFileText(file: File): Promise<string> {
		if (typeof file.text === 'function') {
			return file.text();
		}
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.onload = () => resolve(typeof reader.result === 'string' ? reader.result : '');
			reader.onerror = () => reject(reader.error ?? new Error('Failed to read file'));
			reader.readAsText(file);
		});
	}

	async function handleGoogleJsonChange(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		if (file.size > MAX_GOOGLE_CLIENT_JSON_BYTES) {
			toast.error(t.importGoogleJsonInvalidError);
			return;
		}
		const raw = await readFileText(file);
		const parsed = parseGoogleOAuthClientJson(raw);
		if (!parsed.ok) {
			toast.error(
				parsed.reason === 'web_client_unsupported'
					? t.importGoogleJsonWebError
					: t.importGoogleJsonInvalidError
			);
			return;
		}
		clearCredentialInputs(form);
		discovery.edit();
		discovery.selectedPreset = 'Gmail OAuth';
		discovery.settings = {
			host: 'imap.gmail.com',
			port: 993,
			method: 'OAuth2ByoCredentials',
			authorize: parsed.authEndpoint,
			token: parsed.tokenEndpoint,
			scopes: parsed.scopes
		};
		advancedExpanded = false;
		demoTls = null;
		await tick();
		// INVARIANT: Imported client_id/client_secret live only in unbound DOM inputs, never Runes state.
		const clientEl = form?.querySelector<HTMLInputElement>('input[name="client"]');
		const secretEl = form?.querySelector<HTMLInputElement>('input[name="secret"]');
		if (clientEl) clientEl.value = parsed.clientId;
		if (secretEl) secretEl.value = parsed.clientSecret;
		toast.success(t.importGoogleJsonSuccess);
	}

	function handleLookupClick() {
		const emailValue = emailInput?.value ?? '';
		advancedExpanded = false;
		if (!demoMode) {
			void discovery.online(emailValue);
			return;
		}
		const found = lookupIspdbDomain(emailValue);
		if (!found) {
			toast.error(t.discoveryInvalid);
			return;
		}
		demoIspdb = found;
		discovery.edit();
		discovery.settings = {
			host: found.imap_host,
			port: found.imap_port,
			method: found.recommended_auth,
			authorize: found.auth_endpoint ?? '',
			token: found.token_endpoint ?? '',
			scopes: found.default_scopes ?? ''
		};
		demoTls = null;
		toast.success(`ISPDB: ${extractDomainOnly(emailValue)}`);
	}

	function handleProbeClick() {
		if (!demoMode) {
			void discovery.probe();
			return;
		}
		const host = isBridge ? '127.0.0.1' : discovery.settings.host.trim();
		const port = isBridge ? 1143 : Number(discovery.settings.port) || 993;
		if (!host) {
			toast.error(t.discoveryUnresolved);
			return;
		}
		demoTls = probeTlsEndpoint(host, port);
		toast.success(`${host}:${port}`);
	}

	function startDemoPkce() {
		stopDemoTimers();
		demoPkceState = 'binding';
		demoPkceReady = false;
		demoPkceSeconds = 0;
		readyTimer = setTimeout(() => {
			demoPkceState = 'waiting';
			demoPkceReady = true;
			tickTimer = setInterval(() => (demoPkceSeconds += 1), 1000);
		}, 250);
	}

	function cancelDemoPkce() {
		if (!demoPkceReady) return;
		stopDemoTimers();
		demoPkceState = 'idle';
		demoPkceReady = false;
		demoPkceSeconds = 0;
	}

	function completeDemoPkce() {
		stopDemoTimers();
		demoPkceState = 'authorized';
		demoPkceReady = false;
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (controller.busy || discovery.busy || !form) return;
		discovery.invalidate();
		const data = new FormData(form);
		const value = (key: string) => String(data.get(key) ?? '').trim();

		if (isArchive) {
			const cleanEmail = value('email');
			clearCredentialInputs(form);
			if (!cleanEmail || !archivePath) {
				toast.error(t.localArchiveImportError);
				return;
			}
			if (demoMode) {
				onAccountAdded?.(createDemoAccountDto(cleanEmail, '', 0, 'LocalArchive'));
				if (emailInput) emailInput.value = '';
				archivePath = '';
				return;
			}
			const imported = await controller.importArchive(archivePath, cleanEmail, archiveFormat);
			if (imported) {
				toast.success(`${t.localArchiveSuccess} ${imported.imported_count}`);
				archivePath = '';
				if (emailInput) emailInput.value = '';
				onSuccess?.();
			}
			return;
		}

		if (demoMode) {
			const cleanEmail = value('email');
			const finalHost = isBridge ? '127.0.0.1' : value('host');
			const finalPort = isBridge ? 1143 : Number(value('port')) || 993;
			const secretLen = String(data.get('secret') ?? '').trim().length;
			const clientLen = String(data.get('client') ?? '').trim().length;
			for (const key of ['secret', 'client']) data.delete(key);
			clearCredentialInputs(form);
			const err = validateDemoDiscoveryInput({
				email: cleanEmail,
				host: finalHost,
				authMethod: method,
				appSecretLen: method === 'AppPassword' ? secretLen : 0,
				bridgeSecretLen: method === 'ProtonBridgeLocal' ? secretLen : 0,
				pkceAuthorized: demoPkceState === 'authorized',
				clientLen
			});
			if (err) {
				toast.error(err);
				return;
			}
			onAccountAdded?.(createDemoAccountDto(cleanEmail, finalHost, finalPort, method));
			if (emailInput) emailInput.value = '';
			demoIspdb = null;
			demoTls = null;
			demoPkceState = 'idle';
			return;
		}

		const config: AccountConfig = {
			account_id: '',
			email_display: value('email'),
			imap_host: isBridge ? '127.0.0.1' : value('host'),
			imap_port: isBridge ? 1143 : Number(value('port')),
			auth_method: method
		};

		let submission: AccountSubmission;
		if (oauth) {
			submission = {
				authorization: {
					authorization_endpoint: value('authorize'),
					token_endpoint: value('token'),
					scopes: value('scopes'),
					client_id: value('client'),
					...(method === 'OAuth2ByoCredentials'
						? { client_secret: String(data.get('secret') ?? '') }
						: {})
				}
			};
		} else {
			const secret = String(data.get('secret') ?? '');
			submission = {
				credentials: isBridge
					? { ProtonBridgeLocal: { bridge_password: secret } }
					: { AppPassword: { secret } }
			};
		}

		const pending = controller.add(config, submission);
		submission = { credentials: { AppPassword: { secret: '' } } };
		for (const key of ['secret', 'client']) data.delete(key);
		clearCredentialInputs(form);
		const ok = await pending;
		if (ok) {
			if (emailInput) emailInput.value = '';
			onSuccess?.();
		}
	}
</script>

<section
	aria-labelledby="add-account-title"
	class="mx-auto w-full max-w-3xl space-y-5 rounded-lg border border-separator bg-surface p-5 md:p-6"
>
	<div class="flex flex-wrap items-start justify-between gap-3 border-b border-separator pb-4">
		<div>
			<h2 id="add-account-title" class="font-display text-base font-semibold text-foreground">
				{t.title}
			</h2>
			<p class="mt-0.5 text-xs text-secondary">{t.discoveryIntro}</p>
		</div>
		<span class="shrink-0 whitespace-nowrap font-mono text-[11px] text-secondary">
			{obT.manualNetworkOnly}
		</span>
	</div>

	<form bind:this={form} onsubmit={submit} class="flex flex-col gap-5" autocomplete="off">
		<fieldset disabled={controller.busy} class="flex flex-col gap-5">
			<div class="flex flex-col gap-2">
				<label for="discovery-email" class="text-xs font-medium text-foreground">
					{isArchive ? t.localArchiveEmailLabel : isBridge ? t.emailLabelBridge : t.emailLabel}
				</label>
				<div class="flex flex-col gap-2 sm:flex-row">
					<input
						id="discovery-email"
						bind:this={emailInput}
						name="email"
						type="text"
						required
						maxlength="320"
						autocomplete="off"
						placeholder="alex.work@example.com"
						oninput={(event) => {
							clearCredentialInputs(form);
							if (!isArchive) discovery.input(demoMode ? undefined : event.currentTarget.value);
						}}
						onblur={(event) => {
							if (!demoMode && !isArchive) discovery.blur(event.currentTarget.value);
						}}
						class="{INPUT_CLS} flex-1"
					/>
					{#if !isArchive}
						<button
							type="button"
							disabled={discovery.busy}
							onclick={handleLookupClick}
							class="flex min-h-[40px] shrink-0 items-center justify-center gap-1.5 rounded-md border border-separator bg-surface px-3.5 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
						>
							<Search class="h-3.5 w-3.5 shrink-0 text-secondary" />
							<span>{t.onlineLookup}</span>
						</button>
						{#if demoMode || (discovery.settings.host && discovery.settings.port === 993 && !isBridge)}
							<button
								type="button"
								disabled={discovery.busy}
								onclick={handleProbeClick}
								class="flex min-h-[40px] shrink-0 items-center justify-center gap-1.5 rounded-md border border-separator bg-surface px-3.5 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-50"
							>
								<ShieldCheck class="h-3.5 w-3.5 shrink-0 text-secondary" />
								<span>{t.confirmProbe} {discovery.settings.host || '993'}:993</span>
							</button>
						{/if}
					{/if}
				</div>

				{#if !isArchive}
					<div class="flex flex-wrap items-center gap-1.5 pt-1">
						<span class="mr-1 text-[11px] font-medium text-secondary">{t.quickFillLabel}</span>
						{#each providerPresets as preset (preset.name)}
							<button
								type="button"
								aria-pressed={discovery.selectedPreset === preset.name}
								onclick={() => applyPreset(preset)}
								class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {discovery.selectedPreset ===
								preset.name
									? 'border-accent bg-selection-soft text-accent'
									: 'border-separator bg-field text-secondary hover:text-foreground'}"
							>
								{preset.name}
							</button>
						{/each}
					</div>

					<p role="status" class="min-h-4 text-xs text-secondary">
						{#if discovery.busy}
							{t.discovering}
						{:else if discovery.status === 'found'}
							{t.providerFound} {discovery.provider?.provider_name} · {discovery.settings.host}:{discovery
								.settings.port}
						{:else if discovery.status === 'unresolved'}
							{t.discoveryUnresolved}
						{:else if discovery.status === 'invalid'}
							{t.discoveryInvalid}
						{:else if discovery.status === 'unavailable'}
							{t.discoveryUnavailable}
						{:else if discovery.status === 'probeSuccess'}
							{t.probeSuccess}
						{:else if discovery.status === 'probeFailed'}
							{t.probeFailed}
						{:else if demoIspdb}
							{demoIspdb.note}
						{/if}
					</p>

					{#if demoTls}
						<div
							class="flex flex-wrap items-center gap-2 rounded-md border border-separator bg-field px-3 py-2 font-mono text-[11px] text-foreground"
						>
							<CheckCircle2 class="h-3.5 w-3.5 shrink-0 text-status-healthy" />
							<span>{demoTls.host}:{demoTls.port}</span>
							<span aria-hidden="true">·</span>
							<span>{demoTls.protocol}</span>
							<span aria-hidden="true">·</span>
							<span class="truncate text-secondary">{demoTls.spki_sha256}</span>
						</div>
					{/if}
				{/if}
			</div>

			<div class="flex flex-col gap-4">
				{#if !isArchive}
					<div class="space-y-2.5 rounded-md border border-separator bg-field/70 px-3.5 py-3">
						<p class="text-xs leading-relaxed text-secondary">
							{#if providerKind === 'google'}
								{t.providerGuidanceGoogle}
							{:else if providerKind === 'microsoft'}
								{t.providerGuidanceMicrosoft}
							{:else if providerKind === 'proton'}
								{t.providerGuidanceProton}
							{:else if providerKind === 'custom'}
								{t.providerGuidanceCustom}
							{:else}
								{t.providerGuidanceAppPassword}
							{/if}
						</p>
						<div class="flex flex-wrap items-center gap-2">
							{#if providerKind === 'google'}
								<button
									type="button"
									onclick={() => selectPresetByName('Gmail')}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'AppPassword'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.methodAppPassword} · {t.recommendedOption}
								</button>
								<button
									type="button"
									onclick={() => selectPresetByName('Gmail OAuth')}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'OAuth2ByoCredentials'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.googleCloudJsonOption}
								</button>
							{:else if providerKind === 'microsoft'}
								<button
									type="button"
									onclick={() => selectPresetByName('Outlook')}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'OAuth2PublicPkce'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.microsoftSetup} · {t.recommendedOption}
								</button>
								<button
									type="button"
									onclick={() => switchMethod('AppPassword')}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'AppPassword'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.methodAppPassword}
								</button>
							{:else if providerKind === 'proton'}
								<button
									type="button"
									onclick={() => selectPresetByName('Proton')}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'ProtonBridgeLocal'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.methodProton} · {t.recommendedOption}
								</button>
							{:else if providerKind === 'custom'}
								<button
									type="button"
									onclick={() => {
										switchMethod('AppPassword');
										advancedExpanded = true;
									}}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'AppPassword'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.customImapOption}
								</button>
							{:else}
								<button
									type="button"
									onclick={() => switchMethod('AppPassword')}
									class="rounded-md border px-2.5 py-1 text-[11px] font-medium transition-colors {method === 'AppPassword'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-foreground hover:bg-selection-soft'}"
								>
									{t.methodAppPassword} · {t.recommendedOption}
								</button>
								<button
									type="button"
									onclick={() => (advancedExpanded = true)}
									class="rounded-md border border-separator bg-surface px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-selection-soft"
								>
									{t.customImapOption}
								</button>
							{/if}
							<button
								type="button"
								onclick={() => switchMethod('LocalArchive')}
								class="rounded-md border border-separator bg-surface px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-selection-soft"
							>
								{t.offlineImportOption}
							</button>
						</div>
					</div>
				{/if}

				<div class="flex flex-wrap items-center gap-2">
					<div
						role="tablist"
						aria-label={t.methodLabel}
						class="flex flex-1 flex-wrap gap-1 rounded-md border border-separator bg-field p-1"
					>
						{#each authTabs as tab (tab.id)}
							<button
								type="button"
								role="tab"
								hidden={!isTabPrimary(tab.id)}
								aria-selected={method === tab.id}
								onclick={() => switchMethod(tab.id)}
								class="min-h-[36px] flex-1 rounded px-2.5 py-1.5 text-xs font-medium whitespace-nowrap transition-colors {method ===
								tab.id
									? 'bg-surface text-foreground shadow-xs'
									: 'text-secondary hover:text-foreground'}"
							>
								{tab.label}
							</button>
						{/each}
					</div>
					{#if providerKind !== 'custom'}
						<button
							type="button"
							aria-expanded={showAllMethods}
							onclick={() => (showAllMethods = !showAllMethods)}
							class="min-h-[38px] shrink-0 rounded-md border border-separator bg-surface px-3 py-1.5 text-[11px] font-medium text-secondary whitespace-nowrap hover:bg-field hover:text-foreground"
						>
							{showAllMethods ? t.fewerSignInMethods : t.allSignInMethods}
						</button>
					{/if}
				</div>

				{#if isArchive}
					<div
						class="flex items-start gap-2.5 rounded-md border border-separator bg-field px-3.5 py-2.5 text-xs text-secondary"
					>
						<FileArchive class="mt-0.5 h-4 w-4 shrink-0 text-accent" />
						<p class="leading-relaxed">{t.localArchiveNotice}</p>
					</div>

					<div class="flex flex-col gap-3 rounded-md border border-separator bg-field p-3.5">
						<div class="flex flex-col gap-1.5">
							<span class="text-xs font-medium text-secondary">{t.localArchiveFormatLabel}</span>
							<div role="radiogroup" aria-label={t.localArchiveFormatLabel} class="grid grid-cols-1 gap-2 sm:grid-cols-2">
								<button
									type="button"
									role="radio"
									aria-checked={archiveFormat === 'Mbox'}
									onclick={() => (archiveFormat = 'Mbox')}
									class="flex min-h-[38px] items-center justify-center rounded-md border px-3 py-1.5 text-xs font-medium transition-colors {archiveFormat === 'Mbox'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-secondary hover:text-foreground'}"
								>
									{t.localArchiveFormatMbox}
								</button>
								<button
									type="button"
									role="radio"
									aria-checked={archiveFormat === 'Eml'}
									onclick={() => (archiveFormat = 'Eml')}
									class="flex min-h-[38px] items-center justify-center rounded-md border px-3 py-1.5 text-xs font-medium transition-colors {archiveFormat === 'Eml'
										? 'border-accent bg-selection-soft text-accent'
										: 'border-separator bg-surface text-secondary hover:text-foreground'}"
								>
									{t.localArchiveFormatEml}
								</button>
							</div>
						</div>

						<div class="flex flex-wrap items-center justify-between gap-2">
							<span class="min-w-0 flex-1 truncate font-mono text-xs text-foreground">
								{archivePath || t.localArchiveNoFile}
							</span>
							{#if demoMode}
								<input
									bind:this={demoArchiveInput}
									type="file"
									accept=".mbox,.eml,message/rfc822"
									onchange={handleDemoArchiveChange}
									class="sr-only"
									aria-label={t.localArchiveChooseFile}
								/>
							{/if}
							<button
								type="button"
								onclick={() => (demoMode ? demoArchiveInput?.click() : void chooseNativeArchiveFile())}
								class="flex min-h-[36px] shrink-0 items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-foreground whitespace-nowrap hover:bg-selection-soft"
							>
								<FileUp class="h-3.5 w-3.5 shrink-0 text-accent" />
								<span>{t.localArchiveChooseFile}</span>
							</button>
						</div>

						{#if controller.importProgress.active}
							<p role="status" class="font-mono text-xs tabular-nums text-status-info">
								{t.localArchiveProgress} {controller.importProgress.importedCount}
							</p>
						{/if}
					</div>
				{:else}
					{#if isBridge}
						<div
							class="flex items-start gap-2.5 rounded-md border border-separator bg-field px-3.5 py-2.5 text-xs text-secondary"
						>
							<GlobeLock class="mt-0.5 h-4 w-4 shrink-0 text-accent" />
							<p class="leading-relaxed">{t.protonNotice}</p>
						</div>
					{/if}

					{#if method === 'OAuth2ByoCredentials'}
						<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3.5 py-2.5">
							<span class="text-xs text-secondary">{t.importGoogleJsonHelp}</span>
							<input
								bind:this={googleJsonInput}
								type="file"
								accept=".json,application/json"
								aria-label={t.importGoogleJson}
								onchange={handleGoogleJsonChange}
								class="sr-only"
							/>
							<button
								type="button"
								onclick={() => googleJsonInput?.click()}
								class="flex min-h-[34px] shrink-0 items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-foreground whitespace-nowrap hover:bg-selection-soft"
							>
								<FileUp class="h-3.5 w-3.5 shrink-0 text-accent" />
								<span>{t.importGoogleJson}</span>
							</button>
						</div>
					{/if}

					<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3.5 py-2.5">
						<span class="min-w-0 truncate font-mono text-[11px] text-secondary">
							{discovery.settings.host.trim()
								? `${t.resolvedServerSummary} · ${discovery.settings.host}:${discovery.settings.port}`
								: t.customImapOption}
						</span>
						<button
							type="button"
							aria-expanded={showAdvanced}
							aria-controls="advanced-connection-settings"
							onclick={() => (advancedExpanded = !showAdvanced)}
							class="flex min-h-[32px] shrink-0 items-center gap-1.5 rounded-md border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-selection-soft"
						>
							<span>{t.advancedSettings}</span>
							<ChevronDown
								class="h-3.5 w-3.5 transition-transform duration-150 {showAdvanced ? 'rotate-180' : ''}"
							/>
						</button>
					</div>

					<div
						id="advanced-connection-settings"
						hidden={!showAdvanced}
						class="space-y-3 rounded-md border border-separator bg-field/50 p-3.5"
					>
						<div class="grid grid-cols-1 gap-3 sm:grid-cols-6">
							<label class="flex flex-col gap-1 sm:col-span-4">
								<span class="text-xs font-medium text-secondary">{t.hostLabel}</span>
								<input
									name="host"
									type="text"
									required
									maxlength="253"
									readonly={isBridge}
									bind:value={discovery.settings.host}
									oninput={() => {
										advancedExpanded = true;
										discovery.edit();
									}}
									placeholder={isBridge ? '127.0.0.1' : 'imap.example.com'}
									class="{INPUT_CLS} font-mono"
								/>
							</label>
							<label class="flex flex-col gap-1 sm:col-span-2">
								<span class="text-xs font-medium text-secondary">{t.portLabel}</span>
								<input
									name="port"
									type="number"
									required
									min="1"
									max="65535"
									readonly={isBridge}
									bind:value={discovery.settings.port}
									oninput={() => {
										advancedExpanded = true;
										discovery.edit();
									}}
									class="{INPUT_CLS} font-mono tabular-nums"
								/>
							</label>
						</div>

						{#if oauth}
							<p class="text-xs text-secondary">{t.oauthHelp}</p>
							<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
								<label class="flex flex-col gap-1">
									<span class="text-xs font-medium text-secondary">{t.authEndpointLabel}</span>
									<input
										name="authorize"
										type="url"
										required
										pattern="https://.*"
										maxlength="2048"
										bind:value={discovery.settings.authorize}
										oninput={() => {
											advancedExpanded = true;
											discovery.edit();
										}}
										class="{INPUT_CLS} font-mono"
									/>
								</label>
								<label class="flex flex-col gap-1">
									<span class="text-xs font-medium text-secondary">{t.tokenEndpointLabel}</span>
									<input
										name="token"
										type="url"
										required
										pattern="https://.*"
										maxlength="2048"
										bind:value={discovery.settings.token}
										oninput={() => {
											advancedExpanded = true;
											discovery.edit();
										}}
										class="{INPUT_CLS} font-mono"
									/>
								</label>
								<label class="flex flex-col gap-1 sm:col-span-2">
									<span class="text-xs font-medium text-secondary">{t.scopesLabel}</span>
									<input
										name="scopes"
										type="text"
										required
										maxlength="4096"
										bind:value={discovery.settings.scopes}
										oninput={() => {
											advancedExpanded = true;
											discovery.edit();
										}}
										class="{INPUT_CLS} font-mono"
									/>
									<span class="text-[11px] text-secondary">{t.scopesHelp}</span>
								</label>
							</div>
						{/if}
					</div>

					{#key method}
						<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
							{#if oauth}
								<label
									class="flex flex-col gap-1 {method === 'OAuth2PublicPkce' ? 'sm:col-span-2' : ''}"
								>
									<span class="text-xs font-medium text-secondary">{t.clientIdLabel}</span>
									<input
										name="client"
										type="text"
										data-credential
										required
										maxlength="4096"
										autocomplete="off"
										oninput={() => discovery.protectCredentials()}
										class="{INPUT_CLS} font-mono"
									/>
								</label>
							{/if}
							{#if method !== 'OAuth2PublicPkce'}
								<label class="flex flex-col gap-1 {!oauth ? 'sm:col-span-2' : ''}">
									<span class="text-xs font-medium text-secondary">
										{method === 'OAuth2ByoCredentials'
											? t.clientSecretLabel
											: isBridge
												? t.bridgePasswordLabel
												: t.appPasswordLabel}
									</span>
									<input
										name="secret"
										type="password"
										data-credential
										required
										maxlength="8192"
										autocomplete="off"
										oninput={() => discovery.protectCredentials()}
										class="{INPUT_CLS} font-mono"
									/>
								</label>
							{/if}
						</div>
					{/key}

					{#if demoMode && method === 'OAuth2PublicPkce'}
						<div class="flex flex-wrap items-center gap-2 rounded-md border border-separator bg-field p-3">
							<button
								type="button"
								onclick={startDemoPkce}
								disabled={demoPkceState === 'binding' || demoPkceState === 'waiting'}
								class="flex min-h-[38px] items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground disabled:opacity-60"
							>
								<KeyRound class="h-3.5 w-3.5" />
								<span>{t.authorizeAndConnect} (127.0.0.1 PKCE)</span>
							</button>
							<button
								type="button"
								onclick={cancelDemoPkce}
								disabled={!demoPkceReady}
								class="flex min-h-[38px] items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-foreground disabled:opacity-40"
							>
								<X class="h-3.5 w-3.5" />
								<span>{t.cancelAuth}</span>
							</button>
							{#if demoPkceState === 'binding' || demoPkceState === 'waiting'}
								<span class="flex items-center gap-1.5 font-mono text-xs tabular-nums text-status-info">
									<Timer class="h-3.5 w-3.5" />
									<span>{formatPkceTimer(demoPkceSeconds)}</span>
								</span>
								{#if demoPkceReady}
									<button
										type="button"
										onclick={completeDemoPkce}
										class="ml-auto rounded border border-separator bg-surface px-2.5 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
									>
										Simulate Loopback Callback
									</button>
								{/if}
							{:else if demoPkceState === 'authorized'}
								<span class="flex items-center gap-1.5 text-xs font-medium text-status-healthy">
									<CheckCircle2 class="h-3.5 w-3.5" />
									<span>127.0.0.1 PKCE OK</span>
								</span>
							{/if}
						</div>
					{/if}

					{#if discovery.selectedPreset === 'Gmail' && method === 'AppPassword'}
						<a
							class="text-xs text-accent underline"
							href="https://myaccount.google.com/apppasswords"
							target="_blank"
							rel="noopener noreferrer"
						>
							{t.gmailAppPasswordHelp}
						</a>
					{/if}
				{/if}
			</div>

			<div class="flex flex-wrap items-center justify-between gap-3 border-t border-separator pt-4">
				<p class="text-xs text-secondary">
					{isArchive ? t.localArchiveNoticeFooter : oauth ? t.oauthVaultNotice : t.connectVaultNotice}
				</p>
				<button
					type="submit"
					disabled={discovery.busy || controller.busy || (isArchive && !archivePath)}
					class="flex min-h-[42px] shrink-0 items-center gap-2 rounded-md bg-accent px-5 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap hover:opacity-95 active:scale-[0.98] disabled:opacity-60 md:min-h-[38px]"
				>
					{#if isArchive}
						<FileArchive class="h-3.5 w-3.5" />
						<span>{controller.busy ? t.localArchiveImporting : t.localArchiveSubmit}</span>
					{:else}
						<MailPlus class="h-3.5 w-3.5" />
						<span>{controller.busy ? t.connecting : oauth ? t.authorizeAndConnect : t.connectAccount}</span>
					{/if}
				</button>
			</div>
		</fieldset>
	</form>

	{#if controller.authorizing}
		<div
			role="status"
			class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-separator bg-field px-4 py-3 text-xs"
		>
			<span class="text-foreground">{t.browserAuthPending}</span>
			<button
				type="button"
				onclick={() => void controller.cancel()}
				class="flex min-h-[34px] items-center gap-1.5 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
			>
				<X class="h-3.5 w-3.5" />
				<span>{t.cancelAuth}</span>
			</button>
		</div>
	{/if}

	{#if controller.error}
		<p role="alert" class="rounded-md border border-separator bg-field px-4 py-3 text-xs font-medium text-status-danger">
			{controller.error}
		</p>
	{/if}

	<WdgAccountHelpGuide selectedMethod={method} selectedPreset={discovery.selectedPreset} />
</section>
