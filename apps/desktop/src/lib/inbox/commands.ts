import { i18n } from '../i18n/index.svelte';
import type { TriageAction } from './triage';
export type PaletteCommand = {
	label: string;
	hint: string;
	action: TriageAction | 'tag' | 'snooze';
};
export function mailboxCommands(): PaletteCommand[] {
	const t = i18n.t.mailboxes;
	return [
		{ label: t.markRead, hint: 'R', action: 'MarkRead' },
		{ label: t.markUnread, hint: 'Shift U', action: 'MarkUnread' },
		{ label: t.archive, hint: 'E', action: 'Archive' },
		{ label: t.moveTrash, hint: 'Delete', action: 'Trash' },
		{ label: t.tag, hint: 'T', action: 'tag' },
		{
			label: t.unsubscribe,
			hint: '',
			action: 'OneClickUnsubscribe'
		},
		{ label: t.star, hint: '', action: 'Star' },
		{ label: t.unstar, hint: '', action: 'Unstar' },
		{ label: t.snooze, hint: '', action: 'snooze' },
		{ label: t.unsnooze, hint: '', action: 'Unsnooze' }
	];
}
