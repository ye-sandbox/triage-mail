import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import DueDateDashboard from './DueDateDashboard.svelte';
import RemindersModule from './RemindersModule.svelte';
import type { DueDateItem, RemindersTransport, ScheduledReminderConfig } from './types';
import type { AlertsTransport, Destination } from '../alerts/types';
import { SettingsFormGuard } from '../settings/guards';
import { i18n } from '../i18n/index.svelte';

let cleanup: (() => void) | undefined;

afterEach(() => {
	cleanup?.();
	cleanup = undefined;
	document.body.innerHTML = '';
	i18n.locale = 'en';
	vi.restoreAllMocks();
});

async function settle() {
	for (let i = 0; i < 20; i++) await Promise.resolve();
	flushSync();
}

describe('DueDateDashboard', () => {
	const sampleItems: DueDateItem[] = [
		{
			message_id: 'msg-overdue',
			account_id: 'acc-1',
			subject: 'Conta de Energia Elétrica',
			sender: 'Enel <cobranca@example.com>',
			received_at_epoch_ms: 1700000000000,
			is_read: true,
			digitable_line: '34191.79001 01043.510047 91020.150008 8 99990000015025',
			amount_cents: 15025, // R$ 150,25
			due_date_iso: '2020-01-15', // Definitely overdue
			is_dismissed: false
		},
		{
			message_id: 'msg-upcoming',
			account_id: 'acc-1',
			subject: 'Fatura de Internet Fibra',
			sender: 'Provedor <financeiro@example.com>',
			received_at_epoch_ms: 1700000000000,
			is_read: false,
			digitable_line: '23793.38128 60000.123456 12345.678901 1 85000000009990',
			amount_cents: 9990, // R$ 99,90
			due_date_iso: '2099-12-31', // Definitely upcoming
			is_dismissed: false
		}
	];

	function mountDashboard(opts?: {
		items?: DueDateItem[];
		onSelectMessage?: (id: string) => void;
		onConfigureReminders?: () => void;
	}) {
		const items = opts?.items ?? sampleItems;
		const transport: RemindersTransport = {
			listDueDates: vi.fn(async () => items),
			dismissDueDate: vi.fn(async () => {}),
			listConfigs: vi.fn(async () => []),
			saveConfig: vi.fn(async (c) => c),
			deleteConfig: vi.fn(async () => {})
		};

		const target = document.createElement('div');
		document.body.append(target);

		const close = vi.fn();
		const onSelectMessage = opts?.onSelectMessage ?? vi.fn();
		const onConfigureReminders = opts?.onConfigureReminders ?? vi.fn();

		const component = mount(DueDateDashboard, {
			target,
			props: {
				open: true,
				close,
				onSelectMessage,
				onConfigureReminders,
				transport,
				accounts: [{
					account_id: 'acc-1',
					email_display: 'user@example.com',
					imap_host: 'mail.example.com',
					imap_port: 993,
					auth_method: 'AppPassword'
				}]
			}
		});

		cleanup = () => void unmount(component);

		return { target, transport, close, onSelectMessage, onConfigureReminders };
	}

	it('renders bills with formatted currency, dates, and non-payment disclaimer', async () => {
		const d = mountDashboard();
		await settle();

		expect(d.target.textContent).toContain('Due Dates & Boletos');
		expect(d.target.textContent).toContain('Extraction never implies a bill was paid');
		expect(d.target.textContent).toContain('Conta de Energia Elétrica');
		expect(d.target.textContent).toContain('Fatura de Internet Fibra');
		expect(d.target.textContent).toContain('34191.79001');
	});

	it('renders bilingual content in pt-BR', async () => {
		i18n.locale = 'pt-BR';
		const d = mountDashboard();
		await settle();

		expect(d.target.textContent).toContain('Contas e Boletos a Vencer');
		expect(d.target.textContent).toContain('A extração nunca implica que um boleto foi pago');
		expect(d.target.textContent).toContain('Atrasados');
		expect(d.target.textContent).toContain('A Vencer');
	});

	it('copies barcode digitable line to clipboard with visual feedback', async () => {
		const writeTextMock = vi.fn(async () => {});
		Object.assign(navigator, {
			clipboard: { writeText: writeTextMock }
		});

		const d = mountDashboard();
		await settle();

		const copyButtons = [...d.target.querySelectorAll('button')].filter((b) =>
			b.textContent?.includes('Copy barcode')
		);
		expect(copyButtons.length).toBeGreaterThan(0);

		copyButtons[0].click();
		await settle();

		expect(writeTextMock).toHaveBeenCalledWith(
			'34191.79001 01043.510047 91020.150008 8 99990000015025'
		);
		expect(d.target.textContent).toContain('Copied!');
	});

	it('dismisses a bill reminder calling transport and updating state', async () => {
		const d = mountDashboard();
		await settle();

		const dismissButtons = [...d.target.querySelectorAll('button')].filter((b) =>
			b.textContent?.trim() === 'Dismiss'
		);
		expect(dismissButtons.length).toBeGreaterThan(0);

		dismissButtons[0].click();
		await settle();

		expect(d.transport.dismissDueDate).toHaveBeenCalledWith('msg-overdue');
		expect(d.target.textContent).toContain('Dismissed');
	});

	it('navigates to email when View Email button is clicked', async () => {
		const onSelectMessage = vi.fn();
		const d = mountDashboard({ onSelectMessage });
		await settle();

		const viewButtons = [...d.target.querySelectorAll('button')].filter((b) =>
			b.textContent?.includes('View Email')
		);
		expect(viewButtons.length).toBeGreaterThan(0);

		viewButtons[0].click();
		await settle();

		expect(onSelectMessage).toHaveBeenCalledWith('msg-overdue');
		expect(d.close).toHaveBeenCalled();
	});

	it('opens reminders configuration when Configure Reminders is clicked', async () => {
		const onConfigureReminders = vi.fn();
		const d = mountDashboard({ onConfigureReminders });
		await settle();

		const configButton = [...d.target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Configure Reminders')
		);
		expect(configButton).toBeDefined();

		configButton?.click();
		await settle();

		expect(onConfigureReminders).toHaveBeenCalled();
	});

	it('filters list by status tabs', async () => {
		const d = mountDashboard();
		await settle();

		const overdueTab = [...d.target.querySelectorAll('button')].find((b) =>
			b.textContent?.trim() === 'Overdue'
		);
		expect(overdueTab).toBeDefined();

		overdueTab?.click();
		await settle();

		expect(d.target.textContent).toContain('Conta de Energia Elétrica');
		expect(d.target.textContent).not.toContain('Fatura de Internet Fibra');
	});
});

describe('RemindersModule', () => {
	const sampleDestination: Destination = {
		id: 'dest-local',
		revision: 1,
		name: 'Desktop Notification',
		kind: 'Local',
		enabled: true,
		fields: ['Subject'],
		public_origin: null,
		has_credentials: false
	};

	const sampleConfig: ScheduledReminderConfig = {
		version: 1,
		id: 'rule-1',
		revision: 1,
		name: 'Boleto 2 days before',
		enabled: true,
		trigger: {
			DueDate: {
				days_before: 2,
				target_hour: 9,
				min_amount_cents: 5000
			}
		},
		destination_ids: ['dest-local']
	};

	function mountModule(opts?: {
		configs?: ScheduledReminderConfig[];
		destinations?: Destination[];
	}) {
		const configs = opts?.configs ?? [sampleConfig];
		const destinations = opts?.destinations ?? [sampleDestination];

		const transport: RemindersTransport = {
			listDueDates: vi.fn(async () => []),
			dismissDueDate: vi.fn(async () => {}),
			listConfigs: vi.fn(async () => configs),
			saveConfig: vi.fn(async (c) => ({ ...c, revision: c.revision + 1 })),
			deleteConfig: vi.fn(async () => {})
		};

		const alertsTransport: AlertsTransport = {
			list: vi.fn(async () => destinations),
			history: vi.fn(async () => []),
			save: vi.fn(async (d) => d),
			remove: vi.fn(async () => {}),
			test: vi.fn(async () => {}),
			replay: vi.fn(async () => {}),
			available: vi.fn(async () => true)
		};

		const guard = new SettingsFormGuard();
		const target = document.createElement('div');
		document.body.append(target);

		const component = mount(RemindersModule, {
			target,
			props: {
				guard,
				transport,
				alertsTransport,
				accounts: [{
					account_id: 'acc-1',
					email_display: 'user@example.com',
					imap_host: 'mail.example.com',
					imap_port: 993,
					auth_method: 'AppPassword'
				}]
			}
		});

		cleanup = () => void unmount(component);

		return { target, guard, transport, alertsTransport };
	}

	it('lists scheduled reminder rules and displays triggers', async () => {
		const m = mountModule();
		await settle();

		expect(m.target.textContent).toContain('Scheduled Reminders');
		expect(m.target.textContent).toContain('Boleto 2 days before');
		expect(m.target.textContent).toContain('2 days before due date at 09:00');
	});

	it('creates a new reminder rule and marks guard dirty', async () => {
		const m = mountModule({ configs: [] });
		await settle();

		const addBtn = [...m.target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('New Reminder Rule')
		);
		expect(addBtn).toBeDefined();

		addBtn?.click();
		await settle();

		expect(m.guard.dirty).toBe(true);

		const nameInput = m.target.querySelector<HTMLInputElement>('#reminder-name');
		expect(nameInput).toBeDefined();
		if (nameInput) {
			nameInput.value = 'Custom Alert';
			nameInput.dispatchEvent(new Event('input'));
		}

		const saveBtn = [...m.target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Save Rule')
		);
		expect(saveBtn).toBeDefined();
		saveBtn?.click();
		await settle();

		expect(m.transport.saveConfig).toHaveBeenCalled();
		expect(m.guard.dirty).toBe(false);
	});

	it('toggles rule enabled status directly from the list', async () => {
		const m = mountModule();
		await settle();

		const toggleBtn = m.target.querySelector<HTMLButtonElement>('button[aria-label="Enabled"]');
		expect(toggleBtn).toBeDefined();

		toggleBtn?.click();
		await settle();

		expect(m.transport.saveConfig).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'rule-1',
				enabled: false
			})
		);
	});

	it('deletes a reminder rule with revision check', async () => {
		const m = mountModule();
		await settle();

		const deleteBtn = [...m.target.querySelectorAll('button')].find((b) =>
			b.textContent?.trim() === 'Delete'
		);
		expect(deleteBtn).toBeDefined();

		deleteBtn?.click();
		await settle();

		expect(m.transport.deleteConfig).toHaveBeenCalledWith('rule-1', 1);
	});
});

import WdgDueDatesPage from './WdgDueDatesPage.svelte';
import SidebarNavRoutes from '../shell/SidebarNavRoutes.svelte';
import InboxSession from '../onboarding/InboxSession.svelte';
import { appStore } from '../stores/app.svelte';
import { workspaceRegistry } from '../inbox/workspace.svelte';
import type { InboxTransport, MessageRowDto } from '../inbox/types';
import type { SettingsTransport } from '../settings/types';

import { afterEach as afterEachInner } from 'vitest';

describe('UI v2 Due Dates Hub (WdgDueDatesPage & WdgReminderConfigDrawer)', () => {
	afterEachInner(() => {
		appStore.consumePendingSelectMessage();
		appStore.setScope({ kind: 'inbox' });
		appStore.overdueAndDueTodayCount = 0;
	});
	const todayIso = new Date().toISOString().slice(0, 10);

	const v2Items: DueDateItem[] = [
		{
			message_id: 'msg_due_overdue',
			account_id: 'acct_personal',
			subject: 'Conta de Energia — Outubro',
			sender: 'Energia Utility <energia@example.org>',
			received_at_epoch_ms: 1700000000000,
			is_read: true,
			digitable_line: '34191.79001 01043.510047 91020.150008 8 99030000031240',
			amount_cents: 31240,
			due_date_iso: '2020-01-15',
			is_dismissed: false
		},
		{
			message_id: 'msg_due_today',
			account_id: 'acct_work',
			subject: 'Fatura Fibra Empresarial',
			sender: 'Fibra Telecom <fibra@example.com>',
			received_at_epoch_ms: 1700000000000,
			is_read: false,
			digitable_line: '23793.38128 60000.000003 01000.149904 1 99060000014990',
			amount_cents: 14990,
			due_date_iso: todayIso,
			is_dismissed: false
		},
		{
			message_id: 'msg_due_upcoming',
			account_id: 'acct_work',
			subject: 'Cloud Hosting Invoice',
			sender: 'Cloud Infra <cloud@example.net>',
			received_at_epoch_ms: 1700000000000,
			is_read: false,
			digitable_line: '03399.12345 67890.123456 78901.234567 3 99150000008900',
			amount_cents: 8900,
			due_date_iso: '2099-12-31',
			is_dismissed: false
		}
	];

	const v2Destinations: Destination[] = [
		{
			id: 'dest_local',
			revision: 1,
			name: 'Native OS Notification',
			kind: 'Local',
			enabled: true,
			fields: ['Account', 'Sender', 'Subject', 'ExtractedFacts'],
			public_origin: null,
			has_credentials: false
		}
	];

	const v2Configs: ScheduledReminderConfig[] = [
		{
			version: 1,
			id: 'rule_v2_1',
			revision: 2,
			name: 'Boletos 3 days before due',
			enabled: true,
			trigger: { DueDate: { days_before: 3, target_hour: 9, min_amount_cents: 5000 } },
			destination_ids: ['dest_local']
		}
	];

	function mountDueDatesHub(opts?: {
		items?: DueDateItem[];
		configs?: ScheduledReminderConfig[];
		onNavigateToInbox?: (messageId: string) => void;
	}) {
		let currentItems = structuredClone(opts?.items ?? v2Items);
		let currentConfigs = structuredClone(opts?.configs ?? v2Configs);

		const remindersTransport: RemindersTransport = {
			listDueDates: vi.fn(async () => structuredClone(currentItems)),
			dismissDueDate: vi.fn(async (id) => {
				currentItems = currentItems.map((it) =>
					it.message_id === id ? { ...it, is_dismissed: true } : it
				);
			}),
			listConfigs: vi.fn(async () => structuredClone(currentConfigs)),
			saveConfig: vi.fn(async (cfg) => {
				const exists = currentConfigs.some((c) => c.id === cfg.id);
				const next = { ...structuredClone(cfg), revision: cfg.revision + 1 };
				currentConfigs = exists
					? currentConfigs.map((c) => (c.id === cfg.id ? next : c))
					: [next, ...currentConfigs];
				return structuredClone(next);
			}),
			deleteConfig: vi.fn(async (id) => {
				currentConfigs = currentConfigs.filter((c) => c.id !== id);
			})
		};

		const alertsTransport: AlertsTransport = {
			list: vi.fn(async () => structuredClone(v2Destinations)),
			history: vi.fn(async () => []),
			save: vi.fn(async (d) => d),
			remove: vi.fn(async () => {}),
			test: vi.fn(async () => {}),
			replay: vi.fn(async () => {}),
			available: vi.fn(async () => true)
		};

		const accounts = [
			{
				account_id: 'acct_personal',
				email_display: 'personal@example.org',
				imap_host: 'mail.example.org',
				imap_port: 993,
				auth_method: 'AppPassword' as const
			},
			{
				account_id: 'acct_work',
				email_display: 'work@example.com',
				imap_host: 'mail.example.com',
				imap_port: 993,
				auth_method: 'OAuth2PublicPkce' as const
			}
		];

		const target = document.createElement('div');
		document.body.append(target);

		const onNavigateToInbox = opts?.onNavigateToInbox ?? vi.fn();
		const component = mount(WdgDueDatesPage, {
			target,
			props: {
				remindersTransport,
				alertsTransport,
				accounts,
				onNavigateToInbox
			}
		});

		cleanup = () => void unmount(component);
		return { target, remindersTransport, alertsTransport, onNavigateToInbox };
	}

	it('renders KPI cards, non-payment disclaimer, boleto cards, and syncs Overdue + Due Today badge', async () => {
		appStore.overdueAndDueTodayCount = 0;
		const h = mountDueDatesHub();
		await settle();

		expect(h.target.textContent).toContain('Due Dates & Boletos');
		expect(h.target.textContent).toContain('Extraction never implies a bill was paid');
		expect(h.target.textContent).toContain('Conta de Energia — Outubro');
		expect(h.target.textContent).toContain('Fatura Fibra Empresarial');
		expect(h.target.textContent).toContain('Cloud Hosting Invoice');
		expect(h.target.textContent).toContain('1 active');
		// Overdue (1) + Due Today (1) = 2
		expect(appStore.overdueAndDueTodayCount).toBe(2);

		i18n.setLocale('pt-BR');
		await settle();
		expect(h.target.textContent).toContain('Contas e Boletos a Vencer');
		expect(h.target.textContent).toContain('Extração local somente leitura:');
	});

	it('filters via KPI cards, account selector, barcode search, and Clear Filters', async () => {
		const h = mountDueDatesHub();
		await settle();

		// Click "Overdue" KPI card
		const overdueKpi = [...h.target.querySelectorAll<HTMLButtonElement>('button[aria-pressed]')].find(
			(b) => b.textContent?.includes('Overdue')
		);
		expect(overdueKpi).toBeDefined();
		overdueKpi?.click();
		await settle();

		expect(overdueKpi?.getAttribute('aria-pressed')).toBe('true');
		expect(h.target.textContent).toContain('Conta de Energia — Outubro');
		expect(h.target.textContent).not.toContain('Cloud Hosting Invoice');

		// Toggle KPI back to All
		overdueKpi?.click();
		await settle();
		expect(h.target.textContent).toContain('Cloud Hosting Invoice');

		// Search by digitable line digits ignoring dots/spaces
		const searchInput = h.target.querySelector<HTMLInputElement>('input[type="search"]');
		expect(searchInput).not.toBeNull();
		if (searchInput) {
			searchInput.value = '0339912345';
			searchInput.dispatchEvent(new Event('input', { bubbles: true }));
		}
		await settle();
		expect(h.target.textContent).toContain('Cloud Hosting Invoice');
		expect(h.target.textContent).not.toContain('Conta de Energia — Outubro');

		// Empty state + Clear Filters when combined with mismatching account
		const accountSelect = h.target.querySelector<HTMLSelectElement>('select');
		if (accountSelect) {
			accountSelect.value = 'acct_personal';
			accountSelect.dispatchEvent(new Event('change', { bubbles: true }));
		}
		await settle();
		expect(h.target.textContent).toContain('Clear Filters');
	});

	it('copies digitable line, dismisses active bill, and decrements live sidebar badge', async () => {
		const writeTextMock = vi.fn(async () => {});
		Object.assign(navigator, { clipboard: { writeText: writeTextMock } });

		const h = mountDueDatesHub();
		const navHost = document.createElement('div');
		document.body.append(navHost);
		const navComp = mount(SidebarNavRoutes, { target: navHost });
		await settle();

		const badgeBefore = Array.from(navHost.querySelectorAll('span')).find(
			(el) => el.getAttribute('title') === 'Overdue + Due Today'
		);
		expect(badgeBefore?.textContent?.trim()).toBe('2');

		const copyBtn = [...h.target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Copy barcode')
		);
		copyBtn?.click();
		await settle();
		expect(writeTextMock).toHaveBeenCalledWith(v2Items[0].digitable_line);
		expect(h.target.textContent).toContain('Copied!');

		const dismissBtn = [...h.target.querySelectorAll('button')].find(
			(b) => b.textContent?.trim() === 'Dismiss'
		);
		dismissBtn?.click();
		await settle();

		expect(h.remindersTransport.dismissDueDate).toHaveBeenCalledWith('msg_due_overdue');
		expect(appStore.overdueAndDueTodayCount).toBe(1);
		expect(badgeBefore?.textContent?.trim()).toBe('1');

		void unmount(navComp);
	});

	it('manages scheduled reminder rules in WdgReminderConfigDrawer (create, toggle, delete)', async () => {
		const h = mountDueDatesHub();
		await settle();

		const cfgBtn = [...h.target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Configure Reminder Schedule')
		);
		expect(cfgBtn).toBeDefined();
		cfgBtn?.click();
		await settle();

		expect(document.body.textContent).toContain('Boletos 3 days before due');

		// Toggle enabled status
		const switchBtn = document.body.querySelector<HTMLButtonElement>('button[role="switch"]');
		expect(switchBtn).not.toBeNull();
		switchBtn?.click();
		await settle();
		expect(h.remindersTransport.saveConfig).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'rule_v2_1', enabled: false })
		);

		// Create a new rule using a quick preset chip (7d)
		const addBtn = [...document.body.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('New Reminder Rule')
		);
		addBtn?.click();
		await settle();

		const preset7d = [...document.body.querySelectorAll('button')].find(
			(b) => b.textContent?.trim() === '7d before'
		);
		expect(preset7d).toBeDefined();
		preset7d?.click();

		const nameInput = document.body.querySelector<HTMLInputElement>('#reminder-name');
		if (nameInput) {
			nameInput.value = 'Weekly Utility Warning';
			nameInput.dispatchEvent(new Event('input', { bubbles: true }));
		}
		await settle();

		const saveBtn = [...document.body.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Save Rule')
		);
		saveBtn?.click();
		await settle();

		expect(h.remindersTransport.saveConfig).toHaveBeenLastCalledWith(
			expect.objectContaining({
				name: 'Weekly Utility Warning',
				trigger: expect.objectContaining({
					DueDate: expect.objectContaining({ days_before: 7 })
				})
			})
		);

		// Delete original rule with revision check
		const deleteBtn = document.body.querySelector<HTMLButtonElement>(
			'button[aria-label="Delete Boletos 3 days before due"]'
		);
		expect(deleteBtn).not.toBeNull();
		deleteBtn?.click();
		await settle();
		expect(h.remindersTransport.deleteConfig).toHaveBeenCalledWith('rule_v2_1', 3);
	});

	it('routes View Email to inbox and selects target message in InboxSession after connect search finishes', async () => {
		const onNavigateToInbox = vi.fn();
		const h = mountDueDatesHub({ onNavigateToInbox });
		await settle();

		const viewButtons = [...h.target.querySelectorAll('button')].filter((b) =>
			b.textContent?.includes('View Email')
		);
		expect(viewButtons.length).toBe(3);

		viewButtons[0].click();
		await settle();

		expect(appStore.activeScope).toEqual({ kind: 'all' });
		expect(appStore.pendingSelectMessageId).toBe('msg_due_overdue');
		expect(onNavigateToInbox).toHaveBeenCalledWith('msg_due_overdue');

		// Unmount DueDatesPage and mount connected InboxSession to verify delayed search completion selects the message
		cleanup?.();
		cleanup = undefined;

		let resolveSubscribe!: () => void;
		const subscribeGate = new Promise<void>((resolve) => {
			resolveSubscribe = resolve;
		});

		const firstOtherRow: MessageRowDto = {
			id: 'msg_other_top',
			account_id: 'acct_work',
			folder: 'INBOX',
			uid: 41,
			subject: 'Earlier Unrelated Email',
			sender: 'Newsletter <news@example.com>',
			received_at_epoch_ms: 1700000005000,
			is_read: false,
			has_attachments: false,
			tags: [],
			folder_override: null,
			category_badge: 'Newsletter'
		};

		const row: MessageRowDto = {
			id: 'msg_due_overdue',
			account_id: 'acct_personal',
			folder: 'INBOX',
			uid: 42,
			subject: 'Conta de Energia — Outubro',
			sender: 'Energia Utility <energia@example.org>',
			received_at_epoch_ms: 1700000000000,
			is_read: true,
			has_attachments: false,
			tags: ['Boleto'],
			folder_override: null,
			category_badge: 'Financial'
		};

		const inboxTransport: InboxTransport = {
			subscribe: vi.fn(async () => {
				await subscribeGate;
				return () => {};
			}),
			search: vi.fn(async () => [firstOtherRow, row]),
			triage: vi.fn(async () => {}),
			listFolders: vi.fn(async () => [])
		};

		const settingsTransport: SettingsTransport = {
			get: vi.fn(async () => ({
				ai: { enabled: false, provider: 'Ollama' as const, endpoint: '', model: '' },
				tokenizer: 'Unicode61RemoveDiacritics' as const,
				has_api_key: false,
				model_cached: false,
				experimental_semantic_search: false
			})),
			save: vi.fn(async () => {}),
			tokenizer: vi.fn(async () => {}),
			setExperimentalSemanticSearch: vi.fn(async () => {}),
			download: vi.fn(async () => {}),
			summarize: vi.fn(async () => '')
		};

		const inboxTarget = document.createElement('div');
		document.body.append(inboxTarget);
		const sessionComp = mount(InboxSession, {
			target: inboxTarget,
			props: {
				connected: true,
				inboxTransport,
				settingsTransport,
				remindersTransport: h.remindersTransport
			}
		});
		cleanup = () => void unmount(sessionComp);

		// Flush before subscribe resolves: pendingSelectMessageId must NOT be lost prematurely
		await settle();
		expect(appStore.pendingSelectMessageId).toBe('msg_due_overdue');

		resolveSubscribe();
		await settle();

		expect(appStore.pendingSelectMessageId).toBeNull();
		expect(workspaceRegistry.current?.inbox.selectedId).toBe('msg_due_overdue');
		expect(inboxTarget.textContent).toContain('Conta de Energia — Outubro');
	});

	it('injects and selects demo due-date message in InboxSession demo mode', async () => {
		appStore.setScope({ kind: 'all' });
		appStore.requestSelectMessage('msg_due_energia');

		const inboxTarget = document.createElement('div');
		document.body.append(inboxTarget);
		const sessionComp = mount(InboxSession, {
			target: inboxTarget,
			props: { connected: false }
		});
		cleanup = () => void unmount(sessionComp);

		await settle();

		expect(appStore.pendingSelectMessageId).toBeNull();
		expect(workspaceRegistry.current?.inbox.selectedId).toBe('msg_due_energia');
		expect(inboxTarget.textContent).toContain('Fatura de Energia Elétrica');
	});
});
