import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SettingsDialog from './SettingsDialog.svelte';
import { SettingsFormGuard } from './guards';
import { i18n } from '../i18n/index.svelte';
import type { Settings, SettingsTransport } from './types';

function createMockSettingsTransport(overrides?: Partial<Settings>): SettingsTransport {
	const currentSettings: Settings = {
		ai: {
			enabled: false,
			provider: 'Ollama',
			endpoint: 'http://127.0.0.1:11434',
			model: 'llama3'
		},
		tokenizer: 'Unicode61RemoveDiacritics',
		has_api_key: false,
		model_cached: false,
		experimental_semantic_search: false,
		...overrides
	};

	return {
		get: vi.fn(async () => ({ ...currentSettings })),
		save: vi.fn(async () => {}),
		tokenizer: vi.fn(async () => {}),
		setExperimentalSemanticSearch: vi.fn(async () => {}),
		download: vi.fn(async () => {}),
		summarize: vi.fn(async () => 'Mock summary')
	};
}

describe('Settings modal form guards, localization, and flow tests', () => {
	let cleanupFns: Array<() => void> = [];

	afterEach(() => {
		for (const cleanup of cleanupFns) {
			cleanup();
		}
		cleanupFns = [];
		document.body.innerHTML = '';
		i18n.locale = 'en';
	});

	it('intercepts close when form is marked dirty and shows inline prompt', async () => {
		const close = vi.fn();
		const guard = new SettingsFormGuard();
		const transport = createMockSettingsTransport();

		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(SettingsDialog, {
			target,
			props: {
				open: true,
				close,
				transport,
				guard
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		// Await settings load
		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		// Mark form dirty
		guard.markDirty(true);

		// Click close button
		const closeBtn = target.querySelector<HTMLButtonElement>('button[aria-label="Close settings"]');
		closeBtn?.click();
		flushSync();

		// Close was NOT called directly; prompt is shown
		expect(close).not.toHaveBeenCalled();
		const prompt = target.querySelector('[role="alertdialog"]');
		expect(prompt).not.toBeNull();
		expect(prompt?.textContent).toContain('You have unsaved changes');

		// Click "Stay on form"
		const stayBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Stay on form')
		);
		stayBtn?.click();
		flushSync();

		expect(target.querySelector('[role="alertdialog"]')).toBeNull();
		expect(close).not.toHaveBeenCalled();

		// Trigger close again, now click "Discard changes"
		closeBtn?.click();
		flushSync();

		const discardBtn = [...target.querySelectorAll('button')].find((b) =>
			b.textContent?.includes('Discard changes')
		);
		discardBtn?.click();
		flushSync();

		expect(close).toHaveBeenCalledTimes(1);
	});

	it('blocks close and navigation while operation is marked busy', async () => {
		const close = vi.fn();
		const guard = new SettingsFormGuard();
		const transport = createMockSettingsTransport();

		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(SettingsDialog, {
			target,
			props: {
				open: true,
				close,
				transport,
				guard
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		// Mark busy
		guard.setBusy(true);

		// Try closing via Escape
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
		flushSync();

		expect(close).not.toHaveBeenCalled();
		expect(target.querySelector('[role="alertdialog"]')).toBeNull();

		// Unset busy
		guard.setBusy(false);
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
		flushSync();

		expect(close).toHaveBeenCalledTimes(1);
	});

	it('renders localized EN and PT-BR copy', async () => {
		const close = vi.fn();
		const transport = createMockSettingsTransport();

		i18n.locale = 'pt-BR';
		const target = document.createElement('div');
		document.body.appendChild(target);

		const instance = mount(SettingsDialog, {
			target,
			props: {
				open: true,
				close,
				transport
			}
		});
		flushSync();
		cleanupFns.push(() => {
			unmount(instance);
			target.remove();
		});

		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		// Header title in PT-BR
		expect(target.querySelector('#settings-dialog-title')?.textContent).toBe('Configurações');

		// Navigation tabs in PT-BR
		expect(target.textContent).toContain('Geral');
		expect(target.textContent).toContain('Contas');
		expect(target.textContent).toContain('Busca');
		expect(target.textContent).toContain('IA');
		expect(target.textContent).toContain('Privacidade');
	});
});
