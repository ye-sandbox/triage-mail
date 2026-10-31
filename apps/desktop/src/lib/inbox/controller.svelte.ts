import type { InboxTransport, MessageRowDto, SearchMode, SyncProgressEvent } from './types';
import { validTag, type TriageAction } from './triage';
export const PAGE_SIZE = 200;

export class InboxController {
	rows = $state.raw<MessageRowDto[]>([]);
	selectedId = $state<string | null>(null);
	loading = $state(false);
	hasMore = $state(false);
	error = $state('');
	syncStatus = $state('Connecting…');
	query = $state('');
	mode = $state<SearchMode>('Lexical');
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

	async mutate(id: string, action: TriageAction) {
		if (this.disposed || this.mutating || this.loading) return;
		const original = this.rows.find((row) => row.id === id);
		if (!original || (typeof action === 'object' && !validTag(action.Tag))) return;
		// INVARIANT: Invalidate reads begun before this optimistic write; defer sync until it settles.
		const generation = ++this.generation;
		this.mutating = true;
		this.mutationError = '';
		this.mutationNotice = '';
		this.applyOptimistic(id, action);
		try {
			await this.transport.triage({ message_ids: [id], action });
			if (!this.disposed)
				this.mutationNotice =
					action === 'OneClickUnsubscribe'
						? 'Unsubscribe request sent.'
						: typeof action === 'object'
							? 'Local tag saved.'
							: 'Saved locally. Server changes sync in the background.';
		} catch {
			if (this.disposed) return;
			if (generation === this.generation) {
				this.rows = this.rows.map((row) => (row.id === id ? original : row));
				this.rehydrateFeedback();
			}
			this.mutationError =
				action === 'OneClickUnsubscribe'
					? 'Unsubscribe could not be confirmed. Check before trying again.'
					: 'Change could not be saved. Please try again.';
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
			if (action === 'Archive' || action === 'Trash') return { ...row, folder_override: action };
			if (typeof action === 'object')
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

	async search(query: string, mode: SearchMode) {
		const generation = ++this.generation;
		this.query = query;
		this.mode = mode;
		this.rows = [];
		this.rehydrateFeedback();
		this.selectedId = null;
		this.hasMore = true;
		this.error = '';
		this.loading = true;
		this.revision++;
		try {
			const rows = await this.transport.search({
				query,
				mode,
				limit: PAGE_SIZE,
				offset: 0
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
			const next = await this.transport.search({
				query: this.query,
				mode: this.mode,
				limit: PAGE_SIZE,
				offset
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
				const page = await this.transport.search({
					query: this.query,
					mode: this.mode,
					limit: PAGE_SIZE,
					offset
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
		const labels = {
			Syncing: 'Syncing mail…',
			Idle: 'Up to date',
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
		if (error === 'EmbeddingUnavailable')
			return 'Meaning search needs a local model. Choose Words to search without it.';
		if (error === 'VaultUnavailable') return 'Unlock your local vault to access mail.';
		return 'Mail could not be loaded. Please try again.';
	}
	dispose() {
		this.disposed = true;
		this.generation++;
		clearTimeout(this.refreshTimer);
		this.disconnect?.();
	}
}
