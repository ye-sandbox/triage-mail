<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { Dialog, Tabs } from 'bits-ui';
	import { Clock, Lock, ShieldCheck, X } from 'lucide-svelte';
	import { toast } from 'svelte-sonner';
	import { labels as alertLabelsMap } from '$lib/alerts/labels';
	import {
		minutes,
		scheduleLabels,
		time,
		type SchedulePolicy,
		type SchedulePreview,
		type ScheduleState,
		type ScheduleTransport
	} from '$lib/alerts/schedule';
	import {
		EXPORT_FIELDS,
		newDestination,
		type AlertsTransport,
		type AlertSecret,
		type Destination,
		type ExportField
	} from '$lib/alerts/types';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SettingsFormGuard } from '$lib/settings/guards';
	import { automationLabels } from './labels';
	import { clonePlain, validateWebhookOrigin } from './simulator';

	interface Props {
		open: boolean;
		initialTab?: 'destination' | 'schedule';
		destination: Destination | null;
		alertsTransport: AlertsTransport;
		scheduleTransport: ScheduleTransport;
		guard?: SettingsFormGuard;
		onSaved: (saved: Destination, isNew: boolean) => void | Promise<void>;
		onScheduleSaved?: (saved: SchedulePolicy) => void | Promise<void>;
	}

	let {
		open = $bindable(),
		initialTab = 'destination',
		destination,
		alertsTransport,
		scheduleTransport,
		guard,
		onSaved,
		onScheduleSaved
	}: Props = $props();

	const t = $derived(automationLabels[i18n.locale].alertsPane);
	const alertT = $derived(alertLabelsMap[i18n.locale]);
	const schedT = $derived(scheduleLabels[i18n.locale]);

	let activeTab = $state<'destination' | 'schedule'>('destination');
	let draft = $state<Destination>(newDestination());
	let initialKind = $state<Destination['kind']>('Local');
	let destConsent = $state(false);
	let destError = $state('');
	let busy = $state(false);

	let urlInput = $state<HTMLInputElement>();
	let bearerInput = $state<HTMLInputElement>();

	let scheduleInfo = $state<ScheduleState | null>(null);
	let schedulePolicy = $state<SchedulePolicy | null>(null);
	let schedulePreview = $state<SchedulePreview | null>(null);
	let scheduleConsent = $state(false);
	let scheduleError = $state('');
	let scheduleGen = 0;

	function clearSecrets() {
		if (urlInput) urlInput.value = '';
		if (bearerInput) bearerInput.value = '';
		urlInput?.setCustomValidity('');
		bearerInput?.setCustomValidity('');
	}

	onDestroy(() => {
		clearSecrets();
		guard?.markDirty(false);
		guard?.setBusy(false);
	});

	async function loadScheduleFor(destId: string) {
		const gen = ++scheduleGen;
		scheduleError = '';
		schedulePreview = null;
		scheduleConsent = false;
		try {
			const loaded = await scheduleTransport.read(destId);
			if (gen === scheduleGen && open) {
				scheduleInfo = loaded;
				schedulePolicy = clonePlain(loaded.policy);
			}
		} catch {
			if (gen === scheduleGen && open) {
				scheduleError = schedT.error;
			}
		}
	}

	$effect(() => {
		if (open) {
			untrack(() => {
				activeTab = initialTab;
				clearSecrets();
				destError = '';
				destConsent = false;
				scheduleError = '';
				schedulePreview = null;
				scheduleConsent = false;
				draft = destination ? clonePlain(destination) : newDestination();
				initialKind = draft.kind;
				guard?.markDirty(true);
				if (destination && destination.revision > 0) {
					void loadScheduleFor(destination.id);
				} else {
					scheduleInfo = null;
					schedulePolicy = null;
				}
			});
		} else {
			untrack(() => {
				clearSecrets();
				scheduleGen++;
				guard?.markDirty(false);
			});
		}
	});

	function updateDraft(next: Destination) {
		destConsent = false;
		destError = '';
		draft = next;
	}

	function handleKindChange(kind: Destination['kind']) {
		clearSecrets();
		updateDraft({
			...draft,
			kind,
			public_origin:
				kind === 'Telegram'
					? 'https://api.telegram.org'
					: kind === 'Discord'
						? 'https://discord.com'
						: null,
			has_credentials: false,
			fields: []
		});
	}

	function handleUrlInput(input: HTMLInputElement) {
		input.setCustomValidity('');
		if (draft.kind === 'Webhook') {
			let origin: string | null = null;
			try {
				if (input.value) origin = new URL(input.value).origin;
			} catch {
				// Invalid partial URL cleared until validated.
			}
			updateDraft({
				...draft,
				public_origin: input.value ? origin : draft.public_origin
			});
		} else {
			destConsent = false;
			destError = '';
		}
	}

	function toggleField(field: ExportField) {
		if (draft.kind === 'Local') return;
		const fields = draft.fields.includes(field)
			? draft.fields.filter((f) => f !== field)
			: [...draft.fields, field];
		updateDraft({ ...draft, fields });
	}

	async function handleSaveDestination(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		destError = '';

		const trimmedName = draft.name.trim();
		if (!trimmedName || trimmedName.length > 120) {
			destError = t.validationNameRequired;
			return;
		}

		let secret: AlertSecret | null = null;
		const primaryVal = urlInput?.value.trim() ?? '';
		const secondaryVal = bearerInput?.value.trim() ?? '';

		if (draft.kind === 'Telegram' && (primaryVal || secondaryVal)) {
			if (!primaryVal || !secondaryVal) {
				const missing = !primaryVal ? urlInput : bearerInput;
				missing?.setCustomValidity(alertT.complete);
				missing?.reportValidity();
				return;
			}
			secret = { token: primaryVal, chat_id: secondaryVal };
		} else if (draft.kind === 'Discord' && primaryVal) {
			secret = { url: primaryVal, bearer: null };
		} else if (draft.kind === 'Webhook') {
			if (primaryVal || !draft.public_origin) {
				const originCheck = validateWebhookOrigin(primaryVal || draft.public_origin || '');
				if (originCheck.error) {
					destError = t.validationWebhookInvalid;
					return;
				}
				if (originCheck.origin) {
					draft = { ...draft, public_origin: originCheck.origin };
				}
			}
			if (primaryVal) {
				secret = {
					url: primaryVal,
					bearer: secondaryVal || null
				};
			}
		}

		if (draft.kind !== 'Local' && draft.fields.length === 0) {
			destError = t.validationFieldsRequired;
			return;
		}

		if (draft.enabled && !destConsent) {
			destError = t.validationConsentRequired;
			return;
		}

		const snapshot = $state.snapshot(draft);
		const payload: Destination = {
			id: snapshot.id,
			revision: snapshot.revision,
			name: trimmedName,
			kind: snapshot.kind,
			enabled: snapshot.enabled,
			fields: snapshot.kind === 'Local' ? [] : [...snapshot.fields],
			public_origin: snapshot.kind === 'Local' ? null : snapshot.public_origin,
			has_credentials: snapshot.has_credentials
		};

		clearSecrets();
		busy = true;
		guard?.setBusy(true);
		try {
			if (payload.kind === 'Local' && payload.enabled) {
				const avail = await alertsTransport.available();
				if (!avail) {
					destError = alertT.unavailable;
					return;
				}
			}
			const isNew = !destination || destination.revision === 0;
			const saved = await alertsTransport.save(payload, secret, destConsent);
			guard?.markDirty(false);
			await onSaved(saved, isNew);
			open = false;
			toast.success(
				isNew
					? t.destinationCreatedToast(saved.name)
					: t.destinationUpdatedToast(saved.name, saved.revision)
			);
		} catch {
			destError = alertT.error;
		} finally {
			busy = false;
			guard?.setBusy(false);
		}
	}

	function patchSchedule(update: Partial<SchedulePolicy>) {
		if (!schedulePolicy || busy) return;
		schedulePolicy = { ...schedulePolicy, ...update };
		scheduleConsent = false;
		schedulePreview = null;
		scheduleError = '';
		scheduleGen++;
		guard?.markDirty(true);
	}

	async function handlePreviewSchedule() {
		if (!schedulePolicy || busy) return;
		if (schedulePolicy.quiet && schedulePolicy.quiet.start === schedulePolicy.quiet.end) {
			scheduleError = t.validationStartEndEqual;
			return;
		}
		const gen = ++scheduleGen;
		const snapshot = $state.snapshot(schedulePolicy);
		busy = true;
		guard?.setBusy(true);
		scheduleError = '';
		try {
			const res = await scheduleTransport.preview(snapshot);
			if (gen === scheduleGen && open) {
				schedulePreview = res;
			}
		} catch {
			if (gen === scheduleGen && open) {
				scheduleError = schedT.error;
			}
		} finally {
			busy = false;
			guard?.setBusy(false);
		}
	}

	async function handleSaveSchedule(event: SubmitEvent) {
		event.preventDefault();
		if (!schedulePolicy || busy) return;
		if (schedulePolicy.quiet && schedulePolicy.quiet.start === schedulePolicy.quiet.end) {
			scheduleError = t.validationStartEndEqual;
			return;
		}
		if (schedulePolicy.urgent_bypass && !scheduleConsent) {
			scheduleError = schedT.consent;
			return;
		}
		const snapshot = $state.snapshot(schedulePolicy);
		busy = true;
		guard?.setBusy(true);
		scheduleError = '';
		try {
			const saved = await scheduleTransport.save(snapshot, scheduleConsent);
			guard?.markDirty(false);
			await onScheduleSaved?.(saved);
			open = false;
			toast.success(t.scheduleSavedToast);
		} catch {
			scheduleError = schedT.error;
		} finally {
			busy = false;
			guard?.setBusy(false);
		}
	}

	function closeDialog() {
		clearSecrets();
		guard?.markDirty(false);
		open = false;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Portal>
		<Dialog.Overlay class="fixed inset-0 z-50 bg-foreground/25" />
		<Dialog.Content
			class="fixed top-1/2 left-1/2 z-50 max-h-[90vh] w-[calc(100%-2rem)] max-w-xl -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-lg border border-separator bg-surface p-5 shadow-xl outline-none"
		>
			<div class="mb-4 flex items-center justify-between border-b border-separator pb-3">
				<Dialog.Title class="font-display text-sm font-semibold text-foreground">
					{destination && destination.revision > 0
						? t.dialogTitleEdit(destination.revision)
						: t.dialogTitleNew}
				</Dialog.Title>
				<Dialog.Close
					onclick={closeDialog}
					class="rounded p-1 text-secondary hover:bg-field hover:text-foreground"
					aria-label={t.closeDialogAria}
				>
					<X class="h-4 w-4" />
				</Dialog.Close>
			</div>

			<Tabs.Root bind:value={activeTab} class="flex flex-col gap-4">
				<Tabs.List class="grid w-full grid-cols-2 gap-1 rounded-md border border-separator bg-field p-1">
					<Tabs.Trigger
						value="destination"
						class="min-h-[36px] rounded px-3 py-1 text-xs font-medium text-secondary transition-colors data-[state=active]:bg-surface data-[state=active]:text-foreground data-[state=active]:shadow-xs"
					>
						{t.tabDestination}
					</Tabs.Trigger>
					<Tabs.Trigger
						value="schedule"
						class="min-h-[36px] rounded px-3 py-1 text-xs font-medium text-secondary transition-colors data-[state=active]:bg-surface data-[state=active]:text-foreground data-[state=active]:shadow-xs"
					>
						{t.tabSchedule}
					</Tabs.Trigger>
				</Tabs.List>

				<Tabs.Content value="destination" class="outline-none">
					<form onsubmit={handleSaveDestination} class="text-xs">
						<fieldset disabled={busy} class="flex flex-col gap-4 border-0 p-0">
							<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
								<label class="flex flex-col gap-1">
									<span class="font-medium text-secondary">{alertT.name}</span>
									<input
										type="text"
										required
										maxlength="120"
										value={draft.name}
										oninput={(e) => updateDraft({ ...draft, name: e.currentTarget.value })}
										placeholder="Ops Triage Channel"
										class="h-9 rounded-md border border-separator bg-field px-3 text-xs text-foreground focus:border-accent focus:outline-none"
									/>
								</label>

								<label class="flex flex-col gap-1">
									<span class="font-medium text-secondary">{t.kindLabel}</span>
									<select
										value={draft.kind}
										onchange={(e) =>
											handleKindChange(e.currentTarget.value as Destination['kind'])}
										class="h-9 rounded-md border border-separator bg-field px-2.5 text-xs text-foreground focus:border-accent focus:outline-none"
									>
										<option value="Local">{t.kindLocal}</option>
										<option value="Webhook">{t.kindWebhook}</option>
										<option value="Telegram">{t.kindTelegram}</option>
										<option value="Discord">{t.kindDiscord}</option>
									</select>
								</label>
							</div>

							{#if draft.kind === 'Local'}
								<p class="rounded-md border border-separator bg-field/60 p-3 text-[11px] text-secondary">
									{t.localNoExport}
								</p>
							{:else}
								{#if draft.kind === 'Telegram'}
									<p class="rounded-md border border-separator bg-field/60 p-2.5 text-[11px] text-secondary">
										{alertT.telegramHelp}
									</p>
								{:else if draft.kind === 'Discord'}
									<p class="rounded-md border border-separator bg-field/60 p-2.5 text-[11px] text-secondary">
										{alertT.discordHelp}
									</p>
								{/if}

								<div class="space-y-2 rounded-md border border-separator bg-field/40 p-3">
									<div class="flex flex-wrap items-center justify-between gap-2 font-mono text-[11px] text-secondary">
										<span>{t.originLabel}: {draft.public_origin ?? '—'}</span>
									</div>

									<label class="flex flex-col gap-1">
										<span class="flex items-center gap-1.5 font-medium text-secondary">
											<Lock class="h-3 w-3" />
											<span>{draft.kind === 'Telegram' ? alertT.token : alertT.url}</span>
										</span>
										<input
											bind:this={urlInput}
											type="password"
											autocomplete="off"
											required={draft.revision === 0 || draft.kind !== initialKind}
											oninput={(e) => handleUrlInput(e.currentTarget)}
											placeholder={draft.has_credentials
												? t.sealedPlaceholder
												: t.originPlaceholder}
											class="h-9 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
										/>
									</label>

									{#if draft.kind !== 'Discord'}
										<label class="flex flex-col gap-1">
											<span class="font-medium text-secondary">
												{draft.kind === 'Telegram' ? alertT.chat : alertT.bearer}
											</span>
											<input
												bind:this={bearerInput}
												type="password"
												autocomplete="off"
												required={draft.kind === 'Telegram' &&
													(draft.revision === 0 || draft.kind !== initialKind)}
												oninput={(e) => {
													e.currentTarget.setCustomValidity('');
													destConsent = false;
													destError = '';
												}}
												class="h-9 rounded-md border border-separator bg-surface px-3 font-mono text-xs text-foreground focus:border-accent focus:outline-none"
											/>
										</label>
									{/if}

									<p class="text-[11px] text-secondary">{alertT.preserve}</p>
									<p class="text-[11px] text-secondary">{alertT.revoke}</p>
									<p class="font-mono text-[11px] text-secondary">{t.originHint}</p>
								</div>

								<fieldset class="space-y-2 rounded-md border border-separator bg-field/60 p-3">
									<legend class="px-1 font-medium text-foreground">{alertT.fields}</legend>
									<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
										{#each EXPORT_FIELDS as field (field)}
											<label
												class="flex cursor-pointer items-center gap-2 rounded border border-separator bg-surface px-2.5 py-1.5"
											>
												<input
													type="checkbox"
													checked={draft.fields.includes(field)}
													onchange={() => toggleField(field)}
													class="h-3.5 w-3.5 accent-accent"
												/>
												<span class="text-[11px] text-foreground">{alertT.fieldNames[field]}</span>
											</label>
										{/each}
									</div>
									<p class="text-[11px] text-secondary">{alertT.metadata}</p>
									<p class="font-mono text-[11px] text-secondary">
										{alertT.preview}: {draft.fields.map((f) => alertT.fieldNames[f]).join(', ') || '—'}
									</p>
								</fieldset>
							{/if}

							<label class="flex cursor-pointer items-center gap-2 rounded-md border border-separator bg-field/60 px-3 py-2">
								<input
									type="checkbox"
									checked={draft.enabled}
									onchange={(e) => updateDraft({ ...draft, enabled: e.currentTarget.checked })}
									class="h-3.5 w-3.5 accent-accent"
								/>
								<span class="font-medium text-foreground">{alertT.enabled}</span>
							</label>

							{#if draft.enabled}
								<label
									class="flex cursor-pointer items-start gap-2.5 rounded-md border border-separator bg-selection-soft/60 p-3"
								>
									<input
										type="checkbox"
										required
										bind:checked={destConsent}
										class="mt-0.5 h-4 w-4 accent-accent"
									/>
									<span class="text-[11px] leading-relaxed text-foreground">
										<ShieldCheck class="mr-1 inline h-3.5 w-3.5 text-accent" />
										{alertT.consent}
									</span>
								</label>
							{/if}

							{#if destError}
								<p
									role="alert"
									class="rounded border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-[11px] text-status-danger"
								>
									{destError}
								</p>
							{/if}

							<div class="flex justify-end gap-2 border-t border-separator pt-3">
								<button
									type="button"
									onclick={closeDialog}
									class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground"
								>
									{alertT.cancel}
								</button>
								<button
									type="submit"
									class="min-h-[36px] rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95"
								>
									{alertT.save}
								</button>
							</div>
						</fieldset>
					</form>
				</Tabs.Content>

				<Tabs.Content value="schedule" class="outline-none">
					{#if !destination || destination.revision === 0}
						<div class="space-y-3 rounded-md border border-separator bg-field p-4 text-xs text-secondary">
							<p>{t.saveScheduleRequired}</p>
							<button
								type="button"
								onclick={() => (activeTab = 'destination')}
								class="min-h-[34px] rounded-md border border-separator bg-surface px-3 py-1 text-xs font-medium text-foreground hover:bg-selection-soft"
							>
								{t.tabDestination}
							</button>
						</div>
					{:else if schedulePolicy && scheduleInfo}
						<form onsubmit={handleSaveSchedule} class="text-xs">
							<fieldset disabled={busy} class="flex flex-col gap-3 border-0 p-0">
								<div class="space-y-1 text-[11px] text-secondary">
									<p>{schedT.notice}</p>
									<p>{schedT.dst}</p>
								</div>

								<div class="grid grid-cols-2 gap-2 rounded-md border border-separator bg-field/60 p-2.5 font-mono text-[11px] tabular-nums sm:grid-cols-4">
									<div>{schedT.deferred}: {scheduleInfo.deferred}</div>
									<div>{schedT.summarized}: {scheduleInfo.summarized}</div>
									<div>{schedT.expired}: {scheduleInfo.expired}</div>
									<div>{schedT.overflow}: {scheduleInfo.overflow}</div>
								</div>

								<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
									<label class="flex flex-col gap-1">
										<span class="font-medium text-secondary">{schedT.cadence}</span>
										<select
											value={typeof schedulePolicy.cadence === 'string'
												? 'Immediate'
												: 'Hourly' in schedulePolicy.cadence
													? 'Hourly'
													: 'Daily'}
											onchange={(e) =>
												patchSchedule({
													cadence:
														e.currentTarget.value === 'Immediate'
															? 'Immediate'
															: e.currentTarget.value === 'Hourly'
																? { Hourly: { minute: 0 } }
																: { Daily: { minute: 540 } }
												})}
											class="h-9 rounded-md border border-separator bg-field px-2.5 text-xs text-foreground focus:border-accent focus:outline-none"
										>
											<option value="Immediate">{schedT.immediate}</option>
											<option value="Hourly">{schedT.hourly}</option>
											<option value="Daily">{schedT.daily}</option>
										</select>
									</label>

									<label class="flex flex-col gap-1">
										<span class="font-medium text-secondary">{schedT.zone}</span>
										<select
											value={schedulePolicy.zone}
											onchange={(e) =>
												patchSchedule({
													zone: e.currentTarget.value === 'Utc' ? 'Utc' : 'SystemLocal'
												})}
											class="h-9 rounded-md border border-separator bg-field px-2.5 text-xs text-foreground focus:border-accent focus:outline-none"
										>
											<option value="SystemLocal">{schedT.local}</option>
											<option value="Utc">{schedT.utc}</option>
										</select>
									</label>
								</div>

								{#if typeof schedulePolicy.cadence !== 'string' && 'Hourly' in schedulePolicy.cadence}
									<label class="flex flex-col gap-1">
										<span class="font-medium text-secondary">{schedT.minute}</span>
										<input
											type="number"
											min="0"
											max="59"
											required
											value={schedulePolicy.cadence.Hourly.minute}
											oninput={(e) =>
												patchSchedule({
													cadence: { Hourly: { minute: Number(e.currentTarget.value) } }
												})}
											class="h-9 w-36 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums"
										/>
									</label>
								{:else if typeof schedulePolicy.cadence !== 'string' && 'Daily' in schedulePolicy.cadence}
									<label class="flex flex-col gap-1">
										<span class="font-medium text-secondary">{schedT.time}</span>
										<input
											type="time"
											required
											value={time(schedulePolicy.cadence.Daily.minute)}
											oninput={(e) =>
												patchSchedule({
													cadence: { Daily: { minute: minutes(e.currentTarget.value) } }
												})}
											class="h-9 w-36 rounded-md border border-separator bg-field px-3 font-mono text-xs tabular-nums"
										/>
									</label>
								{/if}

								<div class="space-y-2 rounded-md border border-separator bg-field/60 p-3">
									<div class="flex items-center justify-between">
										<span class="flex items-center gap-1.5 font-medium text-foreground">
											<Clock class="h-3.5 w-3.5 text-secondary" />
											<span>{schedT.quiet}</span>
										</span>
										<input
											type="checkbox"
											checked={schedulePolicy.quiet !== null}
											onchange={(e) =>
												patchSchedule({
													quiet: e.currentTarget.checked ? { start: 1320, end: 420 } : null
												})}
											aria-label={schedT.quiet}
											class="h-4 w-4 accent-accent"
										/>
									</div>
									{#if schedulePolicy.quiet}
										<div class="grid grid-cols-2 gap-2">
											<label class="flex flex-col gap-1">
												<span class="text-[11px] text-secondary">{schedT.start}</span>
												<input
													type="time"
													required
													value={time(schedulePolicy.quiet.start)}
													oninput={(e) =>
														schedulePolicy?.quiet &&
														patchSchedule({
															quiet: {
																...schedulePolicy.quiet,
																start: minutes(e.currentTarget.value)
															}
														})}
													class="h-8 rounded border border-separator bg-surface px-2 font-mono text-xs tabular-nums"
												/>
											</label>
											<label class="flex flex-col gap-1">
												<span class="text-[11px] text-secondary">{schedT.end}</span>
												<input
													type="time"
													required
													value={time(schedulePolicy.quiet.end)}
													oninput={(e) =>
														schedulePolicy?.quiet &&
														patchSchedule({
															quiet: {
																...schedulePolicy.quiet,
																end: minutes(e.currentTarget.value)
															}
														})}
													class="h-8 rounded border border-separator bg-surface px-2 font-mono text-xs tabular-nums"
												/>
											</label>
										</div>
									{/if}
								</div>

								<label class="flex cursor-pointer items-center gap-2">
									<input
										type="checkbox"
										checked={schedulePolicy.urgent_bypass}
										onchange={(e) => patchSchedule({ urgent_bypass: e.currentTarget.checked })}
										class="h-3.5 w-3.5 accent-accent"
									/>
									<span>{schedT.urgent}</span>
								</label>

								{#if schedulePolicy.urgent_bypass}
									<label
										class="flex cursor-pointer items-start gap-2 rounded-md border border-separator bg-selection-soft/60 p-3"
									>
										<input
											type="checkbox"
											required
											bind:checked={scheduleConsent}
											class="mt-0.5 h-4 w-4 accent-accent"
										/>
										<span class="text-[11px] leading-relaxed text-foreground">
											{schedT.consent}
										</span>
									</label>
								{/if}

								{#if schedulePreview}
									<p
										class="rounded border border-status-info/40 bg-status-info/10 px-3 py-2 font-mono text-[11px] tabular-nums text-foreground"
									>
										{schedT.next}: {new Date(schedulePreview.next_epoch_ms).toLocaleString(i18n.locale)} ·
										{schedulePreview.timezone}
									</p>
								{/if}

								{#if scheduleError}
									<p
										role="alert"
										class="rounded border border-status-danger/40 bg-status-danger/10 px-3 py-2 text-[11px] text-status-danger"
									>
										{scheduleError}
									</p>
								{/if}

								<div class="flex flex-wrap justify-between gap-2 border-t border-separator pt-3">
									<button
										type="button"
										onclick={() => void handlePreviewSchedule()}
										class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-foreground hover:bg-selection-soft"
									>
										{schedT.preview}
									</button>
									<div class="flex gap-2">
										<button
											type="button"
											onclick={closeDialog}
											class="min-h-[36px] rounded-md border border-separator bg-surface px-3 py-1.5 text-xs font-medium text-secondary hover:text-foreground"
										>
											{schedT.cancel}
										</button>
										<button
											type="submit"
											disabled={schedulePolicy.urgent_bypass && !scheduleConsent}
											class="min-h-[36px] rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground hover:opacity-95 disabled:opacity-45"
										>
											{schedT.save}
										</button>
									</div>
								</div>
							</fieldset>
						</form>
					{:else if scheduleError}
						<p role="alert" class="text-xs text-status-danger">{scheduleError}</p>
					{/if}
				</Tabs.Content>
			</Tabs.Root>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
