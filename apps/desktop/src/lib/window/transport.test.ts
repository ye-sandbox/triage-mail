import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { mockListen } = vi.hoisted(() => ({
	mockListen: vi.fn<
		(event: string, handler: (event: { payload: unknown }) => void) => Promise<() => void>
	>()
}));

const mockWindow = {
	isMaximized: vi.fn<() => Promise<boolean>>(),
	isFocused: vi.fn<() => Promise<boolean>>(),
	startDragging: vi.fn<() => Promise<void>>(),
	minimize: vi.fn<() => Promise<void>>(),
	toggleMaximize: vi.fn<() => Promise<void>>(),
	close: vi.fn<() => Promise<void>>(),
	hide: vi.fn<() => Promise<void>>(),
	onResized: vi.fn<(handler: () => void) => Promise<() => void>>(),
	onFocusChanged: vi.fn<(handler: (event: { payload: boolean }) => void) => Promise<() => void>>()
};

vi.mock('@tauri-apps/api/window', () => ({
	getCurrentWindow: () => mockWindow
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: mockListen
}));

import {
	createFakeWindowTransport,
	createTauriWindowTransport,
	isNativeTauriWindow
} from './transport';
import { getWindowLabels } from './types';

describe('window types and labels', () => {
	it('returns localized EN and PT-BR labels with static appTitle', () => {
		const en = getWindowLabels('en');
		expect(en).toEqual({
			appTitle: 'triage-mail',
			minimize: 'Minimize',
			maximize: 'Maximize',
			restore: 'Restore',
			close: 'Close',
			actionError: 'Window action unavailable.'
		});

		const pt = getWindowLabels('pt-BR');
		expect(pt).toEqual({
			appTitle: 'triage-mail',
			minimize: 'Minimizar',
			maximize: 'Maximizar',
			restore: 'Restaurar',
			close: 'Fechar',
			actionError: 'Ação da janela indisponível.'
		});
	});
});

describe('createTauriWindowTransport', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
	});

	afterEach(() => {
		delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
	});

	it('falls back safely in browser mode when __TAURI_INTERNALS__ is absent', async () => {
		expect(isNativeTauriWindow()).toBe(false);
		const transport = createTauriWindowTransport();
		expect(transport.isAvailable()).toBe(false);
		await expect(transport.getState()).resolves.toEqual({ maximized: false, focused: true });
		await expect(transport.startDragging()).resolves.toBeUndefined();
		await expect(transport.minimize()).resolves.toBeUndefined();
		await expect(transport.toggleMaximize()).resolves.toBe(false);
		await expect(transport.close()).resolves.toBeUndefined();
		await expect(transport.hide!()).resolves.toBeUndefined();

		const unsub = transport.subscribeState(() => {});
		const unsubTray = transport.subscribeTrayNavigate!(() => {});
		expect(() => unsub()).not.toThrow();
		expect(() => unsubTray()).not.toThrow();
		expect(mockWindow.isMaximized).not.toHaveBeenCalled();
		expect(mockListen).not.toHaveBeenCalled();
	});

	it('dispatches native window actions when __TAURI_INTERNALS__ is present', async () => {
		(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
		expect(isNativeTauriWindow()).toBe(true);

		mockWindow.isMaximized.mockResolvedValueOnce(false).mockResolvedValueOnce(true);
		mockWindow.isFocused.mockResolvedValueOnce(true);
		mockWindow.startDragging.mockResolvedValueOnce();
		mockWindow.minimize.mockResolvedValueOnce();
		mockWindow.toggleMaximize.mockResolvedValueOnce();
		mockWindow.close.mockResolvedValueOnce();
		mockWindow.hide.mockResolvedValueOnce();

		const transport = createTauriWindowTransport();
		expect(transport.isAvailable()).toBe(true);
		await expect(transport.getState()).resolves.toEqual({ maximized: false, focused: true });
		await transport.startDragging();
		expect(mockWindow.startDragging).toHaveBeenCalledTimes(1);

		await transport.minimize();
		expect(mockWindow.minimize).toHaveBeenCalledTimes(1);

		await expect(transport.toggleMaximize()).resolves.toBe(true);
		expect(mockWindow.toggleMaximize).toHaveBeenCalledTimes(1);

		await transport.close();
		expect(mockWindow.close).toHaveBeenCalledTimes(1);

		await transport.hide!();
		expect(mockWindow.hide).toHaveBeenCalledTimes(1);
	});

	it('cleans up late-resolving Tauri listeners if unsubscribed early', async () => {
		(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

		let resolveResize!: (unlisten: () => void) => void;
		let resolveFocus!: (unlisten: () => void) => void;
		let resolveTray!: (unlisten: () => void) => void;
		const unlistenResize = vi.fn();
		const unlistenFocus = vi.fn();
		const unlistenTray = vi.fn();

		mockWindow.onResized.mockReturnValueOnce(
			new Promise((r) => {
				resolveResize = r;
			})
		);
		mockWindow.onFocusChanged.mockReturnValueOnce(
			new Promise((r) => {
				resolveFocus = r;
			})
		);
		mockListen.mockReturnValueOnce(
			new Promise((r) => {
				resolveTray = r;
			})
		);

		const transport = createTauriWindowTransport();
		const unsubscribe = transport.subscribeState(() => {});
		const unsubscribeTray = transport.subscribeTrayNavigate!(() => {});
		unsubscribe();
		unsubscribeTray();

		resolveResize(unlistenResize);
		resolveFocus(unlistenFocus);
		resolveTray(unlistenTray);
		await Promise.resolve();
		await Promise.resolve();

		expect(unlistenResize).toHaveBeenCalledTimes(1);
		expect(unlistenFocus).toHaveBeenCalledTimes(1);
		expect(unlistenTray).toHaveBeenCalledTimes(1);
	});

	it('forwards tray://navigate events while active', async () => {
		(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

		let handler!: (event: { payload: unknown }) => void;
		const unlisten = vi.fn();
		mockListen.mockImplementationOnce(async (_event, cb) => {
			handler = cb;
			return unlisten;
		});

		const routes: string[] = [];
		const transport = createTauriWindowTransport();
		const unsubscribe = transport.subscribeTrayNavigate!((route) => routes.push(route));
		await Promise.resolve();

		handler({ payload: '/due-dates' });
		expect(routes).toEqual(['/due-dates']);

		unsubscribe();
		expect(unlisten).toHaveBeenCalledTimes(1);
		handler({ payload: '/' });
		expect(routes).toEqual(['/due-dates']);
	});
});

describe('createFakeWindowTransport', () => {
	it('tracks calls, emits state and tray updates, and supports late listener cleanup', async () => {
		const fake = createFakeWindowTransport({
			initialState: { maximized: false, focused: true },
			lateSubscribe: true
		});

		const observed: Array<{ maximized: boolean; focused: boolean }> = [];
		const trayRoutes: string[] = [];
		const unsub = fake.subscribeState((s) => observed.push(s));
		const unsubTray = fake.subscribeTrayNavigate!((route) => trayRoutes.push(route));
		unsub();
		unsubTray();
		await fake.resolveLateListeners();

		expect(fake.wasUnsubscribed()).toBe(true);
		fake.emitState({ maximized: true });
		fake.emitTrayNavigate('/due-dates');
		expect(observed).toHaveLength(0);
		expect(trayRoutes).toHaveLength(0);
	});
});
