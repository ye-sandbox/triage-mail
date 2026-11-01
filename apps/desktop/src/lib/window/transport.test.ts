import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mockWindow = {
	isMaximized: vi.fn<() => Promise<boolean>>(),
	isFocused: vi.fn<() => Promise<boolean>>(),
	startDragging: vi.fn<() => Promise<void>>(),
	minimize: vi.fn<() => Promise<void>>(),
	toggleMaximize: vi.fn<() => Promise<void>>(),
	close: vi.fn<() => Promise<void>>(),
	onResized: vi.fn<(handler: () => void) => Promise<() => void>>(),
	onFocusChanged: vi.fn<(handler: (event: { payload: boolean }) => void) => Promise<() => void>>()
};

vi.mock('@tauri-apps/api/window', () => ({
	getCurrentWindow: () => mockWindow
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

		const unsub = transport.subscribeState(() => {});
		expect(() => unsub()).not.toThrow();
		expect(mockWindow.isMaximized).not.toHaveBeenCalled();
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
	});

	it('cleans up late-resolving Tauri listeners if unsubscribed early', async () => {
		(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

		let resolveResize!: (unlisten: () => void) => void;
		let resolveFocus!: (unlisten: () => void) => void;
		const unlistenResize = vi.fn();
		const unlistenFocus = vi.fn();

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

		const transport = createTauriWindowTransport();
		const unsubscribe = transport.subscribeState(() => {});
		unsubscribe();

		resolveResize(unlistenResize);
		resolveFocus(unlistenFocus);
		await Promise.resolve();
		await Promise.resolve();

		expect(unlistenResize).toHaveBeenCalledTimes(1);
		expect(unlistenFocus).toHaveBeenCalledTimes(1);
	});
});

describe('createFakeWindowTransport', () => {
	it('tracks calls, emits state updates, and supports late listener cleanup', async () => {
		const fake = createFakeWindowTransport({
			initialState: { maximized: false, focused: true },
			lateSubscribe: true
		});

		const observed: Array<{ maximized: boolean; focused: boolean }> = [];
		const unsub = fake.subscribeState((s) => observed.push(s));
		unsub();
		await fake.resolveLateListeners();

		expect(fake.wasUnsubscribed()).toBe(true);
		fake.emitState({ maximized: true });
		expect(observed).toHaveLength(0);
	});
});
