<script lang="ts">
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Clock from '@lucide/svelte/icons/clock';
	import ImagePlus from '@lucide/svelte/icons/image-plus';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import { toast } from 'svelte-sonner';
	import { i18n } from '../i18n/index.svelte';
	import type { DemoAccountHealth, DemoConnectedAccountDto } from './onboarding.fixtures';
	import type { AccountConfig } from './types';

	interface Props {
		account: DemoConnectedAccountDto;
		expanded: boolean;
		authLabel: string;
		onToggleFolders: () => void;
		onUpdateAvatar?: (accountId: string, avatarUrl: string | null) => void;
		onRemove?: (accountId: string) => void;
	}

	let {
		account,
		expanded,
		authLabel,
		onToggleFolders,
		onUpdateAvatar,
		onRemove
	}: Props = $props();

	let fileInput = $state<HTMLInputElement | null>(null);

	const t = $derived(i18n.t.onboarding);
	const formT = $derived(i18n.t.accountForm);

	function attachInput() {
		return (node: HTMLInputElement) => {
			fileInput = node;
			return () => {
				if (fileInput === node) fileInput = null;
			};
		};
	}

	function initialsFor(email: string): string {
		const local = email.split('@')[0] ?? 'AC';
		const parts = local.split(/[._-]/).filter(Boolean);
		return parts.length >= 2
			? `${parts[0][0]}${parts[1][0]}`.toUpperCase()
			: local.slice(0, 2).toUpperCase();
	}

	function healthClass(health: DemoAccountHealth): string {
		if (health === 'Healthy') return 'text-status-healthy';
		if (health === 'Syncing') return 'text-status-info';
		if (health === 'Idle') return 'text-secondary';
		return 'text-status-warning';
	}

	function handleDemoAvatarSelect(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file || !file.type.startsWith('image/')) return;
		const reader = new FileReader();
		reader.onload = () => {
			if (typeof reader.result === 'string') {
				onUpdateAvatar?.(account.account_id, reader.result);
				toast.success(`Updated avatar for ${account.account_id}`);
			}
		};
		reader.readAsDataURL(file);
	}
</script>

<article class="rounded-lg border border-separator bg-surface">
	<div class="flex flex-wrap items-center justify-between gap-4 p-4">
		<div class="flex min-w-0 items-center gap-4">
			<div
				class="relative flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-md border border-separator bg-field font-display text-xs font-semibold text-foreground"
			>
				{#if account.avatar_url}
					<img
						src={account.avatar_url}
						alt={`Avatar for ${account.email_display}`}
						referrerpolicy="no-referrer"
						class="h-full w-full object-cover"
					/>
				{:else}
					<span>{initialsFor(account.email_display)}</span>
				{/if}
			</div>
			<div class="min-w-0 space-y-1">
				<div class="flex flex-wrap items-center gap-2">
					<span class="truncate text-sm font-semibold text-foreground">{account.email_display}</span>
					<span class="rounded border border-separator bg-field px-2 py-1 font-mono text-[11px] font-medium text-foreground">
						{authLabel}
					</span>
				</div>
				<div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-secondary">
					<span class="font-mono text-foreground">{account.account_id}</span>
					<span aria-hidden="true">·</span>
					{#if account.auth_method === 'LocalArchive'}
						<span>{formT.localArchiveBadge}</span>
					{:else}
						<span class="font-mono">{account.imap_host}:{account.imap_port}</span>
						<span aria-hidden="true">·</span>
						<span>{account.tls_mode}</span>
					{/if}
				</div>
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<span class="flex items-center gap-2 pr-1 text-xs font-medium whitespace-nowrap {healthClass(account.health)}">
				{#if account.health === 'Healthy'}
					<CheckCircle2 class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				{:else if account.health === 'Syncing'}
					<RefreshCw class="h-3.5 w-3.5 shrink-0 animate-spin" aria-hidden="true" />
				{:else if account.health === 'Idle'}
					<Clock class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				{:else}
					<AlertTriangle class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				{/if}
				<span>{account.health}</span>
			</span>

			<input
				{@attach attachInput()}
				type="file"
				accept="image/*"
				onchange={handleDemoAvatarSelect}
				class="hidden"
				aria-label={`Upload avatar for ${account.email_display}`}
			/>
			<button
				type="button"
				onclick={() => fileInput?.click()}
				class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-field px-3 py-1 text-xs text-foreground whitespace-nowrap hover:bg-selection-soft"
			>
				<ImagePlus class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				<span>{account.avatar_url ? i18n.t.accountPhoto.replace : i18n.t.accountPhoto.choose}</span>
			</button>
			{#if account.avatar_url}
				<button
					type="button"
					onclick={() => onUpdateAvatar?.(account.account_id, null)}
					class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs text-secondary whitespace-nowrap hover:text-status-danger"
				>
					<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
					<span>{i18n.t.accountPhoto.remove}</span>
				</button>
			{/if}

			<button
				type="button"
				onclick={onToggleFolders}
				aria-expanded={expanded}
				class="flex min-h-[36px] items-center gap-1 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground whitespace-nowrap hover:bg-field"
			>
				{#if expanded}
					<ChevronDown class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				{:else}
					<ChevronRight class="h-3.5 w-3.5 text-secondary" aria-hidden="true" />
				{/if}
				<span>
					{t.foldersCount} (<span class="font-mono tabular-nums">{account.folders.length}</span>)
				</span>
			</button>

			{#if onRemove}
				<button
					type="button"
					onclick={() => onRemove?.(account.account_id)}
					aria-label={`Disconnect ${account.email_display}`}
					class="flex min-h-[36px] min-w-[36px] items-center justify-center rounded-md border border-separator bg-surface text-secondary hover:text-status-danger"
				>
					<Trash2 class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
			{/if}
		</div>
	</div>

	{#if expanded}
		<div class="overflow-x-auto border-t border-separator bg-toolbar/50 px-4 py-3">
			<table class="w-full text-left text-xs">
				<thead>
					<tr class="border-b border-separator text-[11px] font-medium text-secondary">
						<th class="py-2 pr-4">{t.folderColumn}</th>
						<th class="py-2 pr-4">{t.roleColumn}</th>
						<th class="py-2 pr-4 text-right">UIDVALIDITY</th>
						<th class="py-2 pr-4 text-right">{t.totalColumn}</th>
						<th class="py-2 text-right">{t.unreadColumn}</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-separator">
					{#each account.folders as folder (folder.id)}
						<tr class="h-9 hover:bg-field/60">
							<td class="py-2 pr-4 font-medium text-foreground">{folder.name}</td>
							<td class="py-2 pr-4 font-mono text-[11px] text-secondary">{folder.role}</td>
							<td class="py-2 pr-4 text-right font-mono tabular-nums text-secondary">{folder.uid_validity}</td>
							<td class="py-2 pr-4 text-right font-mono tabular-nums text-foreground">
								{folder.total_messages.toLocaleString()}
							</td>
							<td
								class="py-2 text-right font-mono tabular-nums {folder.unread_messages > 0
									? 'font-semibold text-accent'
									: 'text-secondary'}"
							>
								{folder.unread_messages.toLocaleString()}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</article>
