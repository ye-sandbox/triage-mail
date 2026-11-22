<script lang="ts">
	import { untrack } from 'svelte';
	import Banknote from '@lucide/svelte/icons/banknote';
	import Copy from '@lucide/svelte/icons/copy';
	import FileCheck2 from '@lucide/svelte/icons/file-check-2';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import MailMinus from '@lucide/svelte/icons/mail-minus';
	import Package from '@lucide/svelte/icons/package';
	import QrCode from '@lucide/svelte/icons/qr-code';
	import { formatBrl } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import type { TriageBadge } from '../reader/types';
	import type { BadgeLoader } from './badges';
	import { badgeText } from './badges';

	type Props = {
		id: string;
		inlineBadges?: TriageBadge[];
		loader?: BadgeLoader;
		revision: number;
		onLoaded?: (id: string, badges: TriageBadge[]) => void;
		onCopy?: (value: string, label: string) => void;
	};

	let { id, inlineBadges, loader, revision, onLoaded, onCopy }: Props = $props();

	let loadedBadges = $state.raw<TriageBadge[]>([]);

	$effect(() => {
		if (inlineBadges && inlineBadges.length > 0) {
			loadedBadges = [];
			return;
		}
		if (!loader) {
			loadedBadges = [];
			return;
		}
		const targetId = id;
		const rev = revision;
		return untrack(() => {
			loadedBadges = [];
			return loader.watch(
				targetId,
				(next) => {
					loadedBadges = next;
					onLoaded?.(targetId, next);
				},
				rev
			);
		});
	});

	const badges = $derived(inlineBadges && inlineBadges.length > 0 ? inlineBadges : loadedBadges);
</script>

{#each badges as badge, index (`${id}-${index}`)}
	{@const text = badgeText(badge)}
	{#if 'OtpCode' in badge}
		<button
			type="button"
			aria-label={`${text.label}: ${text.value}`}
			onclick={(e) => {
				e.stopPropagation();
				onCopy?.(badge.OtpCode.code, 'OTP Code');
			}}
			class="inline-flex shrink-0 items-center gap-1 rounded border border-separator bg-field px-2 py-1 font-mono text-[10px] font-medium tabular-nums text-status-info hover:bg-surface"
		>
			<KeyRound class="h-2.5 w-2.5" aria-hidden="true" />
			<span>{badge.OtpCode.code}</span>
			<Copy class="h-2.5 w-2.5" aria-hidden="true" />
		</button>
	{:else if 'FebrabanBoleto' in badge}
		{@const boleto = badge.FebrabanBoleto}
		<span
			aria-label={`${text.label}: ${text.value}`}
			class="inline-flex shrink-0 items-center gap-1 rounded border border-separator bg-field px-2 py-1 font-mono text-[10px] font-medium tabular-nums text-status-warning"
		>
			<Banknote class="h-2.5 w-2.5" aria-hidden="true" />
			{#if boleto.amount_cents !== null}
				<span>{formatBrl(boleto.amount_cents, appStore.locale)}</span>
			{/if}
			{#if boleto.due_date_iso}
				{#if boleto.amount_cents !== null}<span>·</span>{/if}
				<span>{boleto.due_date_iso.slice(5)}</span>
			{/if}
		</span>
	{:else if 'PixPayload' in badge}
		<span
			aria-label={`${text.label}: ${text.value}`}
			class="inline-flex shrink-0 items-center gap-1 rounded border border-separator bg-field px-2 py-1 font-mono text-[10px] font-medium text-status-healthy"
		>
			<QrCode class="h-2.5 w-2.5" aria-hidden="true" />
			<span>Pix</span>
		</span>
	{:else if 'TrackingCode' in badge}
		<span
			aria-label={`${text.label}: ${text.value}`}
			class="inline-flex shrink-0 items-center gap-1 rounded border border-separator bg-field px-2 py-1 font-mono text-[10px] text-foreground"
		>
			<Package class="h-2.5 w-2.5" aria-hidden="true" />
			<span>{badge.TrackingCode.carrier} {badge.TrackingCode.code}</span>
		</span>
	{:else if 'NfeInvoice' in badge}
		<span
			aria-label={`${text.label}: ${text.value}`}
			class="inline-flex shrink-0 items-center gap-1 rounded border border-separator bg-field px-2 py-1 font-mono text-[10px] text-secondary"
		>
			<FileCheck2 class="h-2.5 w-2.5" aria-hidden="true" />
			<span>NF-e</span>
		</span>
	{:else if 'UnsubscribeAvailable' in badge && badge.UnsubscribeAvailable.supports_one_click}
		<span
			aria-label={`${text.label}: ${text.value}`}
			class="inline-flex shrink-0 items-center gap-1 rounded border border-separator bg-field px-2 py-1 text-[10px] text-secondary"
		>
			<MailMinus class="h-2.5 w-2.5" aria-hidden="true" />
			<span>RFC 8058</span>
		</span>
	{/if}
{/each}
