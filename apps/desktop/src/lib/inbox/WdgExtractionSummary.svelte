<script lang="ts">
	import Banknote from '@lucide/svelte/icons/banknote';
	import Copy from '@lucide/svelte/icons/copy';
	import FileCheck2 from '@lucide/svelte/icons/file-check-2';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import MailMinus from '@lucide/svelte/icons/mail-minus';
	import Package from '@lucide/svelte/icons/package';
	import QrCode from '@lucide/svelte/icons/qr-code';
	import { formatBrl, t } from '$lib/i18n';
	import { appStore } from '$lib/stores/app.svelte';
	import type { TriageBadge } from '../reader/types';

	type Props = {
		badges: TriageBadge[];
		onCopy?: (value: string, label: string) => void;
	};

	let { badges, onCopy }: Props = $props();

	function copy(value: string, label: string) {
		onCopy?.(value, label);
	}
</script>

{#if badges.length > 0}
	<section
		aria-label={t(appStore.locale, 'extractedArtifacts')}
		class="mx-4 mt-3 rounded-lg border border-separator bg-surface p-3"
	>
		<div class="mb-2 flex items-center justify-between">
			<h2 class="font-display text-xs font-semibold text-foreground">
				{t(appStore.locale, 'extractedArtifacts')}
			</h2>
			<span class="text-[11px] text-secondary">Local deterministic parser</span>
		</div>

		<div class="flex flex-col gap-2 text-xs">
			{#each badges as badge, index (index)}
				{#if 'OtpCode' in badge}
					<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2">
						<div class="flex items-center gap-2">
							<KeyRound class="h-4 w-4 shrink-0 text-status-info" aria-hidden="true" />
							<span class="text-secondary">OTP Code</span>
							<span class="font-mono text-sm font-semibold tabular-nums text-foreground">
								{badge.OtpCode.code}
							</span>
						</div>
						<button
							type="button"
							onclick={() => copy(badge.OtpCode.code, 'OTP Code')}
							class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
						>
							<Copy class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t(appStore.locale, 'copyCode')}</span>
						</button>
					</div>
				{:else if 'FebrabanBoleto' in badge}
					{@const boleto = badge.FebrabanBoleto}
					<div class="flex flex-col gap-2 rounded-md border border-separator bg-field px-3 py-2">
						<div class="flex flex-wrap items-center justify-between gap-2">
							<div class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
								<span class="inline-flex items-center gap-2">
									<Banknote class="h-4 w-4 shrink-0 text-status-warning" aria-hidden="true" />
									<span class="font-medium text-foreground">FEBRABAN Boleto</span>
								</span>
								{#if boleto.amount_cents !== null}
									<span class="font-mono font-semibold whitespace-nowrap tabular-nums text-foreground">
										{formatBrl(boleto.amount_cents, appStore.locale)}
									</span>
								{/if}
								{#if boleto.due_date_iso}
									<span class="font-mono whitespace-nowrap tabular-nums text-status-warning">
										Due {boleto.due_date_iso}
									</span>
								{/if}
							</div>
							<button
								type="button"
								onclick={() => copy(boleto.digitable_line, 'Digitable line')}
								class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
							>
								<Copy class="h-3.5 w-3.5" aria-hidden="true" />
								<span>{t(appStore.locale, 'copyLine')}</span>
							</button>
						</div>
						<div class="truncate font-mono text-[11px] tabular-nums text-secondary">
							{boleto.digitable_line}
						</div>
					</div>
				{:else if 'PixPayload' in badge}
					<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2">
						<div class="flex min-w-0 items-center gap-2">
							<QrCode class="h-4 w-4 shrink-0 text-status-healthy" aria-hidden="true" />
							<span class="font-medium text-foreground">Pix Copia e Cola</span>
							<span class="max-w-[260px] truncate font-mono text-[11px] text-secondary">
								{badge.PixPayload.payload}
							</span>
						</div>
						<button
							type="button"
							onclick={() => copy(badge.PixPayload.payload, 'Pix Payload')}
							class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
						>
							<Copy class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t(appStore.locale, 'copyPix')}</span>
						</button>
					</div>
				{:else if 'TrackingCode' in badge}
					<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2">
						<div class="flex items-center gap-2">
							<Package class="h-4 w-4 shrink-0 text-status-info" aria-hidden="true" />
							<span class="font-medium text-foreground">{badge.TrackingCode.carrier}</span>
							<span class="font-mono tabular-nums text-foreground">{badge.TrackingCode.code}</span>
						</div>
						<button
							type="button"
							onclick={() => copy(badge.TrackingCode.code, 'Tracking code')}
							class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
						>
							<Copy class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t(appStore.locale, 'copyTracking')}</span>
						</button>
					</div>
				{:else if 'NfeInvoice' in badge}
					<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2">
						<div class="flex min-w-0 items-center gap-2">
							<FileCheck2 class="h-4 w-4 shrink-0 text-secondary" aria-hidden="true" />
							<span class="font-medium text-foreground">NF-e</span>
							<span class="max-w-[240px] truncate font-mono text-[11px] tabular-nums text-secondary">
								{badge.NfeInvoice.access_key}
							</span>
						</div>
						<button
							type="button"
							onclick={() => copy(badge.NfeInvoice.access_key, 'NF-e 44-digit access key')}
							class="flex min-h-11 items-center gap-2 rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground transition-transform duration-100 ease-out hover:bg-selection-soft active:scale-[0.98] md:min-h-8"
						>
							<Copy class="h-3.5 w-3.5" aria-hidden="true" />
							<span>{t(appStore.locale, 'copyNfe')}</span>
						</button>
					</div>
				{:else if 'UnsubscribeAvailable' in badge}
					<div class="flex flex-wrap items-center justify-between gap-2 rounded-md border border-separator bg-field px-3 py-2">
						<div class="flex items-center gap-2">
							<MailMinus class="h-4 w-4 shrink-0 text-secondary" aria-hidden="true" />
							<span class="text-secondary">
								{badge.UnsubscribeAvailable.supports_one_click
									? 'RFC 8058 List-Unsubscribe-Post detected (available in Command Palette)'
									: 'One-click unsubscribe unavailable for this sender'}
							</span>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	</section>
{/if}
