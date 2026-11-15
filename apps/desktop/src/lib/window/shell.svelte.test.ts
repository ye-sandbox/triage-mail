import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { i18n } from '$lib/i18n/index.svelte';
import { appStore } from '$lib/stores/app.svelte';
import Layout from '../../routes/+layout.svelte';
import { createFakeWindowTransport } from './transport';

async function flushMicrotasks() {
	await Promise.resolve();
	await Promise.resolve();
	flushSync();
}

describe('UI v2 Desktop Shell Layout (+layout.svelte / AppShell)', () => {
	let container: HTMLDivElement;
	let instance: Record<string, unknown> | null = null;

	beforeEach(() => {
		localStorage.clear();
		i18n.setLocale('en');
		appStore.setTheme('light');
		appStore.sidebarOpen = true;
		appStore.mobileDrawerOpen = false;
		appStore.commandOpen = false;
		appStore.overdueAndDueTodayCount = 0;
		container = document.createElement('div');
		document.body.appendChild(container);
	});

	afterEach(() => {
		if (instance) {
			unmount(instance);
			instance = null;
		}
		container.remove();
		i18n.setLocale('en');
	});

	it('renders .desktop-shell with persistent AppShell header above .desktop-shell-content when native window is available', async () => {
		const transport = createFakeWindowTransport({ available: true });
		const children = createRawSnippet(() => ({
			render: () => '<div data-testid="child-route">Unified Inbox Content</div>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		await flushMicrotasks();

		const shell = container.querySelector('.desktop-shell');
		expect(shell).not.toBeNull();

		const header = shell?.querySelector('header');
		const content = shell?.querySelector('.desktop-shell-content');
		expect(header).not.toBeNull();
		expect(content).not.toBeNull();
		expect(header?.getAttribute('aria-label')).toBe('triage-mail');
		expect(header?.nextElementSibling).toBe(content);

		const routeContent = content?.querySelector('[data-testid="child-route"]');
		expect(routeContent?.textContent).toBe('Unified Inbox Content');
		expect(shell?.querySelector('nav[aria-label="Workspace navigation"]')).not.toBeNull();
	});

	it('provides browser workstation dock fallback for minimize and close when available is false', async () => {
		const transport = createFakeWindowTransport({ available: false });
		const children = createRawSnippet(() => ({
			render: () => '<div data-testid="browser-route">Browser Preview Content</div>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		await flushMicrotasks();

		const shell = container.querySelector('.desktop-shell');
		expect(shell).not.toBeNull();
		expect(shell?.querySelector('header')).not.toBeNull();
		expect(shell?.querySelector('[data-testid="browser-route"]')?.textContent).toBe(
			'Browser Preview Content'
		);

		const minBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Minimize"]');
		minBtn?.click();
		await flushMicrotasks();

		expect(transport.calls.minimize).toBe(0);
		expect(container.textContent).toContain('Minimized to dock');

		const restoreDockBtn = Array.from(container.querySelectorAll('button')).find((b) =>
			b.textContent?.includes('Restore window')
		);
		expect(restoreDockBtn).toBeDefined();
		restoreDockBtn?.click();
		await flushMicrotasks();
		expect(container.querySelector('[data-testid="browser-route"]')).not.toBeNull();

		const closeBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Close"]');
		closeBtn?.click();
		await flushMicrotasks();

		expect(transport.calls.close).toBe(0);
		expect(container.textContent).toContain('Workstation window closed');

		const reopenBtn = Array.from(container.querySelectorAll('button')).find((b) =>
			b.textContent?.includes('Reopen triage-mail')
		);
		expect(reopenBtn).toBeDefined();
		reopenBtn?.click();
		await flushMicrotasks();
		expect(container.querySelector('[data-testid="browser-route"]')).not.toBeNull();
	});

	it('keeps WindowControls accessible while child modal/dialog is active and never closes window on Escape', async () => {
		const transport = createFakeWindowTransport({ available: true });
		const children = createRawSnippet(() => ({
			render: () =>
				'<div role="dialog" aria-modal="true" data-testid="child-dialog"><button type="button" data-testid="dialog-btn">Dialog Action</button></div>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		await flushMicrotasks();

		const dialog = container.querySelector('[data-testid="child-dialog"]');
		const dialogBtn = container.querySelector<HTMLButtonElement>('[data-testid="dialog-btn"]');
		expect(dialog).not.toBeNull();
		expect(dialogBtn).not.toBeNull();

		dialogBtn?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
		dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
		flushSync();

		expect(transport.calls.close).toBe(0);

		const minimizeBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Minimize"]');
		const maximizeBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Maximize"]');
		const closeBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Close"]');

		expect(minimizeBtn).not.toBeNull();
		expect(maximizeBtn).not.toBeNull();
		expect(closeBtn).not.toBeNull();

		minimizeBtn?.click();
		maximizeBtn?.click();
		await flushMicrotasks();

		expect(transport.calls.minimize).toBe(1);
		expect(transport.calls.toggleMaximize).toBe(1);
		expect(transport.calls.close).toBe(0);

		const restoreBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Restore"]');
		expect(restoreBtn?.getAttribute('aria-pressed')).toBe('true');

		closeBtn?.click();
		await flushMicrotasks();
		expect(transport.calls.close).toBe(1);
	});

	it('wires drag region, live focus/maximize state, error surfacing, Ctrl+K, and unmount cleanup', async () => {
		const transport = createFakeWindowTransport({
			available: true,
			initialState: { maximized: false, focused: true }
		});
		const children = createRawSnippet(() => ({
			render: () => '<div data-testid="inbox-slot">Inbox</div>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		await flushMicrotasks();

		const dragRegion = container.querySelector<HTMLElement>(
			'div[role="presentation"][data-tauri-drag-region]'
		);
		expect(dragRegion).not.toBeNull();

		dragRegion?.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 2 }));
		await flushMicrotasks();
		expect(transport.calls.startDragging).toBe(0);

		dragRegion?.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0 }));
		dragRegion?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
		await flushMicrotasks();
		expect(transport.calls.startDragging).toBe(1);
		expect(transport.calls.toggleMaximize).toBe(1);

		const titleLink = container.querySelector<HTMLAnchorElement>('header a[href="/"]');
		expect(titleLink?.className).not.toContain('opacity-70');
		transport.emitState({ focused: false, maximized: true });
		flushSync();
		expect(titleLink?.className).toContain('opacity-70');

		i18n.setLocale('pt-BR');
		flushSync();
		expect(container.querySelector('button[aria-label="Minimizar"]')).not.toBeNull();
		expect(container.querySelector('button[aria-label="Restaurar"]')).not.toBeNull();
		expect(container.querySelector('button[aria-label="Fechar"]')).not.toBeNull();

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
		flushSync();
		expect(appStore.commandOpen).toBe(true);

		unmount(instance);
		instance = null;
		expect(transport.wasUnsubscribed()).toBe(true);
	});

	it('surfaces localized actionError when native window action rejects and displays due-dates badge', async () => {
		appStore.overdueAndDueTodayCount = 4;
		const transport = createFakeWindowTransport({
			available: true,
			initialState: { maximized: false, focused: true },
			failAction: 'toggleMaximize'
		});
		const children = createRawSnippet(() => ({
			render: () => '<div>Route</div>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		await flushMicrotasks();

		const badge = Array.from(container.querySelectorAll('span')).find(
			(el) => el.getAttribute('title') === 'Overdue + Due Today'
		);
		expect(badge?.textContent?.trim()).toBe('4');

		const maxBtn = container.querySelector<HTMLButtonElement>('button[aria-label="Maximize"]');
		maxBtn?.click();
		await flushMicrotasks();

		const status = container.querySelector('[role="status"]');
		expect(status?.textContent).toContain('Window action unavailable.');
		expect(container.querySelector('button[aria-label="Maximize"]')?.getAttribute('aria-pressed')).toBe(
			'false'
		);
	});
});
