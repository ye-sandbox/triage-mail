export type WindowState = {
	maximized: boolean;
	focused: boolean;
};

export type WindowLabels = {
	appTitle: string;
	minimize: string;
	maximize: string;
	restore: string;
	close: string;
	actionError: string;
};

export interface WindowTransport {
	isAvailable(): boolean;
	getState(): Promise<WindowState>;
	startDragging(): Promise<void>;
	minimize(): Promise<void>;
	toggleMaximize(): Promise<boolean>;
	close(): Promise<void>;
	subscribeState(onChange: (state: WindowState) => void): () => void;
}

const LABELS_EN: WindowLabels = {
	appTitle: 'triage-mail',
	minimize: 'Minimize',
	maximize: 'Maximize',
	restore: 'Restore',
	close: 'Close',
	actionError: 'Window action unavailable.'
};

const LABELS_PT_BR: WindowLabels = {
	appTitle: 'triage-mail',
	minimize: 'Minimizar',
	maximize: 'Maximizar',
	restore: 'Restaurar',
	close: 'Fechar',
	actionError: 'Ação da janela indisponível.'
};

export function getWindowLabels(locale: 'en' | 'pt-BR'): WindowLabels {
	return locale === 'pt-BR' ? LABELS_PT_BR : LABELS_EN;
}
