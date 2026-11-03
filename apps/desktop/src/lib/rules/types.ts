export const FIELDS = [
	'Account',
	'Sender',
	'Domain',
	'DomainSuffix',
	'Subject',
	'Body',
	'AttachmentName',
	'AttachmentMime',
	'AmountAtLeast',
	'AmountAtMost',
	'DueOnOrAfter',
	'DueOnOrBefore'
] as const;
export type Field = (typeof FIELDS)[number];
export type Condition = {
	[K in Field]: K extends 'AmountAtLeast' | 'AmountAtMost'
		? { [P in K]: number }
		: { [P in K]: string };
}[Field];
export type RuleAction =
	{ Tag: string } | { Alert: string } | { UrgentAlert: string } | 'Star' | 'Archive';
export interface Rule {
	version: number;
	id: string;
	revision: number;
	name: string;
	enabled: boolean;
	position: number;
	all: Condition[];
	any: Condition[];
	except: Condition[];
	actions: RuleAction[];
	stop_processing: boolean;
}
export type Outcome = 'Matched' | 'NotMatched' | 'Excluded' | 'Disabled' | 'Stopped';
export type ConditionResult = 'Matched' | 'NotMatched' | 'MissingFact';
export interface Plan {
	actions: RuleAction[];
	explanations: {
		rule_id: string;
		outcome: Outcome;
		all: ConditionResult[];
		any: ConditionResult[];
		except: ConditionResult[];
	}[];
}
export interface RulesTransport {
	list(): Promise<Rule[]>;
	save(rule: Rule): Promise<Rule>;
	remove(id: string, revision: number): Promise<void>;
	reorder(revisions: [string, number][]): Promise<void>;
	simulate(ids: string[], draft?: Rule): Promise<[string, Plan][]>;
	draft(messageId: string, ruleId: string): Promise<Rule>;
}
export function newRule(position = 0): Rule {
	return {
		version: 1,
		id: crypto.randomUUID(),
		revision: 0,
		name: '',
		enabled: false,
		position,
		all: [{ Subject: '' }],
		any: [],
		except: [],
		actions: ['Star'],
		stop_processing: false
	};
}
export function condition(field: Field, value: string): Condition {
	if (field === 'AmountAtLeast') return { AmountAtLeast: Number(value) };
	if (field === 'AmountAtMost') return { AmountAtMost: Number(value) };
	return { [field]: value } as Condition;
}
