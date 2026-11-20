import type { ConnectionDiagnosticCode } from '../inbox/types';
import type { AuthMethod } from './types';

export type ProviderHintKey = 'gmail' | 'microsoftBasicAuth';

const GMAIL_HOSTS = new Set(['imap.gmail.com', 'imap.googlemail.com']);
const MICROSOFT_HOSTS = new Set(['outlook.office365.com', 'imap-mail.outlook.com']);

/**
 * Additive, UI-only refinement of `AuthenticationRejected`.
 * INVARIANT: Resolved from host + method only; the IPC wire diagnostic codes stay unchanged (P1).
 */
export function providerHintKey(
	code: ConnectionDiagnosticCode | null | undefined,
	host: string,
	method: AuthMethod
): ProviderHintKey | null {
	if (code !== 'AuthenticationRejected' || method !== 'AppPassword') return null;
	const normalized = host.trim().toLowerCase();
	if (GMAIL_HOSTS.has(normalized)) return 'gmail';
	if (MICROSOFT_HOSTS.has(normalized)) return 'microsoftBasicAuth';
	return null;
}
