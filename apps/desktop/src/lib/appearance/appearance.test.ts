import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { visibleRange, selectionTop } from '$lib/inbox/virtual';
import { sanitizeEmailHtml } from '$lib/security/sanitize';
import { appStore } from '$lib/stores/app.svelte';
import {
	DENSITY_ROW_HEIGHT,
	getFontCssStack,
	getReaderPaletteColors,
	normalizeAppearance,
	PALETTE_CATALOG,
	resolveEffectiveTheme,
	stepUiZoom
} from './palettes';

describe('appearance preferences and helpers', () => {
	beforeEach(() => {
		localStorage.clear();
		appStore.resetAppearance();
	});

	afterEach(() => {
		appStore.resetAppearance();
		vi.restoreAllMocks();
	});

	it('normalizes malformed payloads and falls back safely', () => {
		const normalized = normalizeAppearance(
			{
				scheme: 'invalid' as never,
				palette: 'nord',
				textScale: 999 as never,
				uiZoom: 120,
				density: 'compact',
				underlineLinks: true
			},
			'dark'
		);
		expect(normalized).toEqual({
			scheme: 'dark',
			palette: 'nord',
			font: 'default',
			textScale: 100,
			uiZoom: 120,
			density: 'compact',
			underlineLinks: true,
			reduceMotion: false
		});
	});

	it('resolves system scheme against OS preference and steps UI zoom within bounds', () => {
		expect(resolveEffectiveTheme('system', true)).toBe('dark');
		expect(resolveEffectiveTheme('system', false)).toBe('light');
		expect(stepUiZoom(100, 1)).toBe(110);
		expect(stepUiZoom(125, 1)).toBe(125);
		expect(stepUiZoom(80, -1)).toBe(80);
	});

	it('applies palette, font, text scale, zoom, contrast, and accessibility attributes to documentElement', () => {
		appStore.setScheme('dark');
		appStore.setPalette('contrast');
		appStore.setFont('legible');
		appStore.setTextScale(125);
		appStore.setUiZoom(110);
		appStore.setDensity('compact');
		appStore.setUnderlineLinks(true);
		appStore.setReduceMotion(true);

		const root = document.documentElement;
		expect(appStore.theme).toBe('dark');
		expect(appStore.rowHeight).toBe(DENSITY_ROW_HEIGHT.compact);
		expect(root.classList.contains('dark')).toBe(true);
		expect(root.dataset.palette).toBe('contrast');
		expect(root.dataset.contrast).toBe('high');
		expect(root.dataset.font).toBe('legible');
		expect(root.dataset.textScale).toBe('125');
		expect(root.style.fontSize).toBe('125%');
		expect(root.dataset.uiZoom).toBe('110');
		expect(root.style.getPropertyValue('--ui-zoom')).toBe('1.1');
		expect(root.dataset.density).toBe('compact');
		expect(root.dataset.underlineLinks).toBe('true');
		expect(root.dataset.reduceMotion).toBe('true');

		const persisted = JSON.parse(localStorage.getItem('triage_mail_appearance_v1') ?? '{}');
		expect(persisted.palette).toBe('contrast');
		expect(persisted.uiZoom).toBe(110);

		appStore.zoomOut();
		expect(appStore.appearance.uiZoom).toBe(100);
		appStore.zoomIn();
		expect(appStore.appearance.uiZoom).toBe(110);
		appStore.resetZoom();
		expect(appStore.appearance.uiZoom).toBe(100);
	});

	it('propagates palette colors, font stack, and text scale into sanitized reader HTML', () => {
		const colors = getReaderPaletteColors('nord', 'dark');
		const output = sanitizeEmailHtml('<p>Body</p>', {
			theme: 'dark',
			textScale: 125,
			fontFamily: getFontCssStack('serif'),
			paletteColors: colors
		});
		expect(output).toContain(`--reader-bg: ${colors.bg}`);
		expect(output).toContain('font-size: 18px;');
		expect(output).toContain('Charter');
		expect(PALETTE_CATALOG.length).toBe(12);
	});

	it('computes virtualized range and selection scroll using custom density row heights', () => {
		const range = visibleRange(100, 720, 360, DENSITY_ROW_HEIGHT.compact);
		expect(range.total).toBe(7200);
		expect(range.start).toBe(5);
		expect(selectionTop(10, 0, 500, DENSITY_ROW_HEIGHT.compact)).toBe(292);
	});
});
