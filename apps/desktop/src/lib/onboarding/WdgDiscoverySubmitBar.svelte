<script lang="ts">
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import MailPlus from '@lucide/svelte/icons/mail-plus';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import { i18n } from '../i18n/index.svelte';

	interface Props {
		demoMode: boolean;
		isArchive: boolean;
		oauth: boolean;
		hasTestConnection: boolean;
		discoveryBusy: boolean;
		controllerBusy: boolean;
		controllerTesting: boolean;
		archivePath: string;
		onTestLogin: () => void;
	}

	let {
		demoMode,
		isArchive,
		oauth,
		hasTestConnection,
		discoveryBusy,
		controllerBusy,
		controllerTesting,
		archivePath,
		onTestLogin
	}: Props = $props();

	const t = $derived(i18n.t.accountForm);
</script>

<div class="flex flex-wrap items-center justify-between gap-3 border-t border-separator pt-4">
	<p class="text-xs text-secondary">
		{isArchive ? t.localArchiveNoticeFooter : oauth ? t.oauthVaultNotice : t.connectVaultNotice}
	</p>
	<div class="flex flex-wrap items-center gap-2">
		{#if !demoMode && !isArchive && !oauth && hasTestConnection}
			<button
				type="button"
				disabled={discoveryBusy || controllerBusy || controllerTesting}
				onclick={onTestLogin}
				class="flex min-h-[42px] shrink-0 items-center gap-2 rounded-md border border-separator bg-surface px-4 py-2 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field disabled:opacity-60 md:min-h-[38px]"
			>
				<ShieldCheck class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{controllerTesting ? t.testingLogin : t.testLogin}</span>
			</button>
		{/if}
		<button
			type="submit"
			disabled={discoveryBusy || controllerBusy || controllerTesting || (isArchive && !archivePath)}
			class="flex min-h-[42px] shrink-0 items-center gap-2 rounded-md bg-accent px-5 py-2 text-xs font-medium text-accent-foreground whitespace-nowrap hover:opacity-95 active:scale-[0.98] disabled:opacity-60 md:min-h-[38px]"
		>
			{#if isArchive}
				<FileArchive class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{controllerBusy ? t.localArchiveImporting : t.localArchiveSubmit}</span>
			{:else}
				<MailPlus class="h-3.5 w-3.5" aria-hidden="true" />
				<span>{controllerBusy ? t.connecting : oauth ? t.authorizeAndConnect : t.connectAccount}</span>
			{/if}
		</button>
	</div>
</div>
