<script lang="ts">
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import Upload from '@lucide/svelte/icons/upload';
	import { toast } from 'svelte-sonner';
	import { i18n } from '$lib/i18n/index.svelte';
	import { nativeAvailable } from '$lib/inbox/transport';
	import { onboardingSession } from '$lib/onboarding/session.svelte';
	import type { AccountTransport, LocalArchiveFormat } from '$lib/onboarding/types';
	import { settingsLabels } from './labels';

	type ArchiveImportTransport = Pick<AccountTransport, 'chooseArchiveFile' | 'importLocalArchive'>;

	interface Props {
		archiveTransport?: ArchiveImportTransport;
		anyBusy: boolean;
		archiveBusy: boolean;
		onArchiveBusyChange: (busy: boolean) => void;
		onArchiveImported?: () => void;
	}

	let {
		archiveTransport,
		anyBusy,
		archiveBusy,
		onArchiveBusyChange,
		onArchiveImported
	}: Props = $props();

	const t = $derived(settingsLabels[i18n.locale].backup);

	let archiveFileEl = $state<HTMLInputElement>();
	let archiveEmail = $state('');
	let archiveFormat = $state<LocalArchiveFormat>('Mbox');
	let archivePath = $state('');
	let archiveCount = $state(0);
	let archiveStatus = $state('');
	let archiveError = $state(false);
	let active = true;

	$effect(() => {
		active = true;
		return () => {
			active = false;
		};
	});

	function inferArchiveFormat(pathOrName: string): LocalArchiveFormat {
		return pathOrName.trim().toLowerCase().endsWith('.eml') ? 'Eml' : 'Mbox';
	}

	async function chooseLocalArchiveFile() {
		if (!archiveTransport?.chooseArchiveFile || anyBusy) return;
		const selected = await archiveTransport.chooseArchiveFile();
		if (active && selected) {
			archivePath = selected;
			archiveFormat = inferArchiveFormat(selected);
		}
	}

	function handleDemoArchiveFile(event: Event) {
		const file =
			event.currentTarget instanceof HTMLInputElement ? event.currentTarget.files?.[0] : null;
		if (archiveFileEl) archiveFileEl.value = '';
		if (!file) return;
		archivePath = file.name;
		archiveFormat = inferArchiveFormat(file.name);
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		const cleanEmail = archiveEmail.trim();
		if (!cleanEmail || !archivePath || anyBusy) return;

		if (!archiveTransport?.importLocalArchive) {
			archiveError = false;
			archiveStatus = `${t.archiveSuccess} ${archivePath}`;
			toast.success(archiveStatus);
			archivePath = '';
			archiveEmail = '';
			return;
		}

		onArchiveBusyChange(true);
		archiveError = false;
		archiveCount = 0;
		archiveStatus = '';
		try {
			const res = await archiveTransport.importLocalArchive(
				archivePath,
				cleanEmail,
				archiveFormat,
				(evt) => {
					if (active && evt.new_messages_count > 0) {
						archiveCount += evt.new_messages_count;
					}
				}
			);
			if (!active) return;
			archiveStatus = `${t.archiveSuccess} ${res.imported_count}`;
			archivePath = '';
			archiveEmail = '';
			toast.success(archiveStatus);
			if (nativeAvailable()) void onboardingSession.accounts.load(true);
			onArchiveImported?.();
		} catch (err: unknown) {
			if (!active) return;
			archiveError = true;
			archiveStatus = err === 'AccountExists' ? t.archiveConflict : t.archiveError;
			toast.error(archiveStatus);
		} finally {
			if (active) onArchiveBusyChange(false);
		}
	}
</script>

<form
	onsubmit={(e) => void handleSubmit(e)}
	aria-label={t.archiveTitle}
	autocomplete="off"
	class="flex flex-col gap-3 rounded-lg border border-separator bg-surface p-4"
>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="max-w-xl space-y-1">
			<div class="flex items-center gap-2 text-sm font-medium text-foreground">
				<FileArchive class="h-4 w-4 text-secondary" aria-hidden="true" />
				<span>{t.archiveTitle}</span>
			</div>
			<p class="text-xs text-secondary">{t.archiveDesc}</p>
		</div>
	</div>

	<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
		<label class="flex flex-col gap-1 text-xs text-foreground">
			<span>{t.archiveEmailLabel}</span>
			<input
				name="archiveEmail"
				type="text"
				required
				maxlength="320"
				disabled={anyBusy}
				bind:value={archiveEmail}
				placeholder="archive@example.com"
				class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none disabled:opacity-50"
			/>
		</label>

		<div class="flex flex-col gap-1 text-xs text-foreground">
			<span>{t.archiveChooseBtn}</span>
			<div class="flex items-center gap-2">
				{#if !archiveTransport}
					<input
						bind:this={archiveFileEl}
						type="file"
						accept=".mbox,.eml,message/rfc822"
						onchange={handleDemoArchiveFile}
						class="hidden"
						aria-label={t.archiveChooseBtn}
					/>
				{/if}
				<button
					type="button"
					disabled={anyBusy}
					onclick={() => (archiveTransport ? void chooseLocalArchiveFile() : archiveFileEl?.click())}
					class="flex h-9 min-w-0 flex-1 items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 text-xs text-foreground hover:bg-selection-soft disabled:opacity-50"
				>
					<span class="truncate font-mono">{archivePath || t.archiveNoFile}</span>
					<Upload class="h-3.5 w-3.5 shrink-0 text-secondary" aria-hidden="true" />
				</button>
			</div>
		</div>
	</div>

	<div class="flex flex-wrap items-center justify-between gap-3">
		<div role="radiogroup" aria-label={t.archiveTitle} class="flex flex-wrap items-center gap-2">
			<button
				type="button"
				role="radio"
				aria-checked={archiveFormat === 'Mbox'}
				disabled={anyBusy}
				onclick={() => (archiveFormat = 'Mbox')}
				class="rounded-md border px-3 py-1 text-xs font-medium transition-colors {archiveFormat === 'Mbox'
					? 'border-accent bg-selection-soft text-accent'
					: 'border-separator bg-field text-secondary hover:text-foreground'}"
			>
				{t.archiveFormatMbox}
			</button>
			<button
				type="button"
				role="radio"
				aria-checked={archiveFormat === 'Eml'}
				disabled={anyBusy}
				onclick={() => (archiveFormat = 'Eml')}
				class="rounded-md border px-3 py-1 text-xs font-medium transition-colors {archiveFormat === 'Eml'
					? 'border-accent bg-selection-soft text-accent'
					: 'border-separator bg-field text-secondary hover:text-foreground'}"
			>
				{t.archiveFormatEml}
			</button>
		</div>

		<button
			type="submit"
			disabled={anyBusy || !archivePath || !archiveEmail.trim()}
			class="flex min-h-[36px] shrink-0 items-center gap-2 whitespace-nowrap rounded-md bg-accent px-4 py-2 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-50"
		>
			<Upload class="h-3.5 w-3.5" aria-hidden="true" />
			<span>{archiveBusy ? `${t.busy} (${archiveCount})` : t.archiveImportBtn}</span>
		</button>
	</div>

	{#if archiveStatus}
		<p role={archiveError ? 'alert' : 'status'} class="text-xs {archiveError ? 'text-status-danger' : 'text-status-healthy'}">
			{archiveStatus}
		</p>
	{/if}
</form>
