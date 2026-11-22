import type { Destination } from '$lib/alerts/types';
import type { Rule, RuleAction } from '$lib/rules/types';
import { extractConditionEntry } from './simulator';

export type GroupKey = 'all' | 'any' | 'except';
export type ActionKind = 'Tag' | 'Star' | 'Archive' | 'Alert' | 'UrgentAlert';

export interface RuleValidationMessages {
	error: string;
	validationMissingCondition: string;
	validationTooManyConditions: string;
	validationMissingAction: string;
	validationInvalidCondition: string;
	validationInvalidTag: string;
}

export function createRuleAction(kind: ActionKind, destinations: Destination[]): RuleAction {
	if (kind === 'Tag') return { Tag: '' };
	if (kind === 'Star') return 'Star';
	if (kind === 'Archive') return 'Archive';
	const firstDestId = destinations[0]?.id ?? '';
	return kind === 'Alert' ? { Alert: firstDestId } : { UrgentAlert: firstDestId };
}

export function actionKindOf(action: RuleAction): ActionKind {
	if (action === 'Star') return 'Star';
	if (action === 'Archive') return 'Archive';
	if ('Tag' in action) return 'Tag';
	if ('Alert' in action) return 'Alert';
	return 'UrgentAlert';
}

export function validateRuleDraft(value: Rule, msgs: RuleValidationMessages): string | null {
	if (!value.name.trim() || value.name.trim().length > 120) {
		return msgs.error;
	}
	if (value.all.length + value.any.length === 0) {
		return msgs.validationMissingCondition;
	}
	if (value.all.length + value.any.length + value.except.length > 64) {
		return msgs.validationTooManyConditions;
	}
	if (value.actions.length === 0 || value.actions.length > 16) {
		return msgs.validationMissingAction;
	}
	for (const cond of [...value.all, ...value.any, ...value.except]) {
		const { field, value: raw } = extractConditionEntry(cond);
		if (field === 'AmountAtLeast' || field === 'AmountAtMost') {
			const num = Number(raw);
			if (!Number.isFinite(num) || !Number.isInteger(num) || num < 0) {
				return msgs.validationInvalidCondition;
			}
		} else if (field === 'DueOnOrAfter' || field === 'DueOnOrBefore') {
			if (!/^\d{4}-\d{2}-\d{2}$/.test(raw.trim())) {
				return msgs.validationInvalidCondition;
			}
		} else if (field === 'Sender') {
			if (!raw.trim().includes('@')) {
				return msgs.validationInvalidCondition;
			}
		} else if (!raw.trim()) {
			return msgs.validationInvalidCondition;
		}
	}
	for (const act of value.actions) {
		if (typeof act === 'object' && 'Tag' in act) {
			const trimmed = act.Tag.trim();
			if (!trimmed || trimmed.length > 80) {
				return msgs.validationInvalidTag;
			}
		} else if (typeof act === 'object' && ('Alert' in act || 'UrgentAlert' in act)) {
			const destId = 'Alert' in act ? act.Alert : act.UrgentAlert;
			if (!destId.trim()) {
				return msgs.validationMissingAction;
			}
		}
	}
	return null;
}
