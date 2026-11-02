import { invoke } from '@tauri-apps/api/core';
import type { RulesTransport, Rule, Plan } from './types';
export const nativeRulesTransport: RulesTransport = {
	list: () => invoke<Rule[]>('cmd_list_rules'),
	save: (rule) => invoke<Rule>('cmd_save_rule', { rule }),
	remove: (id, revision) => invoke<void>('cmd_delete_rule', { id, revision }),
	reorder: (revisions) => invoke<void>('cmd_reorder_rules', { revisions }),
	simulate: (ids, draft) =>
		invoke<[string, Plan][]>('cmd_simulate_rules', {
			ids,
			draft: draft ?? null
		}),
	draft: (messageId, ruleId) => invoke<Rule>('cmd_rule_draft', { messageId, ruleId })
};
