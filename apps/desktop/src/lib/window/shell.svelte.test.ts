import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { i18n } from '$lib/i18n/index.svelte';
import Layout from '../../routes/+layout.svelte';
import { createFakeWindowTransport } from './transport';

describe('Desktop Shell Layout (+layout.svelte)', () => {
	let container: HTMLDivElement;
	let instance: Record<string, unknown> | null = null;

	beforeEach(() => {
		i18n.setLocale('en');
		container = document.createElement('div');
		document.body.appendChild(container);
	});

	afterEach(() => {
		if (instance) {
			unmount(instance);
			instance = null;
		}
		container.remove();
	});

	it('renders .desktop-shell with persistent TitleBar above .desktop-shell-content when native window is available', () => {
		const transport = createFakeWindowTransport({ available: true });
		const children = createRawSnippet(() => ({
			render: () => '<main data-testid="child-route">Unified Inbox Content</main>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		flushSync();

		const shell = container.querySelector('.desktop-shell');
		expect(shell).not.toBeNull();

		const shellChildren = Array.from(shell?.children ?? []);
		expect(shellChildren.length).toBe(2);
		expect(shellChildren[0]?.tagName.toLowerCase()).toBe('header');
		expect(shellChildren[0]?.getAttribute('aria-label')).toBe('triage-mail');
		expect(shellChildren[1]?.classList.contains('desktop-shell-content')).toBe(true);

		const routeContent = shellChildren[1]?.querySelector('[data-testid="child-route"]');
		expect(routeContent?.textContent).toBe('Unified Inbox Content');
	});

	it('renders .desktop-shell and .desktop-shell-content without TitleBar or window control buttons when available is false', () => {
		const transport = createFakeWindowTransport({ available: false });
		const children = createRawSnippet(() => ({
			render: () => '<main data-testid="browser-route">Browser Preview Content</main>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		flushSync();

		const shell = container.querySelector('.desktop-shell');
		expect(shell).not.toBeNull();
		expect(shell?.querySelector('header')).toBeNull();
		expect(shell?.querySelectorAll('button').length).toBe(0);

		const content = shell?.querySelector('.desktop-shell-content');
		expect(content).not.toBeNull();
		expect(content?.querySelector('[data-testid="browser-route"]')?.textContent).toBe(
			'Browser Preview Content'
		);
	});

	it('keeps TitleBar controls accessible while child modal/dialog content is active and never closes window on Escape', async () => {
		const transport = createFakeWindowTransport({ available: true });
		const children = createRawSnippet(() => ({
			render: () =>
				'<div role="dialog" aria-modal="true" data-testid="child-dialog"><button type="button" data-testid="dialog-btn">Dialog Action</button></div>'
		}));

		instance = mount(Layout, {
			target: container,
			props: { children, transport }
		});
		flushSync();

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
		await Promise.resolve();
		flushSync();

		expect(transport.calls.minimize).toBe(1);
		expect(transport.calls.toggleMaximize).toBe(1);
		expect(transport.calls.close).toBe(0);

		closeBtn?.click();
		await Promise.resolve();
		expect(transport.calls.close).toBe(1);
	});
});
