import type { AuthMethod } from './types';

const GMAIL_HOSTS = new Set(['imap.gmail.com', 'imap.googlemail.com']);

/** INVARIANT: Google shows app passwords in space-separated groups; only AppPassword secrets are normalized. */
export function normalizeAppPassword(secret: string): string {
	return secret.replace(/\s+/gu, '');
}

export function normalizeSecretFor(method: AuthMethod, secret: string): string {
	return method === 'AppPassword' ? normalizeAppPassword(secret) : secret;
}

/** True when a Gmail App Password field is non-empty but not 16 ASCII letters (likely a regular password). */
export function looksLikeRegularGmailPassword(
	host: string,
	method: AuthMethod,
	secret: string
): boolean {
	if (method !== 'AppPassword' || !GMAIL_HOSTS.has(host.trim().toLowerCase())) return false;
	const normalized = normalizeAppPassword(secret);
	return normalized.length > 0 && !/^[A-Za-z]{16}$/u.test(normalized);
}
