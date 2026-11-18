import { t } from '$lib/i18n';
import { appStore } from '$lib/stores/app.svelte';
import { createTauriWindowTransport } from '$lib/window/transport';
import type { WindowTransport } from '$lib/window/types';

export type WindowMode = 'maximized' | 'restored' | 'minimized' | 'closed';

export class WindowController {
	mode = $state<WindowMode>('maximized');
	lastActiveMode = $state<'maximized' | 'restored'>('maximized');
	focused = $state<boolean>(true);
	isDragging = $state<boolean>(false);
	actionError = $state<string | null>(null);

	x = $state<number>(48);
	y = $state<number>(36);
	width = $state<number>(1120);
	height = $state<number>(720);
	private hasCustomBounds = false;
	private transport: WindowTransport = createTauriWindowTransport();

	connect(transport: WindowTransport = createTauriWindowTransport()): () => void {
		this.transport = transport;
		this.actionError = null;
		this.isDragging = false;
		this.hasCustomBounds = false;
		const initialMode = transport.isAvailable() ? 'restored' : 'maximized';
		this.mode = initialMode;
		this.lastActiveMode = initialMode;
		this.focused = true;
		if (!transport.isAvailable()) {
			return () => {
				this.isDragging = false;
			};
		}

		let cancelled = false;
		void transport
			.getState()
			.then((initial) => {
				if (!cancelled) {
					this.mode = initial.maximized ? 'maximized' : 'restored';
					this.lastActiveMode = initial.maximized ? 'maximized' : 'restored';
					this.focused = initial.focused;
				}
			})
			.catch(() => {});

		const unsubscribe = transport.subscribeState((next) => {
			if (!cancelled) {
				this.mode = next.maximized ? 'maximized' : 'restored';
				this.lastActiveMode = next.maximized ? 'maximized' : 'restored';
				this.focused = next.focused;
			}
		});

		return () => {
			cancelled = true;
			this.isDragging = false;
			unsubscribe();
		};
	}

	private computeDefaultBounds() {
		if (typeof window === 'undefined') return;
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		const w = vw < 640 ? Math.max(320, vw - 16) : Math.min(1200, Math.round(vw * 0.9));
		const h = vh < 560 ? Math.max(380, vh - 24) : Math.min(820, Math.round(vh * 0.88));
		this.width = w;
		this.height = h;
		this.x = Math.max(8, Math.round((vw - w) / 2));
		this.y = Math.max(8, Math.round((vh - h) / 2));
		this.hasCustomBounds = true;
	}

	private clampPosition(nextX: number, nextY: number): { x: number; y: number } {
		if (typeof window === 'undefined') return { x: nextX, y: nextY };
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		const minVisibleX = Math.min(180, this.width);
		const clampedX = Math.min(vw - minVisibleX, Math.max(-this.width + minVisibleX, nextX));
		const clampedY = Math.min(vh - 44, Math.max(0, nextY));
		return { x: clampedX, y: clampedY };
	}

	async startDrag(event: PointerEvent | MouseEvent) {
		if (event.button !== 0) return;
		if (event.target instanceof Element) {
			const button = event.target.closest('button');
			if (button && !button.hasAttribute('data-tauri-drag-region')) return;
		}

		this.actionError = null;
		if (this.transport.isAvailable()) {
			try {
				await this.transport.startDragging();
			} catch {
				this.actionError = t(appStore.locale, 'windowActionError');
			}
			return;
		}

		if (typeof window === 'undefined') return;

		const startClientX = event.clientX;
		const startClientY = event.clientY;
		let originX = this.x;
		let originY = this.y;
		const startedFromMaximized = this.mode === 'maximized';
		let dragActivated = !startedFromMaximized;

		if (dragActivated) {
			this.isDragging = true;
		}

		const onPointerMove = (moveEvent: PointerEvent) => {
			const dx = moveEvent.clientX - startClientX;
			const dy = moveEvent.clientY - startClientY;

			if (!dragActivated) {
				if (Math.hypot(dx, dy) < 5) return;
				dragActivated = true;
				this.isDragging = true;
				if (startedFromMaximized) {
					this.computeDefaultBounds();
					const ratioX = startClientX / Math.max(1, window.innerWidth);
					originX = Math.round(moveEvent.clientX - this.width * ratioX);
					originY = Math.max(0, moveEvent.clientY - 22);
					this.mode = 'restored';
					this.lastActiveMode = 'restored';
					const clamped = this.clampPosition(originX, originY);
					this.x = clamped.x;
					this.y = clamped.y;
					originX = this.x - dx;
					originY = this.y - dy;
					return;
				}
			}

			const clamped = this.clampPosition(originX + dx, originY + dy);
			this.x = clamped.x;
			this.y = clamped.y;
		};

		const onPointerUp = () => {
			this.isDragging = false;
			window.removeEventListener('pointermove', onPointerMove);
			window.removeEventListener('pointerup', onPointerUp);
			window.removeEventListener('pointercancel', onPointerUp);
		};

		window.addEventListener('pointermove', onPointerMove);
		window.addEventListener('pointerup', onPointerUp);
		window.addEventListener('pointercancel', onPointerUp);
	}

	async minimize() {
		this.actionError = null;
		if (this.transport.isAvailable()) {
			try {
				if (
					appStore.vaultUnlocked &&
					appStore.shellPreferences.minimize_to_tray &&
					this.transport.hide
				) {
					await this.transport.hide();
				} else {
					await this.transport.minimize();
				}
			} catch {
				this.actionError = t(appStore.locale, 'windowActionError');
			}
			return;
		}

		if (this.mode === 'maximized' || this.mode === 'restored') {
			this.lastActiveMode = this.mode;
		}
		this.mode = 'minimized';
	}

	async toggleMaximize() {
		this.actionError = null;
		if (this.transport.isAvailable()) {
			try {
				const isMax = await this.transport.toggleMaximize();
				this.mode = isMax ? 'maximized' : 'restored';
				this.lastActiveMode = this.mode;
			} catch {
				this.actionError = t(appStore.locale, 'windowActionError');
			}
			return;
		}

		if (this.mode === 'maximized') {
			if (!this.hasCustomBounds) {
				this.computeDefaultBounds();
			}
			this.mode = 'restored';
			this.lastActiveMode = 'restored';
		} else {
			this.mode = 'maximized';
			this.lastActiveMode = 'maximized';
		}
	}

	restoreFromDock() {
		if (this.lastActiveMode === 'restored' && !this.hasCustomBounds) {
			this.computeDefaultBounds();
		}
		this.mode = this.lastActiveMode;
	}

	async close() {
		this.actionError = null;
		if (this.transport.isAvailable()) {
			try {
				await this.transport.close();
			} catch {
				this.actionError = t(appStore.locale, 'windowActionError');
			}
			return;
		}

		if (this.mode === 'maximized' || this.mode === 'restored') {
			this.lastActiveMode = this.mode;
		}
		this.mode = 'closed';
	}

	reopen() {
		this.mode = this.lastActiveMode;
	}
}

export const windowController = new WindowController();
