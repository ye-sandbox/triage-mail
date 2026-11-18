import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { WindowState, WindowTransport } from './types';

export function isNativeTauriWindow(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function createTauriWindowTransport(): WindowTransport {
	return {
		isAvailable() {
			return isNativeTauriWindow();
		},
		async getState(): Promise<WindowState> {
			if (!isNativeTauriWindow()) {
				return { maximized: false, focused: true };
			}
			const appWindow = getCurrentWindow();
			const [maximized, focused] = await Promise.all([
				appWindow.isMaximized(),
				appWindow.isFocused()
			]);
			return { maximized, focused };
		},
		async startDragging(): Promise<void> {
			if (!isNativeTauriWindow()) return;
			await getCurrentWindow().startDragging();
		},
		async minimize(): Promise<void> {
			if (!isNativeTauriWindow()) return;
			await getCurrentWindow().minimize();
		},
		async toggleMaximize(): Promise<boolean> {
			if (!isNativeTauriWindow()) return false;
			const appWindow = getCurrentWindow();
			await appWindow.toggleMaximize();
			return await appWindow.isMaximized();
		},
		async close(): Promise<void> {
			if (!isNativeTauriWindow()) return;
			await getCurrentWindow().close();
		},
		async hide(): Promise<void> {
			if (!isNativeTauriWindow()) return;
			await getCurrentWindow().hide();
		},
		subscribeState(onChange: (state: WindowState) => void): () => void {
			if (!isNativeTauriWindow()) {
				return () => {};
			}
			const appWindow = getCurrentWindow();
			let disposed = false;
			const unlisteners: Array<() => void> = [];

			const notify = async () => {
				if (disposed) return;
				try {
					const [maximized, focused] = await Promise.all([
						appWindow.isMaximized(),
						appWindow.isFocused()
					]);
					if (!disposed) {
						onChange({ maximized, focused });
					}
				} catch {
					// Ignore transient query errors during teardown
				}
			};

			// INVARIANT: If unsubscribe() runs before async listener registration finishes, immediately detach on resolution.
			const register = (promise: Promise<() => void>) => {
				void promise
					.then((unlisten) => {
						if (disposed) {
							unlisten();
						} else {
							unlisteners.push(unlisten);
						}
					})
					.catch(() => {});
			};

			register(
				appWindow.onResized(() => {
					void notify();
				})
			);
			register(
				appWindow.onFocusChanged(({ payload }) => {
					if (disposed) return;
					void appWindow
						.isMaximized()
						.then((maximized) => {
							if (!disposed) {
								onChange({ maximized, focused: payload });
							}
						})
						.catch(() => {});
				})
			);

			return () => {
				disposed = true;
				while (unlisteners.length > 0) {
					const unlisten = unlisteners.pop();
					unlisten?.();
				}
			};
		},
		subscribeTrayNavigate(onNavigate: (route: string) => void): () => void {
			if (!isNativeTauriWindow()) {
				return () => {};
			}
			let disposed = false;
			let activeUnlisten: (() => void) | null = null;
			void listen<string>('tray://navigate', (event) => {
				if (!disposed && typeof event.payload === 'string') {
					onNavigate(event.payload);
				}
			})
				.then((unlisten) => {
					if (disposed) {
						unlisten();
					} else {
						activeUnlisten = unlisten;
					}
				})
				.catch(() => {});

			return () => {
				disposed = true;
				activeUnlisten?.();
				activeUnlisten = null;
			};
		}
	};
}

export type FakeWindowAction = 'minimize' | 'toggleMaximize' | 'close' | 'hide' | 'startDragging';

export type FakeWindowTransportOptions = {
	available?: boolean;
	initialState?: WindowState;
	failAction?: FakeWindowAction;
	lateSubscribe?: boolean;
};

export type FakeWindowTransport = WindowTransport & {
	calls: {
		startDragging: number;
		minimize: number;
		toggleMaximize: number;
		close: number;
		hide: number;
		getState: number;
		subscribeState: number;
		subscribeTrayNavigate: number;
	};
	emitState(next: Partial<WindowState>): void;
	emitTrayNavigate(route: string): void;
	wasUnsubscribed(): boolean;
	resolveLateListeners(): Promise<void>;
};

export function createFakeWindowTransport(
	options: FakeWindowTransportOptions = {}
): FakeWindowTransport {
	const available = options.available ?? true;
	let state: WindowState = {
		maximized: options.initialState?.maximized ?? false,
		focused: options.initialState?.focused ?? true
	};
	const listeners = new Set<(next: WindowState) => void>();
	const trayListeners = new Set<(route: string) => void>();
	let unsubscribed = false;
	let lateSubscribedDisposed = false;
	const pendingRegistrations: Array<() => void> = [];

	const calls = {
		startDragging: 0,
		minimize: 0,
		toggleMaximize: 0,
		close: 0,
		hide: 0,
		getState: 0,
		subscribeState: 0,
		subscribeTrayNavigate: 0
	};

	const maybeFail = (action: FakeWindowAction) => {
		if (options.failAction === action) {
			throw new Error(`Simulated ${action} failure`);
		}
	};

	return {
		calls,
		isAvailable() {
			return available;
		},
		async getState(): Promise<WindowState> {
			calls.getState += 1;
			if (!available) {
				return { maximized: false, focused: true };
			}
			return { ...state };
		},
		async startDragging(): Promise<void> {
			calls.startDragging += 1;
			if (!available) return;
			maybeFail('startDragging');
		},
		async minimize(): Promise<void> {
			calls.minimize += 1;
			if (!available) return;
			maybeFail('minimize');
		},
		async toggleMaximize(): Promise<boolean> {
			calls.toggleMaximize += 1;
			if (!available) return false;
			maybeFail('toggleMaximize');
			state = { ...state, maximized: !state.maximized };
			for (const listener of listeners) {
				listener({ ...state });
			}
			return state.maximized;
		},
		async close(): Promise<void> {
			calls.close += 1;
			if (!available) return;
			maybeFail('close');
		},
		async hide(): Promise<void> {
			calls.hide += 1;
			if (!available) return;
			maybeFail('hide');
		},
		subscribeState(onChange: (next: WindowState) => void): () => void {
			calls.subscribeState += 1;
			if (!available) {
				return () => {};
			}
			if (options.lateSubscribe) {
				let localDisposed = false;
				pendingRegistrations.push(() => {
					if (localDisposed) {
						lateSubscribedDisposed = true;
					} else {
						listeners.add(onChange);
					}
				});
				return () => {
					localDisposed = true;
					unsubscribed = true;
					listeners.delete(onChange);
				};
			}
			listeners.add(onChange);
			return () => {
				unsubscribed = true;
				listeners.delete(onChange);
			};
		},
		subscribeTrayNavigate(onNavigate: (route: string) => void): () => void {
			calls.subscribeTrayNavigate += 1;
			if (!available) {
				return () => {};
			}
			if (options.lateSubscribe) {
				let localDisposed = false;
				pendingRegistrations.push(() => {
					if (localDisposed) {
						lateSubscribedDisposed = true;
					} else {
						trayListeners.add(onNavigate);
					}
				});
				return () => {
					localDisposed = true;
					unsubscribed = true;
					trayListeners.delete(onNavigate);
				};
			}
			trayListeners.add(onNavigate);
			return () => {
				unsubscribed = true;
				trayListeners.delete(onNavigate);
			};
		},
		emitState(next: Partial<WindowState>) {
			state = { ...state, ...next };
			for (const listener of listeners) {
				listener({ ...state });
			}
		},
		emitTrayNavigate(route: string) {
			for (const listener of trayListeners) {
				listener(route);
			}
		},
		wasUnsubscribed() {
			return unsubscribed || lateSubscribedDisposed;
		},
		async resolveLateListeners() {
			while (pendingRegistrations.length > 0) {
				const reg = pendingRegistrations.shift();
				reg?.();
			}
		}
	};
}
