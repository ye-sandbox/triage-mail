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

	it('navigates to email when View email button is clicked', async () => {
		const onSelectMessage = vi.fn();
		const d = mountDashboard({ onSelectMessage });
		await settle();

		const viewButtons = [...d.target.querySelectorAll('button')].filter((b) =>
			b.textContent?.includes('View email')
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
