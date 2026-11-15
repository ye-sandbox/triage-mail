export const GOOGLE_AUTH_ENDPOINT = 'https://accounts.google.com/o/oauth2/v2/auth';
export const GOOGLE_TOKEN_ENDPOINT = 'https://oauth2.googleapis.com/token';
export const GOOGLE_IMAP_SCOPE = 'https://mail.google.com/';
export const MAX_GOOGLE_CLIENT_JSON_BYTES = 65536;

export type GoogleOAuthImportError = 'too_large' | 'web_client_unsupported' | 'invalid_format';

export type GoogleOAuthImportResult =
	| {
			ok: true;
			clientId: string;
			clientSecret: string;
			authEndpoint: string;
			tokenEndpoint: string;
			scopes: string;
	  }
	| {
			ok: false;
			reason: GoogleOAuthImportError;
	  };

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function hasControlChars(value: string): boolean {
	for (let i = 0; i < value.length; i++) {
		const code = value.charCodeAt(i);
		if (code < 0x20 || code === 0x7f) return true;
	}
	return false;
}

function validHttpsEndpoint(raw: unknown, fallback: string): string | null {
	if (raw === undefined || raw === null || raw === '') return fallback;
	if (typeof raw !== 'string') return null;
	const trimmed = raw.trim();
	if (trimmed.length === 0 || trimmed.length > 2048 || hasControlChars(trimmed)) return null;
	try {
		const parsed = new URL(trimmed);
		if (
			parsed.protocol !== 'https:' ||
			!parsed.hostname ||
			parsed.username !== '' ||
			parsed.password !== '' ||
			parsed.search !== '' ||
			parsed.hash !== ''
		) {
			return null;
		}
		return trimmed;
	} catch {
		return null;
	}
}

function validCredential(raw: unknown, maxLength: number): string | null {
	if (typeof raw !== 'string') return null;
	const trimmed = raw.trim();
	if (trimmed.length === 0 || trimmed.length > maxLength || hasControlChars(trimmed)) return null;
	return trimmed;
}

export function parseGoogleOAuthClientJson(raw: string): GoogleOAuthImportResult {
	if (raw.length === 0 || raw.length > MAX_GOOGLE_CLIENT_JSON_BYTES) {
		return { ok: false, reason: raw.length > MAX_GOOGLE_CLIENT_JSON_BYTES ? 'too_large' : 'invalid_format' };
	}

	let parsed: unknown;
	try {
		parsed = JSON.parse(raw);
	} catch {
		return { ok: false, reason: 'invalid_format' };
	}

	if (!isRecord(parsed)) {
		return { ok: false, reason: 'invalid_format' };
	}

	if (!isRecord(parsed.installed)) {
		if (isRecord(parsed.web)) {
			return { ok: false, reason: 'web_client_unsupported' };
		}
		return { ok: false, reason: 'invalid_format' };
	}

	const installed = parsed.installed;
	const clientId = validCredential(installed.client_id, 4096);
	const clientSecret = validCredential(installed.client_secret, 8192);
	const authEndpoint = validHttpsEndpoint(installed.auth_uri, GOOGLE_AUTH_ENDPOINT);
	const tokenEndpoint = validHttpsEndpoint(installed.token_uri, GOOGLE_TOKEN_ENDPOINT);

	if (!clientId || !clientSecret || !authEndpoint || !tokenEndpoint) {
		return { ok: false, reason: 'invalid_format' };
	}

	return {
		ok: true,
		clientId,
		clientSecret,
		authEndpoint,
		tokenEndpoint,
		scopes: GOOGLE_IMAP_SCOPE
	};
}
