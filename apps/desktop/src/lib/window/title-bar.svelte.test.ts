import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { i18n } from '$lib/i18n/index.svelte';
import TitleBar from './TitleBar.svelte';
import { createFakeWindowTransport } from './transport';
import type { WindowTransport } from './types';

function renderTitleBar(transport: WindowTransport) {
	const target = document.createElement('div');
	document.body.appendChild(target);
	const instance = mount(TitleBar, {
		target,
		props: { transport }
	});
	flushSync();
	return {
		target,
		destroy() {
			unmount(instance);
			target.remove();
		}
	};
}

async function flushMicrotasks() {
	await Promise.resolve();
	await Promise.resolve();
	flushSync();
}

describe('TitleBar', () => {
	let cleanupFn: (() => void) | null = null;

	beforeEach(() => {
		i18n.setLocale('en');
	});

	afterEach(() => {
		cleanupFn?.();
		cleanupFn = null;
		document.body.innerHTML = '';
		i18n.setLocale('en');
	});

	it('renders nothing when transport.isAvailable() is false', () => {
		const transport = createFakeWindowTransport({ available: false });
		const view = renderTitleBar(transport);
		cleanupFn = () => view.destroy();

		expect(view.target.querySelector('header')).toBeNull();
	});

	it('renders static title and localized EN/PT-BR controls when available', async () => {
		const transport = createFakeWindowTransport({
			available: true,
			initialState: { maximized: false, focused: true }
		});
		const view = renderTitleBar(transport);
		cleanupFn = () => view.destroy();
		await flushMicrotasks();

		const header = view.target.querySelector('header');
		expect(header).not.toBeNull();
		expect(header?.getAttribute('aria-label')).toBe('triage-mail');
		expect(header?.textContent).toContain('triage-mail');

		expect(view.target.querySelector('button[aria-label="Minimize"]')).not.toBeNull();
		expect(view.target.querySelector('button[aria-label="Maximize"]')).not.toBeNull();
		expect(view.target.querySelector('button[aria-label="Close"]')).not.toBeNull();

		i18n.setLocale('pt-BR');
		flushSync();

		expect(view.target.querySelector('button[aria-label="Minimizar"]')).not.toBeNull();
		expect(view.target.querySelector('button[aria-label="Maximizar"]')).not.toBeNull();
		expect(view.target.querySelector('button[aria-label="Fechar"]')).not.toBeNull();
	});

	it('dispatches minimize, toggleMaximize, and close actions', async () => {
		const transport = createFakeWindowTransport({
			available: true,
			initialState: { maximized: false, focused: true }
		});
		const view = renderTitleBar(transport);
		cleanupFn = () => view.destroy();
		await flushMicrotasks();

		const minBtn = view.target.querySelector('button[aria-label="Minimize"]') as HTMLButtonElement;
		minBtn.click();
		await flushMicrotasks();
		expect(transport.calls.minimize).toBe(1);

		const maxBtn = view.target.querySelector('button[aria-label="Maximize"]') as HTMLButtonElement;
		expect(maxBtn.getAttribute('aria-pressed')).toBe('false');
		maxBtn.click();
		await flushMicrotasks();
		expect(transport.calls.toggleMaximize).toBe(1);

		const restoreBtn = view.target.querySelector(
			'button[aria-label="Restore"]'
		) as HTMLButtonElement;
		expect(restoreBtn).not.toBeNull();
		expect(restoreBtn.getAttribute('aria-pressed')).toBe('true');

		const closeBtn = view.target.querySelector('button[aria-label="Close"]') as HTMLButtonElement;
		closeBtn.click();
		await flushMicrotasks();
		expect(transport.calls.close).toBe(1);
	});

	it('starts dragging on primary pointerdown in drag region, but ignores button clicks and secondary buttons', async () => {
		const transport = createFakeWindowTransport({ available: true });
		const view = renderTitleBar(transport);
		cleanupFn = () => view.destroy();
		await flushMicrotasks();

		const dragRegion = view.target.querySelector('[data-tauri-drag-region]') as HTMLElement;
		expect(dragRegion).not.toBeNull();

		dragRegion.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 2 }));
		await flushMicrotasks();
		expect(transport.calls.startDragging).toBe(0);

		const minBtn = view.target.querySelector('button[aria-label="Minimize"]') as HTMLButtonElement;
		minBtn.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0 }));
		await flushMicrotasks();
		expect(transport.calls.startDragging).toBe(0);

		dragRegion.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0 }));
		await flushMicrotasks();
		expect(transport.calls.startDragging).toBe(1);

		dragRegion.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
		await flushMicrotasks();
		expect(transport.calls.toggleMaximize).toBe(1);
	});

	it('updates state from live subscription, mutes title when unfocused, and cleans up on unmount', async () => {
		const transport = createFakeWindowTransport({
			available: true,
			initialState: { maximized: false, focused: true }
		});
		const view = renderTitleBar(transport);
		cleanupFn = () => view.destroy();
		await flushMicrotasks();

		const titleSpan = view.target.querySelector('span[data-tauri-drag-region]') as HTMLElement;
		expect(titleSpan.className).not.toContain('opacity-70');

		transport.emitState({ focused: false, maximized: true });
		flushSync();

		expect(titleSpan.className).toContain('opacity-70');
		expect(view.target.querySelector('button[aria-label="Restore"]')).not.toBeNull();

		view.destroy();
		cleanupFn = null;
		expect(transport.wasUnsubscribed()).toBe(true);
	});

	it('surfaces localized actionError without flipping maximized state when toggleMaximize rejects', async () => {
		const transport = createFakeWindowTransport({
			available: true,
			initialState: { maximized: false, focused: true },
			failAction: 'toggleMaximize'
		});
		const view = renderTitleBar(transport);
		cleanupFn = () => view.destroy();
		await flushMicrotasks();

		const maxBtn = view.target.querySelector('button[aria-label="Maximize"]') as HTMLButtonElement;
		maxBtn.click();
		await flushMicrotasks();

		const status = view.target.querySelector('[role="status"]');
		expect(status).not.toBeNull();
		expect(status?.textContent).toContain('Window action unavailable.');

		const stillMaxBtn = view.target.querySelector(
			'button[aria-label="Maximize"]'
		) as HTMLButtonElement;
		expect(stillMaxBtn).not.toBeNull();
		expect(stillMaxBtn.getAttribute('aria-pressed')).toBe('false');
	});
});
