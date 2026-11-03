import type { ConversationContext, ConversationRow } from '../conversations/types';
import { labels as conversationLabels } from '../conversations/labels';
import type { ReviewFilter } from '../sender-review/types';
import type { Category } from '../categories/types';
import type { SavedView } from '../saved-views/types';
import type {
	InboxTransport,
	MailboxScope,
	MessageRowDto,
	SearchMode,
	SyncProgressEvent
} from './types';
import { i18n } from '../i18n/index.svelte';
import { validTag, type TriageAction } from './triage';
export const PAGE_SIZE = 200;

export class InboxController {
	conversations = $state(false);
	conversationGeneration = $state<string | null>(null);
	conversationRows = $state.raw<Record<string, ConversationRow>>({});
	get conversationTransport() {
		return this.transport.conversations;
	}
	conversationContext(generation = this.conversationGeneration): ConversationContext {
		return {
			source: this.review
				? { Review: this.review }
				: this.savedView
					? {
							Saved: {
								id: this.savedView.id,
								revision: this.savedView.revision
							}
						}
					: this.category
						? { Category: this.category }
						: 'Messages',
			scope: this.scope ?? null,
			generation
		};
	}
	conversationQuery() {
		return {
			query: this.query,
			mode: this.mode,
			limit: PAGE_SIZE,
			offset: 0,
			...this.scopeField()
		};
	}
	async toggleConversations() {
		this.conversations = !this.conversations;
		await this.search(this.query, this.mode);
	}
	async mutateMember(id: string, action: TriageAction) {
		await this.updateCategory(() => this.transport.triage({ message_ids: [id], action }));
	}

	review = $state<ReviewFilter | null>(null);
	get reviewTransport() {
		return this.transport.review;
	}
	async selectReview(state: ReviewFilter | null) {
		this.review = state;
		this.category = null;
		this.savedView = null;
		this.mutationError = '';
		this.mutationNotice = '';
		clearTimeout(this.refreshTimer);
		await this.search(this.query, this.mode);
	}

	category = $state<Category | null>(null);
	get categoryTransport() {
		return this.transport.categories;
	}
	async selectCategory(category: Category | null) {
		this.category = category;
		this.review = null;
		this.savedView = null;
		this.mutationError = '';
		this.mutationNotice = '';
		clearTimeout(this.refreshTimer);
		await this.search(this.query, this.mode);
	}
	async updateCategory(write: () => Promise<unknown>) {
		if (this.disposed || this.mutating || this.loading) return;
		const generation = ++this.generation;
		this.mutating = true;
		this.mutationError = '';
		this.mutationNotice = '';
		try {
			await write();
			if (!this.disposed && generation === this.generation)
				this.mutationNotice = i18n.t.mailboxes.localSaved;
		} catch {
			if (!this.disposed && generation === this.generation)
				this.mutationError = i18n.t.mailboxes.saveError;
		} finally {
			this.mutating = false;
			if (!this.disposed) {
				this.badgeRevision++;
				this.scheduleRefresh();
			}
		}
	}

	rows = $state.raw<MessageRowDto[]>([]);
	selectedId = $state<string | null>(null);
	loading = $state(false);
	hasMore = $state(false);
	error = $state('');
	syncStatus = $state('Connecting…');
	query = $state('');
	mode = $state<SearchMode>('Lexical');
	savedView = $state.raw<SavedView | null>(null);
	get savedTransport() {
		return this.transport.savedViews;
	}
	async selectSavedView(view: SavedView) {
		this.savedView = view;
		this.category = null;
		this.review = null;
		this.scope = undefined;
		this.mutationError = '';
		this.mutationNotice = '';
		clearTimeout(this.refreshTimer);
		await this.search(view.query, view.mode);
	}
	scope = $state.raw<MailboxScope | undefined>(undefined);
	experimentalSemanticSearch = $state(false);
	totalEmbeddingErrors = $state(0);
	revision = $state(0);
	badgeRevision = $state(0);
	mutating = $state(false);
	mutationError = $state('');
	mutationNotice = $state('');
	localTags = $state<Record<string, string[]>>({});
	folderLabels = $state<Record<string, string>>({});
	private generation = 0;
	private disposed = false;
	private refreshTimer: ReturnType<typeof setTimeout> | undefined;
	private disconnect?: () => void;
	private refreshing = false;
	private refreshAgain = false;
	constructor(private transport: InboxTransport) {}

	listFolders = (accountId: string) =>
		this.transport.listFolders
			? this.transport.listFolders(accountId)
			: Promise.reject(new Error('Unavailable'));

	async selectScope(scope?: MailboxScope) {
		if (
			!this.review &&
			!this.category &&
			!this.savedView &&
			JSON.stringify(scope) === JSON.stringify(this.scope)
		)
			return;
		this.savedView = null;
		this.category = null;
		this.review = null;
		// INVARIANT: Scope changes invalidate all old reads, selection and mutation feedback.
		this.scope = scope === 'GlobalAllMail' ? undefined : scope;
		this.mutationError = '';
		this.mutationNotice = '';
		clearTimeout(this.refreshTimer);
		await this.search(this.query, this.mode);
	}

	private request(page: {
		query: string;
		mode: SearchMode;
		limit: number;
		offset: number;
		scope?: MailboxScope;
	}) {
		if (this.conversations) {
			const generation = this.generation;
			const context = this.conversationContext(
				page.offset === 0 ? null : this.conversationGeneration
			);
			return this.transport.conversations
				? this.transport.conversations.search(page, context).then((result) => {
						if (generation === this.generation && !this.disposed) {
							this.conversationGeneration = result.generation;
							this.conversationRows = {
								...(page.offset === 0 ? {} : this.conversationRows),
								...Object.fromEntries(result.rows.map((row) => [row.representative.id, row]))
							};
						}
						return result.rows.map((row) => row.representative);
					})
				: Promise.reject('InvalidRequest');
		}
		if (this.review)
			return this.transport.review
				? this.transport.review.search(page, this.review)
				: Promise.reject('InvalidRequest');
		const saved = this.savedView;
		if (saved)
			return this.transport.savedViews
				? this.transport.savedViews.search(saved.id, saved.revision, page.limit, page.offset)
				: Promise.reject('InvalidRequest');
		if (this.category)
			return this.transport.categories
				? this.transport.categories.search(page, this.category)
				: Promise.reject('InvalidRequest');
		return this.transport.search(page);
	}

	private scopeField() {
		return this.scope ? { scope: this.scope } : {};
	}

	async mutate(id: string, action: TriageAction) {
		if (this.disposed || this.mutating || this.loading) return;
		const original = this.rows.find((row) => row.id === id);
		if (!original || (typeof action === 'object' && 'Tag' in action && !validTag(action.Tag)))
			return;
		// INVARIANT: Invalidate reads begun before this optimistic write; defer sync until it settles.
		const generation = ++this.generation;
		this.mutating = true;
		this.mutationError = '';
		this.mutationNotice = '';
		this.applyOptimistic(id, action);
		try {
			await this.transport.triage({ message_ids: [id], action });
			if (!this.disposed && generation === this.generation)
				this.mutationNotice =
					action === 'OneClickUnsubscribe'
						? 'Unsubscribe request sent.'
						: typeof action === 'object' && 'Tag' in action
							? i18n.t.mailboxes.localSaved
							: (typeof action === 'object' && 'Snooze' in action) || action === 'Unsnooze'
								? i18n.t.mailboxes.localSaved
								: i18n.t.mailboxes.saved;
		} catch {
			if (this.disposed) return;
			if (generation === this.generation) {
				this.rows = this.rows.map((row) => (row.id === id ? original : row));
				this.rehydrateFeedback();
			}
			if (generation !== this.generation) return;
			this.mutationError =
				action === 'OneClickUnsubscribe'
					? 'Unsubscribe could not be confirmed. Check before trying again.'
					: i18n.t.mailboxes.saveError;
		} finally {
			this.mutating = false;
			if (!this.disposed) this.scheduleRefresh();
		}
	}

	private applyOptimistic(id: string, action: TriageAction) {
		// INVARIANT: Display metadata never overwrites canonical IMAP attachment identity.
		this.rows = this.rows.map((row) => {
			if (row.id !== id) return row;
			if (action === 'MarkRead' || action === 'MarkUnread')
				return { ...row, is_read: action === 'MarkRead' };
			if (action === 'Archive' || action === 'Trash')
				return {
					...row,
					folder_override: action,
					...(row.snooze_until_epoch_ms !== undefined ? { snooze_until_epoch_ms: null } : {})
				};
			if (action === 'Star' || action === 'Unstar')
				return { ...row, is_starred: action === 'Star' };
			if (action === 'Unsnooze') return { ...row, snooze_until_epoch_ms: null };
			if (typeof action === 'object' && 'Snooze' in action)
				return { ...row, snooze_until_epoch_ms: action.Snooze.until_epoch_ms };
			if (typeof action === 'object' && 'Tag' in action)
				return { ...row, tags: [...new Set([...row.tags, action.Tag])].sort() };
			return row;
		});
		this.rehydrateFeedback();
	}

	async connect() {
		try {
			const stop = await this.transport.subscribe((event) => this.onSync(event));
			if (this.disposed) stop();
			else {
				this.disconnect = stop;
				this.syncStatus = 'Up to date';
			}
		} catch {
			if (!this.disposed) this.syncStatus = 'Live updates unavailable';
		}
		if (!this.disposed && this.generation === 0) await this.search('', 'Lexical');
	}

	setExperimentalSemanticSearch(enabled: boolean) {
		this.experimentalSemanticSearch = enabled;
		if (!enabled && this.mode !== 'Lexical' && !this.savedView) {
			this.mode = 'Lexical';
			void this.search(this.query, 'Lexical');
		}
	}

	async search(query: string, mode: SearchMode) {
		const generation = ++this.generation;
		if (!this.savedView && !this.experimentalSemanticSearch && mode !== 'Lexical') {
			mode = 'Lexical';
		}
		this.query = query;
		this.mode = mode;
		this.conversationGeneration = null;
		this.conversationRows = {};
		this.rows = [];
		this.rehydrateFeedback();
		this.selectedId = null;
		this.hasMore = true;
		this.error = '';
		this.loading = true;
		this.revision++;
		try {
			const rows = await this.request({
				query,
				mode,
				limit: PAGE_SIZE,
				offset: 0,
				...this.scopeField()
			});
			if (generation !== this.generation || this.disposed) return;
			this.rows = rows;
			this.rehydrateFeedback();
			this.hasMore = rows.length === PAGE_SIZE;
		} catch (error) {
			if (generation === this.generation && !this.disposed) this.error = this.errorMessage(error);
		} finally {
			if (generation === this.generation && !this.disposed) this.loading = false;
		}
		if (this.refreshAgain && !this.disposed) {
			this.refreshAgain = false;
			this.scheduleRefresh();
		}
	}

	async loadMore() {
		if (
			this.loading ||
			!this.hasMore ||
			this.disposed ||
			this.error ||
			this.refreshing ||
			this.mutating
		)
			return;
		const generation = this.generation;
		const offset = this.rows.length;
		this.loading = true;
		try {
			const next = await this.request({
				query: this.query,
				mode: this.mode,
				limit: PAGE_SIZE,
				offset,
				...this.scopeField()
			});
			if (generation !== this.generation || this.disposed) return;
			const known = new Set(this.rows.map((row) => row.id));
			// INVARIANT: Overlapping pages during sync never create duplicate keyed DOM nodes.
			this.rows = [...this.rows, ...next.filter((row) => !known.has(row.id))];
			this.rehydrateFeedback();
			this.hasMore = next.length === PAGE_SIZE && this.rows.length > offset;
		} catch (error) {
			if (generation === this.generation && !this.disposed) this.error = this.errorMessage(error);
		} finally {
			if (generation === this.generation && !this.disposed) this.loading = false;
		}
		if (this.refreshAgain) {
			this.refreshAgain = false;
			this.scheduleRefresh();
		}
	}

	async refresh() {
		if (this.disposed) return;
		if (this.loading || this.refreshing || this.mutating) {
			this.refreshAgain = true;
			return;
		}
		const generation = this.generation;
		const target = Math.max(PAGE_SIZE, this.rows.length);
		this.refreshing = true;
		try {
			const rows: MessageRowDto[] = [];
			let hasMore = false;
			for (let offset = 0; offset < target; offset += PAGE_SIZE) {
				const page = await this.request({
					query: this.query,
					mode: this.mode,
					limit: PAGE_SIZE,
					offset,
					...this.scopeField()
				});
				if (generation !== this.generation || this.disposed) break;
				rows.push(...page);
				hasMore = page.length === PAGE_SIZE;
				if (page.length < PAGE_SIZE) break;
			}
			if (generation !== this.generation || this.disposed) return;
			const unique = new Map(rows.map((row) => [row.id, row]));
			this.rows = [...unique.values()];
			this.rehydrateFeedback();
			this.badgeRevision++;
			this.hasMore = hasMore;
			if (this.selectedId && !unique.has(this.selectedId)) this.selectedId = null;
			this.error = '';
		} catch (error) {
			if (generation === this.generation && !this.disposed) this.error = this.errorMessage(error);
		} finally {
			this.refreshing = false;
			if (this.refreshAgain && !this.disposed) {
				this.refreshAgain = false;
				this.scheduleRefresh();
			}
		}
	}

	private rehydrateFeedback() {
		// INVARIANT: Only accepted authoritative pages replace optimistic display metadata.
		this.localTags = Object.fromEntries(this.rows.map((row) => [row.id, [...row.tags]]));
		this.folderLabels = Object.fromEntries(
			this.rows.flatMap((row) =>
				row.folder_override === null ? [] : [[row.id, row.folder_override]]
			)
		);
	}

	private onSync(event: SyncProgressEvent) {
		if (this.disposed) return;
		if (event.embedding_errors_count && event.embedding_errors_count > 0) {
			this.totalEmbeddingErrors += event.embedding_errors_count;
		}
		const labels = {
			Syncing: 'Syncing mail…',
			Idle:
				this.totalEmbeddingErrors > 0
					? `Up to date (${this.totalEmbeddingErrors} indexing warning${this.totalEmbeddingErrors > 1 ? 's' : ''})`
					: 'Up to date',
			Retrying: 'Reconnecting…',
			MutationPending: 'Saving changes…',
			MutationFailed: 'Some changes need attention',
			ResyncRequired: 'Updating inbox…'
		};
		this.syncStatus = labels[event.state];
		this.scheduleRefresh();
	}
	private scheduleRefresh() {
		clearTimeout(this.refreshTimer);
		this.refreshTimer = setTimeout(() => void this.refresh(), 300);
	}
	private errorMessage(error: unknown) {
		// INVARIANT: Never display arbitrary IPC errors that could contain private account data.
		if (this.conversations && error === 'InvalidRequest')
			return conversationLabels[i18n.locale].unavailable;
		if (error === 'EmbeddingUnavailable')
			return 'Experimental semantic search needs a local model. Choose Words to search without it.';
		if (error === 'VaultUnavailable') return i18n.t.mailboxes.vaultError;
		if (error === 'InvalidRequest' && (this.scope || this.savedView))
			return i18n.t.mailboxes.unavailable;
		return i18n.t.mailboxes.loadError;
	}
	dispose() {
		this.disposed = true;
		this.generation++;
		clearTimeout(this.refreshTimer);
		this.disconnect?.();
	}
}
