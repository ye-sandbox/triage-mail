import type { MessageCategory } from '$lib/types';

export const BUILTIN_CATEGORY_IDS: readonly MessageCategory[] = [
	'Security',
	'Financial',
	'Shopping',
	'Newsletter',
	'Notifications',
	'Personal'
] as const;

export interface LocalCategoryItem {
	id: string;
	kind: 'builtin' | 'custom';
	customLabel: string | null;
	tag?: string;
	hidden?: boolean;
}

export const DEFAULT_LOCAL_CATEGORIES: readonly LocalCategoryItem[] = BUILTIN_CATEGORY_IDS.map(
	(id) => ({
		id,
		kind: 'builtin',
		customLabel: null,
		hidden: false
	})
);

function isValidName(value: unknown): value is string {
	return (
		typeof value === 'string' &&
		value.trim().length > 0 &&
		value.trim().length <= 64 &&
		!/[\u0000-\u001f\u007f-\u009f]/u.test(value)
	);
}

export function createDefaultLocalCategories(): LocalCategoryItem[] {
	return DEFAULT_LOCAL_CATEGORIES.map((item) => ({ ...item }));
}

export function makeCustomCategory(name: string): LocalCategoryItem | null {
	const trimmed = name.trim();
	if (!isValidName(trimmed)) return null;
	const id = `custom:${typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function' ? crypto.randomUUID() : `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`}`;
	return {
		id,
		kind: 'custom',
		customLabel: trimmed,
		tag: trimmed,
		hidden: false
	};
}

export function normalizeLocalCategories(raw: unknown): LocalCategoryItem[] {
	if (!Array.isArray(raw)) return createDefaultLocalCategories();

	const seen = new Set<string>();
	const result: LocalCategoryItem[] = [];

	for (const entry of raw) {
		if (!entry || typeof entry !== 'object') continue;
		const candidate = entry as Partial<LocalCategoryItem>;
		if (typeof candidate.id !== 'string' || seen.has(candidate.id)) continue;

		if (
			candidate.kind === 'builtin' &&
			BUILTIN_CATEGORY_IDS.includes(candidate.id as MessageCategory)
		) {
			seen.add(candidate.id);
			result.push({
				id: candidate.id,
				kind: 'builtin',
				customLabel: isValidName(candidate.customLabel) ? candidate.customLabel.trim() : null,
				hidden: Boolean(candidate.hidden)
			});
		} else if (
			candidate.kind === 'custom' &&
			candidate.id.startsWith('custom:') &&
			isValidName(candidate.customLabel)
		) {
			const label = candidate.customLabel.trim();
			const tag = isValidName(candidate.tag) ? candidate.tag.trim() : label;
			seen.add(candidate.id);
			result.push({
				id: candidate.id,
				kind: 'custom',
				customLabel: label,
				tag,
				hidden: false
			});
		}
	}

	for (const builtinId of BUILTIN_CATEGORY_IDS) {
		if (!seen.has(builtinId)) {
			result.push({
				id: builtinId,
				kind: 'builtin',
				customLabel: null,
				hidden: false
			});
		}
	}

	return result;
}
