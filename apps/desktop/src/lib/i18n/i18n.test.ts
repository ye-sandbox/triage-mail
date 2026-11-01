import { beforeEach, describe, expect, it } from 'vitest';
import { i18n } from './index.svelte';
import { translations } from './translations';

describe('i18n store', () => {
	beforeEach(() => {
		i18n.setLocale('en');
	});

	it('initializes with English by default and exposes typed dictionaries', () => {
		expect(i18n.locale).toBe('en');
		expect(i18n.t.app.title).toBe('Triage Mail');
		expect(i18n.t.accountForm.methodAppPassword).toBe('App password');
		expect(i18n.t.guide.appPassword.title).toBe('App Password');
	});

	it('switches dynamically to Portuguese and updates derived translations', () => {
		i18n.setLocale('pt-BR');
		expect(i18n.locale).toBe('pt-BR');
		expect(i18n.t.accountForm.methodAppPassword).toBe('Senha de aplicativo');
		expect(i18n.t.guide.appPassword.title).toBe('Senha de Aplicativo (App Password)');
		expect(i18n.t.accountForm.quickFillLabel).toBe('Preenchimento rápido:');
	});

	it('persists selected locale to localStorage', () => {
		i18n.setLocale('pt-BR');
		expect(localStorage.getItem('triage_mail_locale')).toBe('pt-BR');
		i18n.setLocale('en');
		expect(localStorage.getItem('triage_mail_locale')).toBe('en');
	});

	it('maintains 1:1 key parity between en and pt-BR translation dictionaries', () => {
		const enKeys = Object.keys(translations.en).sort();
		const ptKeys = Object.keys(translations['pt-BR']).sort();
		expect(enKeys).toEqual(ptKeys);

		for (const section of enKeys as (keyof typeof translations.en)[]) {
			const enSubKeys = Object.keys(translations.en[section]).sort();
			const ptSubKeys = Object.keys(translations['pt-BR'][section]).sort();
			expect(enSubKeys).toEqual(ptSubKeys);
		}
	});
});
