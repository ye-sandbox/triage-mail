<script lang="ts">
	import { untrack } from 'svelte';
	import { Download, FileArchive, FileJson, ShieldAlert, Upload } from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { transferGuards } from '$lib/backup/guards';
	import {
		transfer,
		type Definitions,
		type Mapping,
		type TransferTransport
	} from '$lib/backup/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { clearCredentialInputs } from '$lib/onboarding/forms';
	import type { AccountConfig } from '$lib/onboarding/types';
	import { appStore } from '$lib/stores/app.svelte';
	import { SettingsFormGuard } from './guards';
	import { settingsLabels } from './labels';
	import {
		PORTABLE_IMPORT_PREVIEW_FIXTURE,
		type PortableImportPreview
	} from './settings.fixtures';
	import { buildImportPreviewFromJson, triggerBlobDownload } from './settings.helpers';
	import WdgImportPreviewDialog from './WdgImportPreviewDialog.svelte';

	interface Props {
		transport?: TransferTransport;
		accounts?: AccountConfig[];
		guard?: SettingsFormGuard;
		restore?: boolean;
		ondone?: () => void;
	}

	let {
		transport: transportProp,
		accounts = [],
		guard = new SettingsFormGuard(),
		restore = false,
		ondone = () => {}
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].backup);
	const useNativeFlow = $derived(Boolean(transportProp) || nativeAvailable());
	const activeTransport = $derived(transportProp ?? transfer);
	const guards = transferGuards(untrack(() => guard));

	// Form element refs for unbound credential zeroization
	let exportFormEl = $state<HTMLFormElement>();
	let restoreFormEl = $state<HTMLFormElement>();
	let restoreFileEl = $state<HTMLInputElement>();
	let portableFileEl = $state<HTMLInputElement>();

	// Native encrypted backup state
	let backupConsent = $state(false);
	let backupBusy = $state(false);
	let backupStatus = $state('');
	let backupError = $state(false);

	// Native recovery restore state (available only on fresh install or explicit restore mode)
	let recoveryAvailable = $state(false);
	let restoreConsent = $state(false);
	let restoreBusy = $state(false);
	let restoreFinished = $state(false);
	let restoreStatus = $state('');
	let restoreError = $state(false);

	// Native portable definitions state
	let nativePreview = $state<Definitions | null>(null);
	let nativeMapping = $state<Mapping>({ accounts: {}, destinations: {} });
	let nativeDestinations = $state<{ id: string; name: string }[]>([]);
	let nativePath = '';
	let nativeConsent = $state(false);
	let portableBusy = $state(false);
	let portableStatus = $state('');
	let portableError = $state(false);
	let importModalOpen = $state(false);

	// Browser demo mode state (used strictly when !nativeAvailable() and no transport is injected)
	let demoRestoreFile = $state<string | null>(null);
	let demoImportPreview = $state<PortableImportPreview>(
		structuredClone(PORTABLE_IMPORT_PREVIEW_FIXTURE)
	);

	let active = true;

	const anyBusy = $derived(backupBusy || restoreBusy || portableBusy || guard.isBlocked());
	const nativeComplete = $derived(
		Boolean(
			nativePreview &&
				nativePreview.accounts.every((id) => Boolean(nativeMapping.accounts[id])) &&
				nativePreview.destinations.every((id) => Boolean(nativeMapping.destinations[id]))
		)
	);

	$effect(() => {
		active = true;
		if (useNativeFlow && !restore) {
			void activeTransport
				.available()
				.then((ok) => {
					if (active) recoveryAvailable = ok;
				})
				.catch(() => {
					if (active) recoveryAvailable = false;
				});
		} else if (restore) {
			recoveryAvailable = true;
		}
		return () => {
			active = false;
			clearCredentialInputs(exportFormEl);
			clearCredentialInputs(restoreFormEl);
			nativePreview = null;
			nativePath = '';
			nativeMapping = { accounts: {}, destinations: {} };
			importModalOpen = false;
			guards.backup.reset();
			guards.portable.reset();
		};
	});

	function resetNativePreviewState() {
		nativePreview = null;
		nativePath = '';
		nativeMapping = { accounts: {}, destinations: {} };
		nativeConsent = false;
		importModalOpen = false;
		guards.portable.markDirty(false);
	}

	async function handleNativeBackupOrRestore(event: SubmitEvent, isRestore: boolean) {
		event.preventDefault();
		const targetForm = isRestore ? restoreFormEl : exportFormEl;
		const consentAccepted = isRestore ? restoreConsent : backupConsent;
		if (!targetForm || anyBusy || !consentAccepted || (isRestore && restoreFinished)) return;

		const secret = targetForm.elements.namedItem('passphrase') as HTMLInputElement | null;
		const confirm = targetForm.elements.namedItem('confirmation') as HTMLInputElement | null;
		const passphrase = secret?.value ?? '';
		const confirmation = confirm?.value ?? '';

		if (passphrase.length < 12 || passphrase !== confirmation) {
			if (isRestore) {
				restoreError = true;
				restoreStatus = t.mismatch;
			} else {
				backupError = true;
				backupStatus = t.mismatch;
			}
			return;
		}

		if (isRestore) {
			restoreBusy = true;
			restoreError = false;
			restoreStatus = '';
		} else {
			backupBusy = true;
			backupError = false;
			backupStatus = '';
		}
		guards.backup.setBusy(true);

		try {
			const path = await activeTransport.choose(!isRestore);
			if (!active || !path) {
				clearCredentialInputs(targetForm);
				return;
			}
			// INVARIANT: Zeroize unbound passphrase inputs before awaiting native IPC.
			clearCredentialInputs(targetForm);
			if (isRestore) {
				await activeTransport.restore(path, passphrase, true);
				if (active) {
					restoreFinished = true;
					restoreConsent = false;
					restoreStatus = t.restored;
					guards.backup.markDirty(false);
					toast.success(t.restored);
				}
			} else {
				await activeTransport.backup(path, passphrase);
				if (active) {
					backupConsent = false;
					backupStatus = t.done;
					guards.backup.markDirty(false);
					toast.success(t.done);
				}
			}
		} catch {
			if (active) {
				if (isRestore) {
					restoreError = true;
					restoreStatus = t.error;
				} else {
					backupError = true;
					backupStatus = t.error;
				}
				toast.error(t.error);
			}
		} finally {
			clearCredentialInputs(targetForm);
			if (active) {
				if (isRestore) restoreBusy = false;
				else backupBusy = false;
				guards.backup.setBusy(false);
			}
		}
	}

	async function runNativePortable(action: 'export' | 'preview' | 'import') {
		if (anyBusy) return;
		portableBusy = true;
		portableError = false;
		portableStatus = '';
		guards.portable.setBusy(true);
		try {
			if (action === 'import') {
				if (!nativePreview || !nativeConsent || !nativeComplete) return;
				await activeTransport.import(
					nativePath,
					$state.snapshot(nativeMapping),
					$state.snapshot(nativePreview),
					true
				);
				if (typeof window !== 'undefined') {
					window.dispatchEvent(new Event('triage-definitions-imported'));
				}
				if (active) {
					resetNativePreviewState();
					portableStatus = t.done;
					toast.success(t.done);
				}
				return;
			}

			const selected = await activeTransport.choose(action === 'export');
			if (!active || !selected) return;
			if (action === 'export') {
				await activeTransport.export(selected);
				if (active) {
					portableStatus = t.done;
					toast.success(t.done);
				}
			} else {
				const value = await activeTransport.preview(selected);
				const targets = await activeTransport.destinations();
				if (active) {
					nativePreview = value;
					nativeDestinations = targets;
					nativePath = selected;
					nativeMapping = { accounts: {}, destinations: {} };
					nativeConsent = false;
					guards.portable.markDirty(true);
					importModalOpen = true;
				}
			}
		} catch {
			if (active) {
				portableError = true;
				portableStatus = t.error;
				toast.error(t.error);
			}
		} finally {
			if (active) {
				portableBusy = false;
				guards.portable.setBusy(false);
			}
		}
	}

	// Demo-only browser fallback handlers (unreachable inside native runtime or injected tests)
	function handleDemoExport(event: SubmitEvent) {
		event.preventDefault();
		if (!exportFormEl) return;
		const secret = exportFormEl.elements.namedItem('passphrase') as HTMLInputElement | null;
		const confirm = exportFormEl.elements.namedItem('confirmation') as HTMLInputElement | null;
		const pass = secret?.value ?? '';
		const conf = confirm?.value ?? '';
		clearCredentialInputs(exportFormEl);

		if (pass.length < 12 || pass !== conf) {
			backupError = true;
			backupStatus = t.mismatch;
			toast.error(t.mismatch);
			return;
		}
		backupError = false;
		backupStatus = t.done;
		toast.success('Demo encrypted backup created locally.');
	}

	function handleDemoRestoreFile(event: Event) {
		const file =
			event.currentTarget instanceof HTMLInputElement ? event.currentTarget.files?.[0] : null;
		if (!file) return;
		if (!file.name.endsWith('.triagebak')) {
			toast.error(t.error);
			if (restoreFileEl) restoreFileEl.value = '';
			return;
		}
		demoRestoreFile = file.name;
	}

	function handleDemoRestore(event: SubmitEvent) {
		event.preventDefault();
		if (!restoreFormEl) return;
		const secret = restoreFormEl.elements.namedItem('passphrase') as HTMLInputElement | null;
		const pass = secret?.value ?? '';
		clearCredentialInputs(restoreFormEl);

		if (!demoRestoreFile || pass.length < 12) {
			toast.error(t.error);
			return;
		}
		toast.success(`Verified demo snapshot "${demoRestoreFile}"`);
		demoRestoreFile = null;
		if (restoreFileEl) restoreFileEl.value = '';
	}

	function exportDemoPortable() {
		const payload = {
			schema_version: 'triage-portable-v1',
			exported_at_iso: new Date().toISOString(),
			rules: appStore.rules,
			saved_views: appStore.savedViews,
			categories: ['Security', 'Financial', 'Shopping', 'Newsletter', 'Notifications', 'Personal']
		};
		triggerBlobDownload(
			'triage-definitions.json',
			JSON.stringify(payload, null, 2),
			'application/json'
		);
		toast.success(t.done);
	}

	async function handleDemoPortableFile(event: Event) {
		const file =
			event.currentTarget instanceof HTMLInputElement ? event.currentTarget.files?.[0] : null;
		if (!file) return;
		try {
			demoImportPreview = buildImportPreviewFromJson(file.name, await file.text());
			importModalOpen = true;
		} catch {
			toast.error(t.error);
		} finally {
			if (portableFileEl) portableFileEl.value = '';
		}
	}

	function openDemoSamplePreview() {
		demoImportPreview = structuredClone(PORTABLE_IMPORT_PREVIEW_FIXTURE);
		importModalOpen = true;
	}

	function resolveDemoConflict(id: string, resolution: 'keep_local' | 'overwrite') {
		demoImportPreview = {
			...demoImportPreview,
			items: demoImportPreview.items.map((item) =>
				item.id === id ? { ...item, resolution } : item
			)
		};
	}

	function commitDemoImport() {
		importModalOpen = false;
		const { added, updated, conflicting } = demoImportPreview.counts;
		toast.success(`Imported ${added} added, ${updated} updated, ${conflicting} resolved definitions`);
	}
</script>

<section class="flex flex-col gap-5" aria-labelledby="backup-heading">
	<div class="border-b border-separator pb-3">
		<h2 id="backup-heading" class="font-display text-base font-semibold text-foreground">
			{t.heading}
		</h2>
		<p class="mt-0.5 text-xs text-secondary">{t.subtitle}</p>
	</div>

	<div class="rounded-lg border border-separator bg-field p-3.5 text-xs leading-relaxed text-secondary space-y-1.5">
		<p>{t.help}</p>
		<p>{t.newFile}</p>
	</div>

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
		{#if !restore}
			<form
				bind:this={exportFormEl}
				onsubmit={(e) =>
					useNativeFlow ? void handleNativeBackupOrRestore(e, false) : handleDemoExport(e)}
				oninput={() => guards.backup.markDirty(true)}
				aria-label={t.exportTitle}
				autocomplete="off"
				class="flex flex-col justify-between gap-4 rounded-lg border border-separator bg-surface p-4"
			>
				<div class="space-y-3">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<FileArchive class="h-4 w-4 text-secondary" />
						<span>{t.exportTitle}</span>
					</div>
					<p class="text-xs text-secondary">{t.exportDesc}</p>
					<div class="grid grid-cols-1 gap-2.5">
						<label class="flex flex-col gap-1 text-xs text-foreground">
							<span>{t.passLabel}</span>
							<input
								name="passphrase"
								type="password"
								data-credential
								required
								minlength="12"
								maxlength="4096"
								disabled={anyBusy}
								autocomplete="off"
								class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
							/>
						</label>
						<label class="flex flex-col gap-1 text-xs text-foreground">
							<span>{t.confirmLabel}</span>
							<input
								name="confirmation"
								type="password"
								data-credential
								required
								minlength="12"
								maxlength="4096"
								disabled={anyBusy}
								autocomplete="off"
								class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
							/>
						</label>
						<label class="flex cursor-pointer items-start gap-2 pt-1 text-xs text-secondary">
							<input
								type="checkbox"
								bind:checked={backupConsent}
								disabled={anyBusy}
								class="mt-0.5 h-3.5 w-3.5 accent-accent"
							/>
							<span>{t.consentLabel}</span>
						</label>
					</div>
				</div>

				<div class="space-y-2">
					{#if backupStatus}
						<p
							role={backupError ? 'alert' : 'status'}
							class="text-xs {backupError ? 'text-status-danger' : 'text-status-healthy'}"
						>
							{backupStatus}
						</p>
					{/if}
					<button
						type="submit"
						disabled={anyBusy || (useNativeFlow && !backupConsent)}
						class="flex min-h-[38px] w-full items-center justify-center gap-2 whitespace-nowrap rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
					>
						<Download class="h-3.5 w-3.5" />
						<span>{backupBusy ? t.busy : t.exportBtn}</span>
					</button>
				</div>
			</form>
		{/if}

		{#if restore || !useNativeFlow}
			<form
				bind:this={restoreFormEl}
				onsubmit={(e) =>
					useNativeFlow ? void handleNativeBackupOrRestore(e, true) : handleDemoRestore(e)}
				oninput={() => guards.backup.markDirty(true)}
				aria-label={t.restoreTitle}
				autocomplete="off"
				class="flex flex-col justify-between gap-4 rounded-lg border border-separator bg-surface p-4"
			>
				<div class="space-y-3">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<Upload class="h-4 w-4 text-secondary" />
						<span>{t.restoreTitle}</span>
					</div>
					<p class="text-xs text-secondary">{t.recovery}</p>

					{#if !restoreFinished}
						{#if !useNativeFlow}
							<input
								bind:this={restoreFileEl}
								type="file"
								accept=".triagebak"
								onchange={handleDemoRestoreFile}
								class="hidden"
								aria-label="Select .triagebak file"
							/>
							<button
								type="button"
								onclick={() => restoreFileEl?.click()}
								class="flex h-9 w-full items-center justify-between rounded-md border border-separator bg-field px-3 text-xs text-foreground hover:bg-selection-soft"
							>
								<span class="truncate font-mono">{demoRestoreFile ?? 'Select .triagebak file…'}</span>
								<span class="shrink-0 text-secondary">Browse</span>
							</button>
						{/if}

						<label class="flex flex-col gap-1 text-xs text-foreground">
							<span>{t.passLabel}</span>
							<input
								name="passphrase"
								type="password"
								data-credential
								required
								minlength="12"
								maxlength="4096"
								disabled={anyBusy}
								autocomplete="off"
								class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
							/>
						</label>

						{#if useNativeFlow}
							<label class="flex flex-col gap-1 text-xs text-foreground">
								<span>{t.confirmLabel}</span>
								<input
									name="confirmation"
									type="password"
									data-credential
									required
									minlength="12"
									maxlength="4096"
									disabled={anyBusy}
									autocomplete="off"
									class="h-9 rounded-md border border-separator bg-field px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
								/>
							</label>
							<label class="flex cursor-pointer items-start gap-2 pt-1 text-xs text-secondary">
								<input
									type="checkbox"
									bind:checked={restoreConsent}
									disabled={anyBusy}
									class="mt-0.5 h-3.5 w-3.5 accent-accent"
								/>
								<span>{t.consentLabel}</span>
							</label>
						{/if}
					{/if}
				</div>

				<div class="space-y-2">
					{#if restoreStatus}
						<p
							role={restoreError ? 'alert' : 'status'}
							class="text-xs {restoreError ? 'text-status-danger' : 'text-status-healthy'}"
						>
							{restoreStatus}
						</p>
					{/if}

					{#if restoreFinished}
						<button
							type="button"
							onclick={ondone}
							class="flex min-h-[38px] w-full items-center justify-center rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95"
						>
							{t.continueBtn}
						</button>
					{:else}
						<button
							type="submit"
							disabled={anyBusy || (useNativeFlow && !restoreConsent)}
							class="flex min-h-[38px] w-full items-center justify-center gap-2 whitespace-nowrap rounded-md border border-separator bg-field px-4 py-2 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
						>
							<FileArchive class="h-3.5 w-3.5" />
							<span>{restoreBusy ? t.busy : t.restoreBtn}</span>
						</button>
					{/if}
				</div>
			</form>
		{:else}
			<div class="flex flex-col justify-between gap-4 rounded-lg border border-separator bg-surface p-4">
				<div class="space-y-2.5">
					<div class="flex items-center gap-2 text-sm font-medium text-foreground">
						<ShieldAlert class="h-4 w-4 text-status-warning" />
						<span>{t.restoreTitle}</span>
					</div>
					<p class="text-xs font-medium text-foreground">{t.restoreUnavailableTitle}</p>
					<p class="text-xs leading-relaxed text-secondary">{t.recovery}</p>
				</div>
				<span class="font-mono text-[11px] text-secondary">
					recoveryAvailable: {recoveryAvailable ? 'true' : 'false'}
				</span>
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4">
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div class="max-w-xl space-y-1">
				<div class="flex items-center gap-2 text-sm font-medium text-foreground">
					<FileJson class="h-4 w-4 text-secondary" />
					<span>{t.portableTitle}</span>
				</div>
				<p class="text-xs text-secondary">
					{useNativeFlow ? t.portableDisclosure : t.portableDesc}
				</p>
			</div>

			<div class="flex shrink-0 flex-wrap items-center gap-2">
				{#if useNativeFlow}
					<button
						type="button"
						disabled={anyBusy}
						onclick={() => void runNativePortable('export')}
						class="flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft disabled:opacity-50"
					>
						<Download class="h-3.5 w-3.5" />
						<span>{t.exportJsonBtn}</span>
					</button>
					<button
						type="button"
						disabled={anyBusy}
						onclick={() => void runNativePortable('preview')}
						class="flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
					>
						<Upload class="h-3.5 w-3.5" />
						<span>{t.importJsonBtn}</span>
					</button>
				{:else}
					<input
						bind:this={portableFileEl}
						type="file"
						accept=".json"
						onchange={handleDemoPortableFile}
						class="hidden"
						aria-label="Import portable JSON file"
					/>
					<button
						type="button"
						onclick={exportDemoPortable}
						class="flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft"
					>
						<Download class="h-3.5 w-3.5" />
						<span>{t.exportJsonBtn}</span>
					</button>
					<button
						type="button"
						onclick={() => portableFileEl?.click()}
						class="flex min-h-[36px] shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-separator bg-field px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft"
					>
						<Upload class="h-3.5 w-3.5" />
						<span>{t.importJsonBtn}</span>
					</button>
					<button
						type="button"
						onclick={openDemoSamplePreview}
						class="min-h-[36px] shrink-0 whitespace-nowrap rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95"
					>
						{t.previewDiffBtn}
					</button>
				{/if}
			</div>
		</div>

		{#if portableStatus}
			<p
				role={portableError ? 'alert' : 'status'}
				class="text-xs {portableError ? 'text-status-danger' : 'text-status-healthy'}"
			>
				{portableStatus}
			</p>
		{/if}
	</div>
</section>

<WdgImportPreviewDialog
	bind:open={importModalOpen}
	preview={useNativeFlow ? null : demoImportPreview}
	onResolveConflict={resolveDemoConflict}
	onCommit={commitDemoImport}
	{nativePreview}
	{accounts}
	destinations={nativeDestinations}
	mapping={nativeMapping}
	consent={nativeConsent}
	busy={portableBusy}
	complete={nativeComplete}
	onMappingChange={(next) => {
		nativeMapping = next;
		guards.portable.markDirty(true);
	}}
	onConsentChange={(next) => (nativeConsent = next)}
	onCommitNative={() => void runNativePortable('import')}
	onCancel={() => {
		if (useNativeFlow) resetNativePreviewState();
	}}
/>
