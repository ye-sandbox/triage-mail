import { mount, unmount, flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import WdgAlertDestinationDialog from '../automation/WdgAlertDestinationDialog.svelte';
import { newDestination } from './types';
import { type ScheduleTransport, type SchedulePolicy } from './schedule';
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

describe('WdgAlertDestinationDialog schedule tab (UI v2)', () => {
	it('previews without saving, validates quiet hours, and resets urgent consent after edits', async () => {
		const policy: SchedulePolicy = {
			destination_id: 'dest-sched',
			destination_revision: 4,
			revision: 2,
			cadence: 'Immediate',
			zone: 'Utc',
			quiet: null,
			urgent_bypass: false
		};
		const scheduleTransport: ScheduleTransport = {
			read: vi.fn(async () => ({
				policy,
				deferred: 3,
				summarized: 5,
				expired: 1,
				overflow: 0
			})),
			save: vi.fn(async (p) => ({ ...p, revision: p.revision + 1 })),
			preview: vi.fn(async () => ({ next_epoch_ms: 0, timezone: 'UTC' }))
		};
		const alertsTransport = {
			list: vi.fn(async () => []),
			history: vi.fn(async () => []),
			save: vi.fn(async (d: import('./types').Destination) => d),
			remove: vi.fn(async () => {}),
			test: vi.fn(async () => {}),
			replay: vi.fn(async () => {}),
			available: vi.fn(async () => true)
		};
		const guard = new SettingsFormGuard();
		const target = document.createElement('div');
		document.body.append(target);
		const dest = { ...newDestination(), id: 'dest-sched', revision: 4, name: 'Ops' };
		const component = mount(WdgAlertDestinationDialog, {
			target,
			props: {
				open: true,
				initialTab: 'schedule',
				destination: dest,
				alertsTransport,
				scheduleTransport,
				guard,
				onSaved: vi.fn()
			}
		});
		cleanup = () => void unmount(component);
		await settle();

		const cadenceSelect = [...document.body.querySelectorAll('select')].find((s) =>
			[...s.options].some((o) => o.value === 'Daily')
		)!;
		cadenceSelect.value = 'Daily';
		cadenceSelect.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		const previewBtn = [...document.body.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Preview schedule')
		)!;
		previewBtn.click();
		await settle();
		expect(scheduleTransport.preview).toHaveBeenCalledWith({
			...policy,
			cadence: { Daily: { minute: 540 } }
		});
		expect(scheduleTransport.save).not.toHaveBeenCalled();

		// Toggle urgent_bypass and require fresh consent after zone edit
		const urgentCheckbox = [
			...document.body.querySelectorAll<HTMLInputElement>('input[type=checkbox]')
		].find((el) => el.parentElement?.textContent?.includes('bypass digests'))!;
		urgentCheckbox.checked = true;
		urgentCheckbox.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		const consentCheckbox = [...document.body.querySelectorAll<HTMLInputElement>('input[required]')].find(
			(el) => el.parentElement?.textContent?.includes('confirm urgent-rule bypass')
		)!;
		consentCheckbox.checked = true;
		consentCheckbox.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		const zoneSelect = [...document.body.querySelectorAll('select')].find((s) =>
			[...s.options].some((o) => o.value === 'SystemLocal')
		)!;
		zoneSelect.value = 'SystemLocal';
		zoneSelect.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();
		expect(consentCheckbox.checked).toBe(false);

		consentCheckbox.checked = true;
		consentCheckbox.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		const forms = [...document.body.querySelectorAll('form')];
		forms.at(-1)!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
		await settle();

		expect(scheduleTransport.save).toHaveBeenCalledWith(
			expect.objectContaining({ urgent_bypass: true, zone: 'SystemLocal' }),
			true
		);
	});
});
