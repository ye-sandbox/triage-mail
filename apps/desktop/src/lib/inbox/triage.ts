export type TriageAction =
	'MarkRead' | 'MarkUnread' | 'Archive' | 'Trash' | 'OneClickUnsubscribe' | { Tag: string };
export type TriageMutation = { message_ids: string[]; action: TriageAction };
export function validTag(tag: string) {
	return (
		!!tag.trim() &&
		new TextEncoder().encode(tag).length <= 128 &&
		!/[\u0000-\u001f\u007f-\u009f]/u.test(tag)
	);
}
export function shortcut(event: KeyboardEvent): TriageAction | 'palette' | 'tag' | null {
	if (event.isComposing || event.repeat || event.altKey) return null;
	if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') return 'palette';
	if (event.metaKey || event.ctrlKey) return null;
	const target = event.target;
	if (
		target instanceof Element &&
		target.closest('input, textarea, select, [contenteditable], iframe, [role=dialog]')
	)
		return null;
	if (event.shiftKey && event.key.toLowerCase() === 'u') return 'MarkUnread';
	if (event.shiftKey) return null;
	return (
		({ e: 'Archive', r: 'MarkRead', t: 'tag', Delete: 'Trash' } as const)[
			event.key as 'e' | 'r' | 't' | 'Delete'
		] ?? null
	);
}
