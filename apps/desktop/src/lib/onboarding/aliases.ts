export type AccountAlias = { address: string; label?: string | null };
export type AccountAliasMap = Record<string, AccountAlias[]>;

// INVARIANT: mirrors storage::normalize_aliases; the backend remains the authority.
export const MAX_ACCOUNT_ALIASES = 64;
export const MAX_ALIAS_ADDRESS_CHARS = 254;
export const MAX_ALIAS_LABEL_CHARS = 80;

export type AliasIssue = 'invalid' | 'duplicate' | 'limit';

const ADDRESS = /^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z0-9-]{2,63}$/;
// eslint-disable-next-line no-control-regex
const CONTROL = /[\u0000-\u001f\u007f]/;

export function normalizeAliasAddress(raw: string): string {
	return raw.trim().toLowerCase();
}

/** Returns the normalized alias, or the reason it cannot be added to `existing`. */
export function validateAlias(
	rawAddress: string,
	rawLabel: string,
	existing: readonly AccountAlias[]
): { alias: AccountAlias } | { issue: AliasIssue } {
	const address = normalizeAliasAddress(rawAddress);
	const label = rawLabel.trim();
	if (
		!address ||
		address.length > MAX_ALIAS_ADDRESS_CHARS ||
		!ADDRESS.test(address) ||
		[...label].length > MAX_ALIAS_LABEL_CHARS ||
		CONTROL.test(label)
	) {
		return { issue: 'invalid' };
	}
	if (existing.some((a) => a.address === address)) return { issue: 'duplicate' };
	if (existing.length >= MAX_ACCOUNT_ALIASES) return { issue: 'limit' };
	return { alias: label ? { address, label } : { address } };
}

export function aliasesFor(map: AccountAliasMap, accountId: string): AccountAlias[] {
	return map[accountId] ?? [];
}
