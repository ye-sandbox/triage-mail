import { describe, expect, it } from 'vitest';
import { looksLikeRegularGmailPassword, normalizeAppPassword, normalizeSecretFor } from './secret';

describe('app password normalization', () => {
	it('strips spaces, NBSP, tabs and newlines', () => {
		expect(normalizeAppPassword(' abcd efgh ijkl\tmnop\r\n')).toBe('abcdefghijklmnop');
	});

	it('only normalizes AppPassword secrets', () => {
		expect(normalizeSecretFor('AppPassword', 'a b')).toBe('ab');
		expect(normalizeSecretFor('ProtonBridgeLocal', 'a b')).toBe('a b');
		expect(normalizeSecretFor('OAuth2ByoCredentials', 'a b')).toBe('a b');
	});
});

describe('Gmail regular password hint', () => {
	it('accepts a spaced 16-letter app password', () => {
		expect(looksLikeRegularGmailPassword('imap.gmail.com', 'AppPassword', 'abcd efgh ijkl mnop')).toBe(
			false
		);
	});

	it('flags other shapes on Gmail hosts', () => {
		expect(looksLikeRegularGmailPassword('imap.gmail.com', 'AppPassword', 'hunter2!')).toBe(true);
		expect(looksLikeRegularGmailPassword('IMAP.GoogleMail.com', 'AppPassword', 'abcdefghijklmno1')).toBe(
			true
		);
	});

	it('stays silent for empty input, other hosts and other methods', () => {
		expect(looksLikeRegularGmailPassword('imap.gmail.com', 'AppPassword', '  ')).toBe(false);
		expect(looksLikeRegularGmailPassword('imap.example.com', 'AppPassword', 'x')).toBe(false);
		expect(looksLikeRegularGmailPassword('imap.gmail.com', 'ProtonBridgeLocal', 'x')).toBe(false);
	});
});
