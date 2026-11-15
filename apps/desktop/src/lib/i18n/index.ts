import type { MessageCategory, SyncStatus } from '$lib/types';
import { i18n } from './index.svelte';
import { translations } from './translations';
import type { SupportedLocale, Translations } from './types';

export type ShellTranslationKey = keyof Translations['shell'];

export function t(locale: SupportedLocale, key: ShellTranslationKey): string {
	return translations[locale].shell[key] ?? translations.en.shell[key] ?? key;
}

export function formatCategoryLabel(locale: SupportedLocale, category: MessageCategory): string {
	if (locale === 'en') return category;
	const map: Record<MessageCategory, string> = {
		Security: 'Segurança',
		Financial: 'Financeiro',
		Shopping: 'Compras',
		Newsletter: 'Newsletters',
		Notifications: 'Notificações',
		Personal: 'Pessoal'
	};
	return map[category];
}

export function formatSyncStatus(locale: SupportedLocale, status: SyncStatus): string {
	if (locale === 'en') return status;
	const map: Record<SyncStatus, string> = {
		Idle: 'Ocioso',
		Syncing: 'Sincronizando',
		Retrying: 'Tentando novamente',
		MutationPending: 'Mutação pendente',
		MutationFailed: 'Falha na mutação',
		ResyncRequired: 'Ressincronizar'
	};
	return map[status];
}

export function formatBrl(cents: number, locale: SupportedLocale): string {
	return new Intl.NumberFormat(locale === 'pt-BR' ? 'pt-BR' : 'en-US', {
		style: 'currency',
		currency: 'BRL'
	}).format(cents / 100);
}

export function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function formatTimestamp(iso: string, locale: SupportedLocale): string {
	const d = new Date(iso);
	return new Intl.DateTimeFormat(locale === 'pt-BR' ? 'pt-BR' : 'en-US', {
		month: 'short',
		day: '2-digit',
		hour: '2-digit',
		minute: '2-digit',
		hour12: false
	}).format(d);
}

export { i18n, type SupportedLocale, type Translations };
