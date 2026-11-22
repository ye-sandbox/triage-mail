export type ThemeScheme = 'light' | 'dark' | 'system';

export type ThemePalette =
	| 'default'
	| 'nord'
	| 'dracula'
	| 'catppuccin'
	| 'solarized'
	| 'gruvbox'
	| 'tokyonight'
	| 'onedark'
	| 'github'
	| 'rosepine'
	| 'contrast'
	| 'colorblind';

export type FontPreset = 'default' | 'system' | 'modern' | 'serif' | 'rounded' | 'mono' | 'legible';

export type TextScale = 90 | 100 | 112 | 125;

export type UiZoom = 80 | 90 | 100 | 110 | 120 | 125;

export type ListDensity = 'compact' | 'default' | 'comfortable';

export interface AppearancePreferences {
	scheme: ThemeScheme;
	palette: ThemePalette;
	font: FontPreset;
	textScale: TextScale;
	uiZoom: UiZoom;
	density: ListDensity;
	underlineLinks: boolean;
	reduceMotion: boolean;
}

export interface ReaderPaletteColors {
	bg: string;
	fg: string;
	muted: string;
	border: string;
	link: string;
	codeBg: string;
}

export interface PaletteSwatch {
	bg: string;
	surface: string;
	accent: string;
	text: string;
}

export interface PaletteDefinition {
	id: ThemePalette;
	labelEn: string;
	labelPt: string;
	accessibility?: boolean;
	light: PaletteSwatch;
	dark: PaletteSwatch;
	readerLight: ReaderPaletteColors;
	readerDark: ReaderPaletteColors;
}

export interface FontDefinition {
	id: FontPreset;
	labelEn: string;
	labelPt: string;
	sample: string;
	cssStack: string;
}
