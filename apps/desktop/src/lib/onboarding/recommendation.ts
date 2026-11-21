import type { AuthMethod } from './types';

export type ProviderKind =
	| 'google'
	| 'microsoft'
	| 'proton'
	| 'custom'
	| 'appPassword'
	| 'cloudflare';
export type DiscoveryStatusLike = string;

export type GuidanceKey =
	| 'providerGuidanceGoogle'
	| 'providerGuidanceMicrosoft'
	| 'providerGuidanceProton'
	| 'providerGuidanceCustom'
	| 'providerGuidanceCloudflare'
	| 'providerGuidanceAppPassword';
export type LabelKey =
	| 'methodAppPassword'
	| 'googleCloudJsonOption'
	| 'microsoftSetup'
	| 'methodProton'
	| 'customImapOption';

export type RecommendedAction =
	| { type: 'preset'; name: string }
	| { type: 'method'; method: AuthMethod }
	| { type: 'custom' };

export interface RecommendedOption {
	label: LabelKey;
	recommended: boolean;
	action: RecommendedAction;
	/** Auth method that marks this option as the active one. */
	activeWhen: AuthMethod;
}

export interface RecommendationInput {
	preset: string;
	host: string;
	method: AuthMethod;
	status: DiscoveryStatusLike;
	/** Provider name reported by discovery (e.g. an MX-detected forwarder). */
	providerName?: string | null;
}

export const CLOUDFLARE_PROVIDER = 'Cloudflare Email Routing';
const GMAIL_HOSTS = new Set(['imap.gmail.com', 'imap.googlemail.com']);

/**
 * Maps current (read-only) connection settings to a provider family.
 * INVARIANT: pure; never writes settings, so a late discovery suggestion cannot override manual edits.
 */
export function providerKindFor({
	preset,
	host,
	method,
	status,
	providerName
}: RecommendationInput): ProviderKind {
	const h = host.trim().toLowerCase();
	// INVARIANT: Cloudflare Email Routing only forwards; it has no IMAP host, so it is detected by name while unresolved.
	if (providerName === CLOUDFLARE_PROVIDER && status === 'unresolved' && !h) return 'cloudflare';
	if (
		preset === 'Gmail' ||
		preset === 'Gmail OAuth' ||
		preset === 'Google Workspace' ||
		GMAIL_HOSTS.has(h)
	) {
		return 'google';
	}
	if (preset === 'Outlook' || h === 'outlook.office365.com') return 'microsoft';
	if (preset === 'Proton' || method === 'ProtonBridgeLocal' || h === '127.0.0.1') return 'proton';
	if (preset === 'Other IMAP' || status === 'unresolved' || !h) return 'custom';
	return 'appPassword';
}

export function guidanceKey(kind: ProviderKind): GuidanceKey {
	switch (kind) {
		case 'google':
			return 'providerGuidanceGoogle';
		case 'microsoft':
			return 'providerGuidanceMicrosoft';
		case 'proton':
			return 'providerGuidanceProton';
		case 'custom':
			return 'providerGuidanceCustom';
		case 'cloudflare':
			return 'providerGuidanceCloudflare';
		default:
			return 'providerGuidanceAppPassword';
	}
}

export function recommendedOptions(kind: ProviderKind): RecommendedOption[] {
	switch (kind) {
		case 'google':
			return [
				{
					label: 'methodAppPassword',
					recommended: true,
					action: { type: 'preset', name: 'Gmail' },
					activeWhen: 'AppPassword'
				},
				{
					label: 'googleCloudJsonOption',
					recommended: false,
					action: { type: 'preset', name: 'Gmail OAuth' },
					activeWhen: 'OAuth2ByoCredentials'
				}
			];
		case 'microsoft':
			return [
				{
					label: 'microsoftSetup',
					recommended: true,
					action: { type: 'preset', name: 'Outlook' },
					activeWhen: 'OAuth2PublicPkce'
				},
				{
					label: 'methodAppPassword',
					recommended: false,
					action: { type: 'method', method: 'AppPassword' },
					activeWhen: 'AppPassword'
				}
			];
		case 'proton':
			return [
				{
					label: 'methodProton',
					recommended: true,
					action: { type: 'preset', name: 'Proton' },
					activeWhen: 'ProtonBridgeLocal'
				}
			];
		case 'cloudflare':
			return [
				{
					label: 'methodAppPassword',
					recommended: true,
					action: { type: 'method', method: 'AppPassword' },
					activeWhen: 'AppPassword'
				},
				{
					label: 'customImapOption',
					recommended: false,
					action: { type: 'custom' },
					activeWhen: 'ProtonBridgeLocal'
				}
			];
		case 'custom':
			return [
				{
					label: 'customImapOption',
					recommended: false,
					action: { type: 'custom' },
					activeWhen: 'AppPassword'
				}
			];
		default:
			return [
				{
					label: 'methodAppPassword',
					recommended: true,
					action: { type: 'method', method: 'AppPassword' },
					activeWhen: 'AppPassword'
				},
				{
					label: 'customImapOption',
					recommended: false,
					action: { type: 'custom' },
					activeWhen: 'ProtonBridgeLocal'
				}
			];
	}
}

/** Whether an auth tab stays visible before the user expands "more sign-in methods". */
export function isMethodPrimary(
	tab: AuthMethod,
	kind: ProviderKind,
	current: AuthMethod,
	showAll: boolean
): boolean {
	if (tab === current || tab === 'LocalArchive' || kind === 'custom' || showAll) return true;
	switch (kind) {
		case 'google':
			return tab === 'AppPassword' || tab === 'OAuth2ByoCredentials';
		case 'microsoft':
			return tab === 'OAuth2PublicPkce' || tab === 'AppPassword';
		case 'proton':
			return tab === 'ProtonBridgeLocal';
		default:
			return tab === 'AppPassword';
	}
}
