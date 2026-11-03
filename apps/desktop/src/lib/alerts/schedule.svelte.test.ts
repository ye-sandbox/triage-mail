import { mount, unmount, flushSync } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import ScheduleEditor from './ScheduleEditor.svelte';
import { type ScheduleTransport, type SchedulePolicy } from './schedule';
import { SettingsFormGuard } from '../settings/guards';
import { i18n } from '../i18n/index.svelte';
let cleanup: (() => void) | undefined;
afterEach(() => {
	cleanup?.();
	document.body.innerHTML = '';
	i18n.locale = 'en';
});
async function settle() {
	for (let i = 0; i < 20; i++) await Promise.resolve();
	flushSync();
}
function fixture() {
	const policy: SchedulePolicy = {
		destination_id: 'd',
		destination_revision: 4,
		revision: 2,
		cadence: 'Immediate',
		zone: 'Utc',
		quiet: null,
		urgent_bypass: false
	};
	const transport: ScheduleTransport = {
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
	const target = document.createElement('div');
	document.body.append(target);
	const guard = new SettingsFormGuard(),
		cancel = vi.fn();
	const component = mount(ScheduleEditor, {
		target,
		props: { id: 'd', transport, guard, cancel }
	});
	cleanup = () => void unmount(component);
	const button = (label: string) =>
		[...target.querySelectorAll('button')].find((b) => b.textContent?.includes(label))!;
	return { target, guard, transport, cancel, policy, button };
}
it('previews without saving and binds policy writes to both revisions', async () => {
	const f = fixture();
	await settle();
	expect(f.transport.preview).not.toHaveBeenCalled();
	expect(f.transport.save).not.toHaveBeenCalled();
	const cadence = f.target.querySelector('select')!;
	cadence.value = 'Daily';
	cadence.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	expect(f.guard.dirty).toBe(true);
	f.button('Preview schedule').click();
	await settle();
	expect(f.transport.preview).toHaveBeenCalledWith({
		...f.policy,
		cadence: { Daily: { minute: 540 } }
	});
	expect(f.target.textContent).toContain('Next eligible delivery');
	f.target
		.querySelector('form')!
		.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(f.transport.save).toHaveBeenCalledWith(
		{ ...f.policy, cadence: { Daily: { minute: 540 } } },
		false
	);
	expect(f.cancel).toHaveBeenCalled();
	expect(f.guard.dirty).toBe(false);
});
it('requires urgent consent again after editing the policy', async () => {
	const f = fixture();
	await settle();
	const urgent = f.target.querySelectorAll<HTMLInputElement>('input[type=checkbox]')[1];
	urgent.checked = true;
	urgent.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	const form = f.target.querySelector('form')!;
	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(f.transport.save).not.toHaveBeenCalled();
	const consent = f.target.querySelector<HTMLInputElement>('input[required][type=checkbox]')!;
	consent.checked = true;
	consent.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	const zone = f.target.querySelectorAll('select')[1];
	zone.value = 'SystemLocal';
	zone.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	expect(consent.checked).toBe(false);
	consent.checked = true;
	consent.dispatchEvent(new Event('change', { bubbles: true }));
	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	await settle();
	expect(f.transport.save).toHaveBeenCalledWith(
		expect.objectContaining({ urgent_bypass: true, zone: 'SystemLocal' }),
		true
	);
});
it('has Portuguese disclosure and ignores a preview after unmount', async () => {
	i18n.locale = 'pt-BR';
	const f = fixture();
	await settle();
	expect(f.target.textContent).toContain('Resumos e horário de silêncio');
	expect(f.target.textContent).toContain('24 horas');
	let resolve: (value: { next_epoch_ms: number; timezone: string }) => void = () => {};
	vi.mocked(f.transport.preview).mockImplementation(
		() =>
			new Promise((done) => {
				resolve = done;
			})
	);
	f.button('Prévia da programação').click();
	flushSync();
	expect(f.guard.busy).toBe(true);
	cleanup?.();
	cleanup = undefined;
	await settle();
	resolve({ next_epoch_ms: 0, timezone: 'UTC' });
	await settle();
	expect(f.guard.busy).toBe(false);
	expect(f.guard.dirty).toBe(false);
	expect(f.cancel).not.toHaveBeenCalled();
});
