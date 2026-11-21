import { describe, expect, it } from 'vitest';
import { guidanceKey, isMethodPrimary, providerKindFor, recommendedOptions } from './recommendation';

const base = { preset: '', host: '', method: 'AppPassword' as const, status: 'idle' };

describe('Cloudflare Email Routing', () => {
	const cf = { ...base, status: 'unresolved', providerName: 'Cloudflare Email Routing' };
	it('is detected only for the unresolved forwarder', () => {
		expect(providerKindFor(cf)).toBe('cloudflare');
		expect(providerKindFor({ ...cf, host: 'imap.example.com' })).toBe('custom');
		expect(providerKindFor({ ...cf, status: 'found' })).toBe('custom');
		expect(providerKindFor({ ...cf, providerName: 'Other' })).toBe('custom');
	});
	it('maps guidance and options', () => {
		expect(guidanceKey('cloudflare')).toBe('providerGuidanceCloudflare');
		const options = recommendedOptions('cloudflare');
		expect(options[0]).toMatchObject({ label: 'methodAppPassword', recommended: true });
	});
});

describe('providerKindFor', () => {
	it('maps presets and hosts', () => {
		expect(providerKindFor({ ...base, preset: 'Gmail' })).toBe('google');
		expect(providerKindFor({ ...base, host: 'IMAP.Gmail.com' })).toBe('google');
		expect(providerKindFor({ ...base, preset: 'Outlook' })).toBe('microsoft');
		expect(providerKindFor({ ...base, host: 'outlook.office365.com' })).toBe('microsoft');
		expect(providerKindFor({ ...base, host: '127.0.0.1' })).toBe('proton');
		expect(providerKindFor({ ...base, host: 'x', method: 'ProtonBridgeLocal' })).toBe('proton');
		expect(providerKindFor({ ...base, host: 'imap.example.com', status: 'unresolved' })).toBe(
			'custom'
		);
		expect(providerKindFor({ ...base, host: '' })).toBe('custom');
		expect(providerKindFor({ ...base, host: 'imap.example.com', status: 'found' })).toBe(
			'appPassword'
		);
	});
	it('does not mutate its input', () => {
		const input = { ...base, host: 'imap.example.com' };
		const copy = { ...input };
		providerKindFor(input);
		expect(input).toEqual(copy);
	});
});

describe('recommendedOptions', () => {
	it('recommends per provider with alternatives', () => {
		const g = recommendedOptions('google');
		expect(g[0]).toMatchObject({ recommended: true, action: { type: 'preset', name: 'Gmail' } });
		expect(g[1].action).toEqual({ type: 'preset', name: 'Gmail OAuth' });
		expect(recommendedOptions('microsoft')[0].action).toEqual({ type: 'preset', name: 'Outlook' });
		expect(recommendedOptions('proton')[0].action).toEqual({ type: 'preset', name: 'Proton' });
		expect(recommendedOptions('custom')[0].action).toEqual({ type: 'custom' });
		expect(guidanceKey('custom')).toBe('providerGuidanceCustom');
	});
});

describe('isMethodPrimary', () => {
	it('collapses other methods unless expanded or active', () => {
		expect(isMethodPrimary('OAuth2PublicPkce', 'google', 'AppPassword', false)).toBe(false);
		expect(isMethodPrimary('OAuth2ByoCredentials', 'google', 'AppPassword', false)).toBe(true);
		expect(isMethodPrimary('OAuth2PublicPkce', 'google', 'AppPassword', true)).toBe(true);
		expect(isMethodPrimary('OAuth2PublicPkce', 'google', 'OAuth2PublicPkce', false)).toBe(true);
		expect(isMethodPrimary('ProtonBridgeLocal', 'appPassword', 'AppPassword', false)).toBe(false);
		expect(isMethodPrimary('ProtonBridgeLocal', 'custom', 'AppPassword', false)).toBe(true);
		expect(isMethodPrimary('LocalArchive', 'proton', 'ProtonBridgeLocal', false)).toBe(true);
	});
});
