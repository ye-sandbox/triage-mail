import { formatCategoryLabel, t } from '$lib/i18n';
import { i18n } from '$lib/i18n/index.svelte';
import { appStore } from '$lib/stores/app.svelte';
import type { MessageCategory } from '$lib/types';
import { labels as reviewLabels } from '../sender-review/labels';
import type { InboxWorkspace } from './workspace.svelte';

export function resolveInboxScopeLabel(workspace: InboxWorkspace): string {
	const inbox = workspace.inbox;
	if (inbox.review) {
		return `${reviewLabels[i18n.locale].title} · ${reviewLabels[i18n.locale][inbox.review]}`;
	}
	if (inbox.savedView) return inbox.savedView.name;
	if (inbox.category) return formatCategoryLabel(appStore.locale, inbox.category as MessageCategory);
	const s = appStore.activeScope;
	switch (s.kind) {
		case 'all':
			return t(appStore.locale, 'globalAllMail');
		case 'inbox':
			return t(appStore.locale, 'inbox');
		case 'starred':
			return t(appStore.locale, 'starred');
		case 'snoozed':
			return t(appStore.locale, 'snoozed');
		case 'folder': {
			const acct = appStore.accounts.find((a) => a.account_id === s.account_id);
			return `${acct?.address ?? s.account_id} / ${s.folder}`;
		}
		case 'alias':
			return s.address;
		case 'new_senders':
			return `${t(appStore.locale, 'newSendersReview')} · ${s.status}`;
		case 'category':
			return formatCategoryLabel(appStore.locale, s.category);
		case 'saved_view': {
			const v = appStore.savedViews.find((item) => item.id === s.view_id);
			return v?.name ?? t(appStore.locale, 'savedViews');
		}
	}
}
