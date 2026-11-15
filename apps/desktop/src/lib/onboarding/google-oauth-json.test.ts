import { describe, expect, it } from 'vitest';
import {
	GOOGLE_AUTH_ENDPOINT,
	GOOGLE_IMAP_SCOPE,
	GOOGLE_TOKEN_ENDPOINT,
	MAX_GOOGLE_CLIENT_JSON_BYTES,
	parseGoogleOAuthClientJson
} from './google-oauth-json';

describe('parseGoogleOAuthClientJson', () => {
	it('parses a valid Google Cloud Desktop (installed) OAuth JSON', () => {
		const raw = JSON.stringify({
			installed: {
				client_id: 'synthetic-client-id.apps.googleusercontent.com',
				project_id: 'synthetic-project',
				auth_uri: 'https://accounts.google.com/o/oauth2/auth',
				token_uri: 'https://oauth2.googleapis.com/token',
				client_secret: 'synthetic-client-secret',
				redirect_uris: ['http://localhost']
			}
		});

		expect(parseGoogleOAuthClientJson(raw)).toEqual({
			ok: true,
			clientId: 'synthetic-client-id.apps.googleusercontent.com',
			clientSecret: 'synthetic-client-secret',
			authEndpoint: 'https://accounts.google.com/o/oauth2/auth',
			tokenEndpoint: 'https://oauth2.googleapis.com/token',
			scopes: GOOGLE_IMAP_SCOPE
		});
	});

	it('falls back to default Google OAuth endpoints when auth_uri or token_uri are omitted', () => {
		const raw = JSON.stringify({
			installed: {
				client_id: 'synthetic-client-id',
				client_secret: 'synthetic-client-secret'
			}
		});

		expect(parseGoogleOAuthClientJson(raw)).toEqual({
			ok: true,
			clientId: 'synthetic-client-id',
			clientSecret: 'synthetic-client-secret',
			authEndpoint: GOOGLE_AUTH_ENDPOINT,
			tokenEndpoint: GOOGLE_TOKEN_ENDPOINT,
			scopes: GOOGLE_IMAP_SCOPE
		});
	});

	it('rejects Web application JSON credentials with web_client_unsupported', () => {
		const raw = JSON.stringify({
			web: {
				client_id: 'synthetic-web-client-id',
				client_secret: 'synthetic-web-secret'
			}
		});

		expect(parseGoogleOAuthClientJson(raw)).toEqual({
			ok: false,
			reason: 'web_client_unsupported'
		});
	});

	it('rejects oversized, malformed, non-HTTPS, and incomplete payloads', () => {
		expect(parseGoogleOAuthClientJson('')).toEqual({ ok: false, reason: 'invalid_format' });
		expect(parseGoogleOAuthClientJson('{bad json')).toEqual({ ok: false, reason: 'invalid_format' });
		expect(parseGoogleOAuthClientJson('x'.repeat(MAX_GOOGLE_CLIENT_JSON_BYTES + 1))).toEqual({
			ok: false,
			reason: 'too_large'
		});
		expect(
			parseGoogleOAuthClientJson(
				JSON.stringify({
					installed: {
						client_id: 'synthetic-client-id',
						client_secret: 'synthetic-client-secret',
						auth_uri: 'http://accounts.google.com/o/oauth2/auth'
					}
				})
			)
		).toEqual({ ok: false, reason: 'invalid_format' });
		expect(
			parseGoogleOAuthClientJson(
				JSON.stringify({
					installed: {
						client_id: 'synthetic-client-id'
					}
				})
			)
		).toEqual({ ok: false, reason: 'invalid_format' });
	});
});
