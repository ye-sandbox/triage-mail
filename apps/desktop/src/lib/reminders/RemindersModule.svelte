<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import Icon from '../inbox/Icon.svelte';
	import type { SettingsFormGuard } from '../settings/guards';
	import type { AccountConfig } from '../onboarding/types';
	import type { Destination, AlertsTransport } from '../alerts/types';
	import { nativeAlertsTransport } from '../alerts/transport';
	import { labels } from './labels';
	import {
		newReminderConfig,
		type RemindersTransport,
		type ScheduledReminderConfig
	} from './types';
	import { nativeRemindersTransport } from './transport';

	let {
		guard,
		transport = nativeRemindersTransport,
		alertsTransport = nativeAlertsTransport,
		accounts = []
	}: {
		guard: SettingsFormGuard;
		transport?: RemindersTransport;
		alertsTransport?: AlertsTransport;
		accounts?: AccountConfig[];
	} = $props();

	const t = $derived(labels[i18n.locale]);

	let configs = $state<ScheduledReminderConfig[]>([]);
	let destinations = $state<Destination[]>([]);
	let draft = $state<ScheduledReminderConfig | null>(null);
	let triggerKind = $state<'DueDate' | 'UnreadInterval'>('DueDate');
	let daysBefore = $state(2);
	let targetHour = $state(9);
	let intervalHours = $state(24);
	let minAmountStr = $state('');
	let selectedDestinations = $state<string[]>([]);
	let selectedAccount = $state<string>('All');

	let busy = $state(false);
	let error = $state('');
	let active = true;

	async function load() {
		try {
			const [cfgList, destList] = await Promise.all([
				transport.listConfigs(),
				alertsTransport.list()
			]);
			if (active) {
				configs = cfgList;
				destinations = destList;
			}
		} catch {
			if (active) error = t.error;
		}
	}

	$effect(() => {
		untrack(() => void load());
	});

	$effect(() => () => {
		active = false;
	});

	function edit(config: ScheduledReminderConfig | null) {
		error = '';
		if (!config) {
			draft = null;
			guard.markDirty(false);
			return;
		}

		draft = structuredClone(config);
		selectedAccount = draft.account_id ?? 'All';
		selectedDestinations = [...draft.destination_ids];

		if ('DueDate' in draft.trigger) {
			triggerKind = 'DueDate';
			daysBefore = draft.trigger.DueDate.days_before;
			targetHour = draft.trigger.DueDate.target_hour;
			minAmountStr = draft.trigger.DueDate.min_amount_cents
				? (draft.trigger.DueDate.min_amount_cents / 100).toFixed(2)
				: '';
		} else {
			triggerKind = 'UnreadInterval';
			intervalHours = draft.trigger.UnreadInterval.interval_hours;
			minAmountStr = draft.trigger.UnreadInterval.min_amount_cents
				? (draft.trigger.UnreadInterval.min_amount_cents / 100).toFixed(2)
				: '';
		}

		guard.markDirty(true);
	}

	function startCreate() {
		const fresh = newReminderConfig();
		if (destinations.length > 0) {
			fresh.destination_ids = [destinations[0].id];
		}
		edit(fresh);
	}

	function parseMinAmountCents(val: string): number | undefined {
		const clean = val.replace(',', '.').trim();
		if (!clean) return undefined;
		const parsed = parseFloat(clean);
		if (isNaN(parsed) || parsed <= 0) return undefined;
		return Math.round(parsed * 100);
	}

	async function saveDraft() {
		if (!draft || busy) return;

		const minAmt = parseMinAmountCents(minAmountStr);

		const updated: ScheduledReminderConfig = {
			...draft,
			name: draft.name.trim(),
			account_id: selectedAccount === 'All' ? undefined : selectedAccount,
			destination_ids: selectedDestinations,
			trigger:
				triggerKind === 'DueDate'
					? {
							DueDate: {
								days_before: Number(daysBefore) || 0,
								target_hour: Number(targetHour) || 0,
								min_amount_cents: minAmt
							}
						}
					: {
							UnreadInterval: {
								interval_hours: Math.max(1, Number(intervalHours) || 1),
								min_amount_cents: minAmt
							}
						}
		};

		if (!updated.name) {
			error = t.ruleNamePlaceholder;
			return;
		}

		if (updated.destination_ids.length === 0) {
			error = t.noDestinations;
			return;
		}

		busy = true;
		guard.setBusy(true);
		error = '';

		try {
			await transport.saveConfig(updated);
			guard.markDirty(false);
			draft = null;
			await load();
		} catch {
			if (active) error = t.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}

	async function remove(id: string, revision: number) {
		if (busy) return;
		busy = true;
		guard.setBusy(true);
		error = '';
		try {
			await transport.deleteConfig(id, revision);
			if (draft?.id === id) {
				draft = null;
				guard.markDirty(false);
			}
			await load();
		} catch {
			if (active) error = t.error;
		} finally {
			if (active) {
				busy = false;
				guard.setBusy(false);
			}
		}
	}

	async function toggleEnabled(cfg: ScheduledReminderConfig) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			await transport.saveConfig({
				...cfg,
				enabled: !cfg.enabled
			});
			await load();
		} catch {
			if (active) error = t.error;
		} finally {
			if (active) busy = false;
		}
	}

	function toggleDestination(id: string) {
		if (selectedDestinations.includes(id)) {
			selectedDestinations = selectedDestinations.filter((d) => d !== id);
		} else {
			selectedDestinations = [...selectedDestinations, id];
		}
	}

	function formatMinAmountDisplay(cents?: number): string {
		if (!cents) return '';
		return new Intl.NumberFormat(i18n.locale, {
			style: 'currency',
			currency: 'BRL'
		}).format(cents / 100);
	}
</script>

<div class="space-y-6">
	<!-- Title and Notice -->
	<div>
		<h2 class="text-base font-semibold text-[var(--text)]">{t.rulesTitle}</h2>
		<p class="text-xs text-[var(--secondary)] mt-1">{t.rulesNotice}</p>
	</div>

	{#if error}
		<div
			class="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-2.5 text-xs text-red-600 dark:text-red-400"
			role="alert"
		>
			{error}
		</div>
	{/if}

	{#if draft}
		<!-- Editor Form -->
		<form
			onsubmit={(e) => {
				e.preventDefault();
				void saveDraft();
			}}
			class="space-y-4 rounded-xl border border-[var(--separator)] bg-[var(--surface)] p-5"
		>
			<div class="flex items-center justify-between border-b border-[var(--separator)] pb-3">
				<h3 class="text-sm font-semibold text-[var(--text)]">
					{draft.revision > 0 ? t.editRule : t.addRule}
				</h3>
				<span class="text-xs text-[var(--secondary)]">
					Rev. {draft.revision}
				</span>
			</div>

			<!-- Rule Name -->
			<div class="space-y-1">
				<label for="reminder-name" class="block text-xs font-medium text-[var(--text)]">
					{t.ruleName}
				</label>
				<input
					id="reminder-name"
					type="text"
					bind:value={draft.name}
					placeholder={t.ruleNamePlaceholder}
					class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] placeholder:text-[var(--secondary)] focus:outline-hidden"
					required
				/>
			</div>

			<!-- Trigger Type Selector -->
			<div class="space-y-1">
				<label for="reminder-trigger-type" class="block text-xs font-medium text-[var(--text)]">
					{t.triggerType}
				</label>
				<select
					id="reminder-trigger-type"
					bind:value={triggerKind}
					class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] focus:outline-hidden"
				>
					<option value="DueDate">{t.triggerDueDate}</option>
					<option value="UnreadInterval">{t.triggerUnread}</option>
				</select>
			</div>

			<!-- Trigger-specific fields -->
			{#if triggerKind === 'DueDate'}
				<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
					<div class="space-y-1">
						<label for="reminder-days-before" class="block text-xs font-medium text-[var(--text)]">
							{t.daysBefore}
						</label>
						<input
							id="reminder-days-before"
							type="number"
							min="0"
							max="365"
							bind:value={daysBefore}
							class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] focus:outline-hidden"
						/>
					</div>
					<div class="space-y-1">
						<label for="reminder-target-hour" class="block text-xs font-medium text-[var(--text)]">
							{t.targetHour}
						</label>
						<input
							id="reminder-target-hour"
							type="number"
							min="0"
							max="23"
							bind:value={targetHour}
							class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] focus:outline-hidden"
						/>
					</div>
				</div>
			{:else}
				<div class="space-y-1">
					<label for="reminder-unread-hours" class="block text-xs font-medium text-[var(--text)]">
						{t.unreadHours}
					</label>
					<input
						id="reminder-unread-hours"
						type="number"
						min="1"
						max="8760"
						bind:value={intervalHours}
						class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] focus:outline-hidden"
					/>
				</div>
			{/if}

			<!-- Optional Min Amount Threshold -->
			<div class="space-y-1">
				<label for="reminder-min-amount" class="block text-xs font-medium text-[var(--text)]">
					{t.minAmount}
				</label>
				<input
					id="reminder-min-amount"
					type="text"
					bind:value={minAmountStr}
					placeholder={t.minAmountPlaceholder}
					class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] placeholder:text-[var(--secondary)] focus:outline-hidden"
				/>
			</div>

			<!-- Account Scope -->
			{#if accounts.length > 1}
				<div class="space-y-1">
					<label for="reminder-account" class="block text-xs font-medium text-[var(--text)]">
						{t.accountScope}
					</label>
					<select
						id="reminder-account"
						bind:value={selectedAccount}
						class="w-full rounded-lg border border-[var(--separator)] bg-[var(--reader)] px-3 py-2 text-xs text-[var(--text)] focus:outline-hidden"
					>
						<option value="All">{t.allAccounts}</option>
						{#each accounts as acc}
							<option value={acc.account_id}>{acc.email_display}</option>
						{/each}
					</select>
				</div>
			{/if}

			<!-- Alert Destinations Selection -->
			<div class="space-y-2">
				<span class="block text-xs font-medium text-[var(--text)]">
					{t.destinations}
				</span>
				{#if destinations.length === 0}
					<p class="text-xs text-[var(--secondary)] italic">
						{t.noDestinations}
					</p>
				{:else}
					<div class="space-y-1.5 rounded-lg border border-[var(--separator)] bg-[var(--reader)] p-3">
						{#each destinations as dest (dest.id)}
							<label class="flex items-center gap-2 text-xs text-[var(--text)] cursor-pointer">
								<input
									type="checkbox"
									checked={selectedDestinations.includes(dest.id)}
									onchange={() => toggleDestination(dest.id)}
									class="rounded border-[var(--separator)] text-[var(--accent)]"
								/>
								<span class="font-medium">{dest.name}</span>
								<span class="text-[var(--secondary)]">({dest.kind})</span>
							</label>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Form Actions -->
			<div class="flex items-center justify-end gap-2 pt-2 border-t border-[var(--separator)]">
				<button
					type="button"
					onclick={() => edit(null)}
					class="rounded-lg border border-[var(--separator)] px-3 py-1.5 text-xs text-[var(--secondary)] hover:bg-[var(--reader)]"
					disabled={busy}
				>
					{t.cancel}
				</button>
				<button
					type="submit"
					class="rounded-lg bg-[var(--text)] text-[var(--reader)] px-4 py-1.5 text-xs font-medium hover:opacity-90 disabled:opacity-50"
					disabled={busy || selectedDestinations.length === 0}
				>
					{t.save}
				</button>
			</div>
		</form>
	{:else}
		<!-- Configs List -->
		<div class="space-y-3">
			<div class="flex justify-end">
				<button
					type="button"
					onclick={startCreate}
					class="flex items-center gap-1.5 rounded-lg bg-[var(--text)] text-[var(--reader)] px-3 py-1.5 text-xs font-medium hover:opacity-90 transition-opacity"
				>
					+ {t.addRule}
				</button>
			</div>

			{#if configs.length === 0}
				<div
					class="flex flex-col items-center justify-center rounded-xl border border-dashed border-[var(--separator)] py-10 text-center"
				>
					<div class="text-[var(--secondary)] mb-2">
						<Icon name="bell" size={28} />
					</div>
					<p class="text-xs text-[var(--secondary)]">{t.emptyRules}</p>
				</div>
			{:else}
				<div class="space-y-2">
					{#each configs as cfg (cfg.id)}
						<div
							class="flex items-center justify-between gap-3 rounded-xl border border-[var(--separator)] bg-[var(--surface)] p-3.5 transition-colors"
						>
							<div class="flex items-center gap-3 min-w-0">
								<!-- Enable Toggle -->
								<button
									type="button"
									onclick={() => toggleEnabled(cfg)}
									aria-label={cfg.enabled ? t.enabled : t.disabled}
									class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-hidden"
									class:bg-emerald-600={cfg.enabled}
									class:bg-zinc-400={!cfg.enabled}
								>
									<span
										class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out"
										class:translate-x-4={cfg.enabled}
										class:translate-x-0={!cfg.enabled}
									></span>
								</button>

								<!-- Rule Summary -->
								<div class="min-w-0 flex flex-col">
									<div class="flex items-center gap-2">
										<span class="text-xs font-semibold text-[var(--text)] truncate">
											{cfg.name}
										</span>
										{#if cfg.account_id}
											<span class="rounded bg-[var(--reader)] px-1.5 py-0.5 text-[10px] text-[var(--secondary)]">
												Account
											</span>
										{/if}
									</div>
									<div class="flex items-center gap-1.5 text-[11px] text-[var(--secondary)] mt-0.5">
										{#if 'DueDate' in cfg.trigger}
											<span>
												{t.ruleSummaryDueDate(
													cfg.trigger.DueDate.days_before,
													cfg.trigger.DueDate.target_hour
												)}
											</span>
											{#if cfg.trigger.DueDate.min_amount_cents}
												<span>•</span>
												<span>
													{t.minAmountSummary(
														formatMinAmountDisplay(cfg.trigger.DueDate.min_amount_cents)
													)}
												</span>
											{/if}
										{:else}
											<span>
												{t.ruleSummaryUnread(cfg.trigger.UnreadInterval.interval_hours)}
											</span>
											{#if cfg.trigger.UnreadInterval.min_amount_cents}
												<span>•</span>
												<span>
													{t.minAmountSummary(
														formatMinAmountDisplay(cfg.trigger.UnreadInterval.min_amount_cents)
													)}
												</span>
											{/if}
										{/if}
									</div>
								</div>
							</div>

							<!-- Actions -->
							<div class="flex items-center gap-2 shrink-0">
								<button
									type="button"
									onclick={() => edit(cfg)}
									class="rounded-lg border border-[var(--separator)] px-2.5 py-1 text-xs text-[var(--text)] hover:bg-[var(--reader)]"
								>
									{t.edit}
								</button>
								<button
									type="button"
									onclick={() => remove(cfg.id, cfg.revision)}
									class="rounded-lg px-2.5 py-1 text-xs text-red-500 hover:bg-red-500/10"
								>
									{t.delete}
								</button>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
