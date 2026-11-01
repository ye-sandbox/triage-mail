import { translations } from './translations';
import type { SupportedLocale, Translations } from './types';

const STORAGE_KEY = 'triage_mail_locale';

function getInitialLocale(): SupportedLocale {
	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (saved === 'en' || saved === 'pt-BR') {
			return saved;
		}
	}
	if (typeof navigator !== 'undefined' && typeof navigator.language === 'string') {
		if (navigator.language.toLowerCase().startsWith('pt')) {
			return 'pt-BR';
		}
	}
	return 'en';
}

class I18nStore {
	locale = $state<SupportedLocale>(getInitialLocale());
	t = $derived<Translations>(translations[this.locale]);

	setLocale(next: SupportedLocale) {
		this.locale = next;
		if (typeof localStorage !== 'undefined') {
			try {
				localStorage.setItem(STORAGE_KEY, next);
			} catch {
				// Local storage unavailable or quota exceeded; in-memory state persists.
			}
		}
	}
}

export const i18n = new I18nStore();
export { type SupportedLocale, type Translations } from './types';
