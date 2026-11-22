import type { ThemeMode } from '$lib/types';
import type {
	AppearancePreferences,
	FontDefinition,
	FontPreset,
	ListDensity,
	PaletteDefinition,
	ReaderPaletteColors,
	TextScale,
	ThemePalette,
	ThemeScheme,
	UiZoom
} from './types';

export const THEME_SCHEMES: readonly ThemeScheme[] = ['light', 'dark', 'system'];

export const THEME_PALETTES: readonly ThemePalette[] = [
	'default',
	'nord',
	'dracula',
	'catppuccin',
	'solarized',
	'gruvbox',
	'tokyonight',
	'onedark',
	'github',
	'rosepine',
	'contrast',
	'colorblind'
];

export const FONT_PRESETS: readonly FontPreset[] = [
	'default',
	'system',
	'modern',
	'serif',
	'rounded',
	'mono',
	'legible'
];

export const TEXT_SCALES: readonly TextScale[] = [90, 100, 112, 125];

export const UI_ZOOMS: readonly UiZoom[] = [80, 90, 100, 110, 120, 125];

export const LIST_DENSITIES: readonly ListDensity[] = ['compact', 'default', 'comfortable'];

export const DENSITY_ROW_HEIGHT: Record<ListDensity, number> = {
	compact: 72,
	default: 88,
	comfortable: 104
};

export const DEFAULT_APPEARANCE: AppearancePreferences = {
	scheme: 'light',
	palette: 'default',
	font: 'default',
	textScale: 100,
	uiZoom: 100,
	density: 'default',
	underlineLinks: false,
	reduceMotion: false
};

export const PALETTE_CATALOG: readonly PaletteDefinition[] = [
	{
		id: 'default',
		labelEn: 'Triage Default',
		labelPt: 'Padrão Triage',
		light: { bg: '#f6f7f9', surface: '#ffffff', accent: '#2563eb', text: '#111827' },
		dark: { bg: '#0d1117', surface: '#151b26', accent: '#3b82f6', text: '#eef2f7' },
		readerLight: {
			bg: '#ffffff',
			fg: '#111827',
			muted: '#525d6d',
			border: '#e2e6ec',
			link: '#2563eb',
			codeBg: '#f1f3f6'
		},
		readerDark: {
			bg: '#121721',
			fg: '#eef2f7',
			muted: '#98a4b8',
			border: '#242d3d',
			link: '#60a5fa',
			codeBg: '#1c2433'
		}
	},
	{
		id: 'nord',
		labelEn: 'Nord',
		labelPt: 'Nord',
		light: { bg: '#eceff4', surface: '#ffffff', accent: '#5e81ac', text: '#2e3440' },
		dark: { bg: '#242933', surface: '#2e3440', accent: '#88c0d0', text: '#eceff4' },
		readerLight: {
			bg: '#f8f9fb',
			fg: '#2e3440',
			muted: '#4c566a',
			border: '#d8dee9',
			link: '#5e81ac',
			codeBg: '#e5e9f0'
		},
		readerDark: {
			bg: '#2e3440',
			fg: '#eceff4',
			muted: '#d8dee9',
			border: '#434c5e',
			link: '#88c0d0',
			codeBg: '#3b4252'
		}
	},
	{
		id: 'dracula',
		labelEn: 'Dracula',
		labelPt: 'Dracula',
		light: { bg: '#f8f8f2', surface: '#ffffff', accent: '#7c3aed', text: '#282a36' },
		dark: { bg: '#21222c', surface: '#282a36', accent: '#bd93f9', text: '#f8f8f2' },
		readerLight: {
			bg: '#ffffff',
			fg: '#282a36',
			muted: '#6272a4',
			border: '#e2e2dc',
			link: '#7c3aed',
			codeBg: '#f1f1ea'
		},
		readerDark: {
			bg: '#282a36',
			fg: '#f8f8f2',
			muted: '#bfc4d6',
			border: '#44475a',
			link: '#bd93f9',
			codeBg: '#343746'
		}
	},
	{
		id: 'catppuccin',
		labelEn: 'Catppuccin',
		labelPt: 'Catppuccin',
		light: { bg: '#eff1f5', surface: '#ffffff', accent: '#1e66f5', text: '#4c4f69' },
		dark: { bg: '#181825', surface: '#1e1e2e', accent: '#cba6f7', text: '#cdd6f4' },
		readerLight: {
			bg: '#ffffff',
			fg: '#4c4f69',
			muted: '#6c6f85',
			border: '#ccd0da',
			link: '#1e66f5',
			codeBg: '#e6e9ef'
		},
		readerDark: {
			bg: '#1e1e2e',
			fg: '#cdd6f4',
			muted: '#a6adc8',
			border: '#313244',
			link: '#cba6f7',
			codeBg: '#313244'
		}
	},
	{
		id: 'solarized',
		labelEn: 'Solarized',
		labelPt: 'Solarized',
		light: { bg: '#fdf6e3', surface: '#fffdf5', accent: '#268bd2', text: '#073642' },
		dark: { bg: '#002b36', surface: '#073642', accent: '#2aa198', text: '#eee8d5' },
		readerLight: {
			bg: '#fdf6e3',
			fg: '#073642',
			muted: '#586e75',
			border: '#e6dfc8',
			link: '#268bd2',
			codeBg: '#eee8d5'
		},
		readerDark: {
			bg: '#002b36',
			fg: '#eee8d5',
			muted: '#93a1a1',
			border: '#0c4352',
			link: '#2aa198',
			codeBg: '#073642'
		}
	},
	{
		id: 'gruvbox',
		labelEn: 'Gruvbox',
		labelPt: 'Gruvbox',
		light: { bg: '#fbf1c7', surface: '#fff8dc', accent: '#af3a03', text: '#3c3836' },
		dark: { bg: '#1d2021', surface: '#282828', accent: '#fabd2f', text: '#ebdbb2' },
		readerLight: {
			bg: '#fbf1c7',
			fg: '#3c3836',
			muted: '#665c54',
			border: '#d5c4a1',
			link: '#076678',
			codeBg: '#ebdbb2'
		},
		readerDark: {
			bg: '#282828',
			fg: '#ebdbb2',
			muted: '#bdae93',
			border: '#3c3836',
			link: '#83a598',
			codeBg: '#3c3836'
		}
	},
	{
		id: 'tokyonight',
		labelEn: 'Tokyo Night',
		labelPt: 'Tokyo Night',
		light: { bg: '#e1e2e7', surface: '#f5f6fa', accent: '#2e7de9', text: '#3760bf' },
		dark: { bg: '#16161e', surface: '#1a1b26', accent: '#7aa2f7', text: '#c0caf5' },
		readerLight: {
			bg: '#f5f6fa',
			fg: '#29344d',
			muted: '#6172b0',
			border: '#d0d5e3',
			link: '#2e7de9',
			codeBg: '#e1e2e7'
		},
		readerDark: {
			bg: '#1a1b26',
			fg: '#c0caf5',
			muted: '#9aa5ce',
			border: '#292e42',
			link: '#7aa2f7',
			codeBg: '#24283b'
		}
	},
	{
		id: 'onedark',
		labelEn: 'One Dark',
		labelPt: 'One Dark',
		light: { bg: '#fafafa', surface: '#ffffff', accent: '#4078f2', text: '#383a42' },
		dark: { bg: '#21252b', surface: '#282c34', accent: '#61afef', text: '#abb2bf' },
		readerLight: {
			bg: '#ffffff',
			fg: '#383a42',
			muted: '#696c77',
			border: '#e5e5e6',
			link: '#4078f2',
			codeBg: '#f0f0f1'
		},
		readerDark: {
			bg: '#282c34',
			fg: '#dce0e6',
			muted: '#9da5b4',
			border: '#3b4048',
			link: '#61afef',
			codeBg: '#21252b'
		}
	},
	{
		id: 'github',
		labelEn: 'GitHub',
		labelPt: 'GitHub',
		light: { bg: '#f6f8fa', surface: '#ffffff', accent: '#0969da', text: '#1f2328' },
		dark: { bg: '#010409', surface: '#0d1117', accent: '#58a6ff', text: '#e6edf3' },
		readerLight: {
			bg: '#ffffff',
			fg: '#1f2328',
			muted: '#656d76',
			border: '#d0d7de',
			link: '#0969da',
			codeBg: '#f6f8fa'
		},
		readerDark: {
			bg: '#0d1117',
			fg: '#e6edf3',
			muted: '#8b949e',
			border: '#30363d',
			link: '#58a6ff',
			codeBg: '#161b22'
		}
	},
	{
		id: 'rosepine',
		labelEn: 'Rosé Pine',
		labelPt: 'Rosé Pine',
		light: { bg: '#faf4ed', surface: '#fffaf3', accent: '#907aa9', text: '#575279' },
		dark: { bg: '#191724', surface: '#1f1d2e', accent: '#c4a7e7', text: '#e0def4' },
		readerLight: {
			bg: '#fffaf3',
			fg: '#575279',
			muted: '#797593',
			border: '#dfdad9',
			link: '#286983',
			codeBg: '#f2e9e1'
		},
		readerDark: {
			bg: '#1f1d2e',
			fg: '#e0def4',
			muted: '#908caa',
			border: '#403d52',
			link: '#c4a7e7',
			codeBg: '#26233a'
		}
	},
	{
		id: 'contrast',
		labelEn: 'High Contrast',
		labelPt: 'Alto Contraste',
		accessibility: true,
		light: { bg: '#ffffff', surface: '#ffffff', accent: '#0000cc', text: '#000000' },
		dark: { bg: '#000000', surface: '#0a0a0a', accent: '#ffff00', text: '#ffffff' },
		readerLight: {
			bg: '#ffffff',
			fg: '#000000',
			muted: '#222222',
			border: '#000000',
			link: '#0000cc',
			codeBg: '#eeeeee'
		},
		readerDark: {
			bg: '#000000',
			fg: '#ffffff',
			muted: '#e5e5e5',
			border: '#ffffff',
			link: '#ffff00',
			codeBg: '#1a1a1a'
		}
	},
	{
		id: 'colorblind',
		labelEn: 'Colorblind Safe',
		labelPt: 'Daltonismo',
		accessibility: true,
		light: { bg: '#f7f8fa', surface: '#ffffff', accent: '#0072b2', text: '#111827' },
		dark: { bg: '#0f141b', surface: '#161d26', accent: '#56b4e9', text: '#f3f4f6' },
		readerLight: {
			bg: '#ffffff',
			fg: '#111827',
			muted: '#4b5563',
			border: '#d1d5db',
			link: '#0072b2',
			codeBg: '#f3f4f6'
		},
		readerDark: {
			bg: '#161d26',
			fg: '#f3f4f6',
			muted: '#9ca3af',
			border: '#374151',
			link: '#56b4e9',
			codeBg: '#1f2937'
		}
	}
];

export const FONT_CATALOG: readonly FontDefinition[] = [
	{
		id: 'default',
		labelEn: 'Triage Default',
		labelPt: 'Padrão Triage',
		sample: 'DM Sans + Plus Jakarta Sans',
		cssStack: "'DM Sans', system-ui, -apple-system, sans-serif"
	},
	{
		id: 'system',
		labelEn: 'System UI',
		labelPt: 'Sistema',
		sample: 'system-ui / Segoe UI / SF',
		cssStack: "system-ui, -apple-system, 'Segoe UI', Roboto, Ubuntu, sans-serif"
	},
	{
		id: 'modern',
		labelEn: 'Modern Display',
		labelPt: 'Moderna',
		sample: 'Plus Jakarta Sans / SF Pro',
		cssStack: "'Plus Jakarta Sans', 'SF Pro Display', 'Segoe UI Variable Display', system-ui, sans-serif"
	},
	{
		id: 'serif',
		labelEn: 'Editorial Serif',
		labelPt: 'Serifa Editorial',
		sample: 'Georgia / Charter / Palatino',
		cssStack: "Charter, 'Iowan Old Style', Georgia, Cambria, 'Times New Roman', serif"
	},
	{
		id: 'rounded',
		labelEn: 'Rounded',
		labelPt: 'Arredondada',
		sample: 'ui-rounded / Nunito',
		cssStack: "ui-rounded, 'SF Pro Rounded', Nunito, 'Varela Round', system-ui, sans-serif"
	},
	{
		id: 'mono',
		labelEn: 'Monospace',
		labelPt: 'Monoespaçada',
		sample: 'JetBrains Mono / Cascadia',
		cssStack: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
	},
	{
		id: 'legible',
		labelEn: 'High Legibility',
		labelPt: 'Alta Legibilidade',
		sample: 'Atkinson Hyperlegible / Lexend',
		cssStack: "'Atkinson Hyperlegible', OpenDyslexic, Lexend, Verdana, Tahoma, sans-serif"
	}
];

export function resolveEffectiveTheme(scheme: ThemeScheme, prefersDark: boolean): ThemeMode {
	if (scheme === 'system') return prefersDark ? 'dark' : 'light';
	return scheme;
}

export function getReaderPaletteColors(
	palette: ThemePalette,
	theme: ThemeMode
): ReaderPaletteColors {
	const entry = PALETTE_CATALOG.find((item) => item.id === palette) ?? PALETTE_CATALOG[0];
	return theme === 'dark' ? entry.readerDark : entry.readerLight;
}

export function getFontCssStack(font: FontPreset): string {
	return (FONT_CATALOG.find((item) => item.id === font) ?? FONT_CATALOG[0]).cssStack;
}

export function stepUiZoom(current: UiZoom, delta: -1 | 1): UiZoom {
	const index = UI_ZOOMS.indexOf(current);
	const safeIndex = index === -1 ? UI_ZOOMS.indexOf(100) : index;
	const nextIndex = Math.min(UI_ZOOMS.length - 1, Math.max(0, safeIndex + delta));
	return UI_ZOOMS[nextIndex];
}

export function normalizeAppearance(
	raw: Partial<AppearancePreferences> | null | undefined,
	legacyTheme?: ThemeMode
): AppearancePreferences {
	const fallbackScheme: ThemeScheme = legacyTheme ?? DEFAULT_APPEARANCE.scheme;
	if (!raw || typeof raw !== 'object') {
		return { ...DEFAULT_APPEARANCE, scheme: fallbackScheme };
	}
	const scheme = THEME_SCHEMES.includes(raw.scheme as ThemeScheme)
		? (raw.scheme as ThemeScheme)
		: fallbackScheme;
	const palette = THEME_PALETTES.includes(raw.palette as ThemePalette)
		? (raw.palette as ThemePalette)
		: DEFAULT_APPEARANCE.palette;
	const font = FONT_PRESETS.includes(raw.font as FontPreset)
		? (raw.font as FontPreset)
		: DEFAULT_APPEARANCE.font;
	const textScale = TEXT_SCALES.includes(raw.textScale as TextScale)
		? (raw.textScale as TextScale)
		: DEFAULT_APPEARANCE.textScale;
	const uiZoom = UI_ZOOMS.includes(raw.uiZoom as UiZoom)
		? (raw.uiZoom as UiZoom)
		: DEFAULT_APPEARANCE.uiZoom;
	const density = LIST_DENSITIES.includes(raw.density as ListDensity)
		? (raw.density as ListDensity)
		: DEFAULT_APPEARANCE.density;
	return {
		scheme,
		palette,
		font,
		textScale,
		uiZoom,
		density,
		underlineLinks: Boolean(raw.underlineLinks),
		reduceMotion: Boolean(raw.reduceMotion)
	};
}
