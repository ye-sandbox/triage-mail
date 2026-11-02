import type { FolderDto, MailboxScope, MailboxView } from './types';
import type { AccountConfig } from '../onboarding/types';
import type { Translations } from '../i18n/types';

const STORAGE_KEY = 'triage-mail.mailbox-expansion';
export type Catalog = {
	folders: FolderDto[];
	loading: boolean;
	error: boolean;
	loaded: boolean;
};
export class Mailboxes {
	expanded = $state<string[]>([]);
	catalogs = $state<Record<string, Catalog>>({});
	private active = true;
	private revisions = new Map<string, number>();
	constructor(private load: (id: string) => Promise<FolderDto[]>) {
		try {
			const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]');
			if (Array.isArray(value))
				this.expanded = value
					.filter(
						(id): id is string => typeof id === 'string' && id.length <= 256 && !id.includes('@')
					)
					.slice(0, 1000);
		} catch {
			/* Storage is optional, including in private browser sessions. */
		}
	}
	toggle(id: string) {
		this.expanded = this.expanded.includes(id)
			? this.expanded.filter((item) => item !== id)
			: [...this.expanded, id];
		// INVARIANT: Persist only opaque IDs; account labels, photos and messages stay in memory.
		try {
			localStorage.setItem(
				STORAGE_KEY,
				JSON.stringify(this.expanded.filter((id) => !id.includes('@')))
			);
		} catch {
			/* Optional preference. */
		}
	}
	reconcile(accounts: AccountConfig[]) {
		const ids = new Set(accounts.map((account) => account.account_id));
		for (const id of Object.keys(this.catalogs))
			if (!ids.has(id)) {
				this.revisions.set(id, (this.revisions.get(id) ?? 0) + 1);
				delete this.catalogs[id];
			}
		for (const id of ids) if (!this.catalogs[id]) void this.refresh(id);
	}
	async refresh(id: string) {
		if (!this.active) return;
		const revision = (this.revisions.get(id) ?? 0) + 1;
		this.revisions.set(id, revision);
		const prior = this.catalogs[id];
		this.catalogs[id] = {
			folders: prior?.folders ?? [],
			loaded: prior?.loaded ?? false,
			loading: true,
			error: false
		};
		try {
			const folders = await this.load(id);
			if (this.active && revision === this.revisions.get(id))
				this.catalogs[id] = {
					folders: folders.filter((folder) => folder.account_id === id),
					loaded: true,
					loading: false,
					error: false
				};
		} catch {
			if (this.active && revision === this.revisions.get(id))
				this.catalogs[id] = {
					folders: prior?.folders ?? [],
					loaded: prior?.loaded ?? false,
					loading: false,
					error: true
				};
		}
	}
	dispose() {
		this.active = false;
		this.catalogs = {};
	}
}

export function scopeLabel(
	scope: MailboxScope | undefined,
	accounts: AccountConfig[],
	t: Translations['mailboxes'],
	catalogs: Record<string, Catalog>
) {
	if (!scope || scope === 'GlobalAllMail') return t.allMail;
	const { account_id, view } = scope.AccountView;
	const label =
		typeof view === 'string'
			? t[view]
			: (catalogs[account_id]?.folders.find((f) => f.folder_name === view.Folder)?.display_name ??
				view.Folder);
	return `${accounts.find((a) => a.account_id === account_id)?.email_display ?? ''} · ${label}`;
}
export function sameView(scope: MailboxScope | undefined, id: string, view: MailboxView) {
	return (
		!!scope &&
		scope !== 'GlobalAllMail' &&
		scope.AccountView.account_id === id &&
		JSON.stringify(scope.AccountView.view) === JSON.stringify(view)
	);
}
export function folderGroups(folders: FolderDto[]) {
	const selectable = folders.filter(
		(f) => f.is_selectable && f.role !== 'inbox' && f.folder_name.toUpperCase() !== 'INBOX'
	);
	return {
		special: selectable.filter((f) => ['junk', 'spam', 'archive', 'trash'].includes(f.role)),
		other: selectable.filter((f) => !['junk', 'spam', 'archive', 'trash'].includes(f.role))
	};
}
