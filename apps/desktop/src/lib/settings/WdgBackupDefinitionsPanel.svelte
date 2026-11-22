<script lang="ts">
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { transferGuards } from '$lib/backup/guards';
	import { transfer, type Definitions, type Mapping, type TransferTransport } from '$lib/backup/transport';
	import { i18n } from '$lib/i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { nativeAccountTransport } from '$lib/onboarding/transport';
	import type { AccountConfig, AccountTransport } from '$lib/onboarding/types';
	import { exportDemoPortableDefinitions, parseDemoPortableFile, runPortableAction } from './backup-portable';
	import { SettingsFormGuard } from './guards';
	import { settingsLabels } from './labels';
	import { PORTABLE_IMPORT_PREVIEW_FIXTURE, type PortableImportPreview } from './settings.fixtures';
	import WdgBackupExportCard from './WdgBackupExportCard.svelte';
	import WdgBackupRestoreCard from './WdgBackupRestoreCard.svelte';
	import WdgImportPreviewDialog from './WdgImportPreviewDialog.svelte';
	import WdgLocalArchiveImportCard from './WdgLocalArchiveImportCard.svelte';
	import WdgPortableDefinitionsCard from './WdgPortableDefinitionsCard.svelte';

	type ArchiveImportTransport = Pick<AccountTransport, 'chooseArchiveFile' | 'importLocalArchive'>;

	interface Props {
		transport?: TransferTransport;
		archiveTransport?: ArchiveImportTransport;
		accounts?: AccountConfig[];
		guard?: SettingsFormGuard;
		restore?: boolean;
		ondone?: () => void;
		onArchiveImported?: () => void;
	}

	let {
		transport: transportProp,
		archiveTransport: archiveTransportProp,
		accounts = [],
		guard = new SettingsFormGuard(),
		restore = false,
		ondone = () => {},
		onArchiveImported
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].backup);
	const useNativeFlow = $derived(Boolean(transportProp) || nativeAvailable());
	const activeTransport = $derived(transportProp ?? transfer);
	const effectiveArchiveTransport = $derived(
		archiveTransportProp ?? (nativeAvailable() ? nativeAccountTransport : undefined)
	);
	const guards = transferGuards(untrack(() => guard));

	let backupBusy = $state(false);
	let restoreBusy = $state(false);
	let recoveryAvailable = $state(false);
	let archiveBusy = $state(false);
	let nativePreview = $state<Definitions | null>(null);
	let nativeMapping = $state<Mapping>({ accounts: {}, destinations: {} });
	let nativeDestinations = $state<{ id: string; name: string }[]>([]);
	let nativePath = '';
	let nativeConsent = $state(false);
	let portableBusy = $state(false);
	let portableStatus = $state('');
	let portableError = $state(false);
	let importModalOpen = $state(false);
	let demoImportPreview = $state<PortableImportPreview>(structuredClone(PORTABLE_IMPORT_PREVIEW_FIXTURE));
	let active = true;

	const anyBusy = $derived(backupBusy || restoreBusy || portableBusy || archiveBusy || guard.isBlocked());
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
			void activeTransport.available().then((ok) => active && (recoveryAvailable = ok)).catch(() => active && (recoveryAvailable = false));
		} else if (restore) {
			recoveryAvailable = true;
		}
		return () => {
			active = false;
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

	async function runNativePortable(action: 'export' | 'preview' | 'import') {
		if (anyBusy) return;
		portableBusy = true;
		portableError = false;
		portableStatus = '';
		guards.portable.setBusy(true);
		try {
			const res = await runPortableAction(action, activeTransport, {
				nativePreview: nativePreview ? $state.snapshot(nativePreview) : null,
				nativeMapping: $state.snapshot(nativeMapping),
				nativePath,
				nativeConsent,
				nativeComplete
			});
			if (!active) return;
			if (res.kind === 'imported') {
				resetNativePreviewState();
				portableStatus = t.done;
				toast.success(t.done);
			} else if (res.kind === 'exported') {
				portableStatus = t.done;
				toast.success(t.done);
			} else if (res.kind === 'preview') {
				nativePreview = res.value;
				nativeDestinations = res.targets;
				nativePath = res.path;
				nativeMapping = { accounts: {}, destinations: {} };
				nativeConsent = false;
				guards.portable.markDirty(true);
				importModalOpen = true;
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
</script>

<section class="flex flex-col gap-5" aria-labelledby="backup-heading">
	<div class="border-b border-separator pb-3">
		<h2 id="backup-heading" class="font-display text-base font-semibold text-foreground">{t.heading}</h2>
		<p class="mt-1 text-xs text-secondary">{t.subtitle}</p>
	</div>

	<div class="space-y-2 rounded-lg border border-separator bg-field p-4 text-xs leading-relaxed text-secondary">
		<p>{t.help}</p>
		<p>{t.newFile}</p>
	</div>

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
		{#if !restore}
			<WdgBackupExportCard
				{useNativeFlow}
				{activeTransport}
				backupGuard={guards.backup}
				{anyBusy}
				busy={backupBusy}
				onBusyChange={(v) => (backupBusy = v)}
			/>
		{/if}
		<WdgBackupRestoreCard
			{useNativeFlow}
			{restore}
			{recoveryAvailable}
			{activeTransport}
			backupGuard={guards.backup}
			{anyBusy}
			busy={restoreBusy}
			onBusyChange={(v) => (restoreBusy = v)}
			{ondone}
		/>
	</div>

	<WdgPortableDefinitionsCard
		{useNativeFlow}
		{anyBusy}
		{portableStatus}
		{portableError}
		onNativeExport={() => runNativePortable('export')}
		onNativePreview={() => runNativePortable('preview')}
		onDemoExport={() => exportDemoPortableDefinitions(t.done)}
		onDemoFile={async (e) => {
			const parsed = await parseDemoPortableFile(e, t.error);
			if (parsed) {
				demoImportPreview = parsed;
				importModalOpen = true;
			}
		}}
		onDemoSamplePreview={() => {
			demoImportPreview = structuredClone(PORTABLE_IMPORT_PREVIEW_FIXTURE);
			importModalOpen = true;
		}}
	/>

	{#if !restore}
		<WdgLocalArchiveImportCard
			archiveTransport={effectiveArchiveTransport}
			{anyBusy}
			{archiveBusy}
			onArchiveBusyChange={(v) => (archiveBusy = v)}
			{onArchiveImported}
		/>
	{/if}
</section>

<WdgImportPreviewDialog
	bind:open={importModalOpen}
	preview={useNativeFlow ? null : demoImportPreview}
	onResolveConflict={(id, resolution) =>
		(demoImportPreview = {
			...demoImportPreview,
			items: demoImportPreview.items.map((item) => (item.id === id ? { ...item, resolution } : item))
		})}
	onCommit={() => {
		importModalOpen = false;
		const { added, updated, conflicting } = demoImportPreview.counts;
		toast.success(`Imported ${added} added, ${updated} updated, ${conflicting} resolved definitions`);
	}}
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
	onCancel={() => useNativeFlow && resetNativePreviewState()}
/>
