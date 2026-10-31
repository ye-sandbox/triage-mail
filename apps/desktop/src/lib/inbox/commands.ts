import type { TriageAction } from './triage';
export type PaletteCommand = {
	label: string;
	hint: string;
	action: TriageAction | 'tag';
};
export const commands: PaletteCommand[] = [
	{ label: 'Mark read', hint: 'R', action: 'MarkRead' },
	{ label: 'Mark unread', hint: 'Shift U', action: 'MarkUnread' },
	{ label: 'Archive', hint: 'E', action: 'Archive' },
	{ label: 'Move to Trash', hint: 'Delete', action: 'Trash' },
	{ label: 'Add local tag', hint: 'T', action: 'tag' },
	{
		label: 'Send one-click unsubscribe request',
		hint: '',
		action: 'OneClickUnsubscribe'
	}
];
