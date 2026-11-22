import { t } from '$lib/i18n';
import { i18n } from '$lib/i18n/index.svelte';
import { appStore } from '$lib/stores/app.svelte';
import { labels as reviewLabels } from '../sender-review/labels';
import type { InboxWorkspace } from './workspace.svelte';

export function resolveInboxScopeLabel(workspace: InboxWorkspace): string {
	const inbox = workspace.inbox;
	if (inbox.review) {
		return `${reviewLabels[i18n.locale].title} · ${reviewLabels[i18n.locale][inbox.review]}`;
	}
	if (inbox.savedView) return inbox.savedView.name;
	if (inbox.category) return appStore.formatCategoryDisplay(inbox.category);
	if (inbox.customTag) return inbox.customTag;
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
			return appStore.formatCategoryDisplay(s.category);
		case 'custom_category':
			return appStore.formatCategoryDisplay(s.category_id);
		case 'saved_view': {
			const v = appStore.savedViews.find((item) => item.id === s.view_id);
			return v?.name ?? t(appStore.locale, 'savedViews');
		}
	}
}
