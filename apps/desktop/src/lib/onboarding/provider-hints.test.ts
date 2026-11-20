import { expect, it } from 'vitest';
import { providerHintKey } from './provider-hints';

it('resolves provider hints only for rejected app-password logins on known hosts', () => {
	expect(providerHintKey('AuthenticationRejected', 'IMAP.Gmail.com ', 'AppPassword')).toBe('gmail');
	expect(providerHintKey('AuthenticationRejected', 'imap.googlemail.com', 'AppPassword')).toBe('gmail');
	expect(providerHintKey('AuthenticationRejected', 'outlook.office365.com', 'AppPassword')).toBe(
		'microsoftBasicAuth'
	);
	expect(providerHintKey('AuthenticationRejected', 'imap.example.com', 'AppPassword')).toBeNull();
	expect(providerHintKey('AuthenticationRejected', 'imap.gmail.com', 'OAuth2ByoCredentials')).toBeNull();
	expect(providerHintKey('TlsHandshakeFailed', 'imap.gmail.com', 'AppPassword')).toBeNull();
	expect(providerHintKey(null, 'imap.gmail.com', 'AppPassword')).toBeNull();
});
