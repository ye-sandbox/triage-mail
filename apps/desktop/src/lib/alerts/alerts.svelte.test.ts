import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import WdgAlertsManager from '../automation/WdgAlertsManager.svelte';
import { demoAutomationBackend } from '../automation/automation.fixtures';
import { newDestination, type AlertsTransport, type Destination } from './types';
import { SettingsFormGuard } from '../settings/guards';
import { i18n } from '../i18n/index.svelte';
let cleanup: (() => void) | undefined;
afterEach(() => {
	cleanup?.();
	cleanup = undefined;
	document.body.innerHTML = '';
	i18n.locale = 'en';
});
async function settle() {
	for (let i = 0; i < 20; i++) await Promise.resolve();
	flushSync();
}

describe('WdgAlertsManager & WdgAlertDestinationDialog (UI v2)', () => {
	async function mountWdg(opts?: { available?: boolean }) {
		const destination: Destination = {
			...newDestination(),
			id: 'destination',
			name: 'Workflow',
			revision: 1,
			kind: 'Webhook',
			enabled: true,
			public_origin: 'https://hooks.example.com',
			fields: ['Subject']
		};
		const alertsTransport: AlertsTransport = {
			list: vi.fn(async () => [destination]),
			history: vi.fn(async () => []),
			save: vi.fn(async (d) => ({ ...d, revision: d.revision + 1 })),
			remove: vi.fn(async () => {}),
			test: vi.fn(async () => {}),
			replay: vi.fn(async () => {}),
			available: vi.fn(async () => opts?.available ?? true)
		};
		const guard = new SettingsFormGuard();
		const target = document.createElement('div');
		document.body.append(target);
		const component = mount(WdgAlertsManager, {
			target,
			props: {
				alertsTransport,
				scheduleTransport: demoAutomationBackend.scheduleTransport,
				guard,
				selectedIds: ['msg-1']
			}
		});
		cleanup = () => void unmount(component);
		await settle();
		const button = (text: string) => {
			const b = [...document.body.querySelectorAll('button')].find((el) =>
				el.textContent?.includes(text)
			);
			if (!b) throw Error(`Missing button: ${text}`);
			return b;
		};
		return { target, guard, alertsTransport, destination, button };
	}

	it('requires 2-step confirmation for test and replay without firing prematurely', async () => {
		const f = await mountWdg();
		f.button('Test delivery').click();
		flushSync();
		expect(f.alertsTransport.test).not.toHaveBeenCalled();

		f.button('Confirm delivery').click();
		await settle();
		expect(f.alertsTransport.test).toHaveBeenCalledWith('destination', 1, true);

		f.button('Replay selected messages').click();
		flushSync();
		expect(f.alertsTransport.replay).not.toHaveBeenCalled();

		f.button('Confirm delivery').click();
		await settle();
		expect(f.alertsTransport.replay).toHaveBeenCalledWith(['msg-1'], 'destination', 1, true);
	});

	it('clears detached secret inputs, keeps secrets out of Destination DTO, and rejects partial Telegram credentials', async () => {
		const f = await mountWdg();
		f.button('Edit').click();
		await settle();
		expect(f.guard.dirty).toBe(true);

		const pwInputs = [...document.body.querySelectorAll<HTMLInputElement>('input[type=password]')];
		pwInputs[0].value = 'https://hooks.example.com/private-secret';
		pwInputs[1].value = 'synthetic-bearer';

		const consent = document.body.querySelector<HTMLInputElement>(
			'input[required][type=checkbox]'
		)!;
		consent.checked = true;
		consent.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		document.body
			.querySelector('form')!
			.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		flushSync();
		expect(pwInputs[0].value).toBe('');
		expect(pwInputs[1].value).toBe('');
		await settle();

		expect(f.alertsTransport.save).toHaveBeenCalledWith(
			f.destination,
			{ url: 'https://hooks.example.com/private-secret', bearer: 'synthetic-bearer' },
			true
		);
		expect(JSON.stringify(vi.mocked(f.alertsTransport.save).mock.calls[0][0])).not.toContain(
			'private-secret'
		);

		// Reopen, switch to Telegram, enter only token -> blocks before IPC
		f.button('Edit').click();
		await settle();
		const kindSelect = [...document.body.querySelectorAll('select')].find((s) =>
			[...s.options].some((o) => o.value === 'Telegram')
		)!;
		kindSelect.value = 'Telegram';
		kindSelect.dispatchEvent(new Event('change', { bubbles: true }));
		await settle();

		const [tok, chat] = [...document.body.querySelectorAll<HTMLInputElement>('input[type=password]')];
		tok.value = '999:synthetic';
		document.body
			.querySelector('form')!
			.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		await settle();
		expect(vi.mocked(f.alertsTransport.save)).toHaveBeenCalledTimes(1);
		expect(chat.validationMessage).toContain('both');
	});

	it('keeps Local destinations free of export fields and checks OS notification availability', async () => {
		const f = await mountWdg({ available: false });
		f.button('New Destination').click();
		await settle();

		const nameInput = document.body.querySelector<HTMLInputElement>('input[maxlength="120"]')!;
		nameInput.value = 'Desktop Local';
		nameInput.dispatchEvent(new Event('input', { bubbles: true }));
		const checkboxes = [...document.body.querySelectorAll<HTMLInputElement>('input[type=checkbox]')];
		checkboxes[0].checked = true;
		checkboxes[0].dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		const consent = document.body.querySelector<HTMLInputElement>(
			'input[required][type=checkbox]'
		)!;
		consent.checked = true;
		consent.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		document.body
			.querySelector('form')!
			.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		await settle();

		expect(f.alertsTransport.available).toHaveBeenCalled();
		expect(f.alertsTransport.save).not.toHaveBeenCalled();
		expect(document.body.textContent).toContain('Local notifications unavailable');
	});
});
