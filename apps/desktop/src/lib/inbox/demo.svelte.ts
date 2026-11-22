import { toast } from 'svelte-sonner';
import { appStore } from '$lib/stores/app.svelte';
import type { EmailMessage, ScopeFilter } from '$lib/types';
import type { Category, CategoryMetadata, CategoryTransport } from '../categories/types';
import type { ConversationTransport } from '../conversations/types';
import type { MessageDetailDto, ReaderTransport, TriageBadge } from '../reader/types';
import type { ReviewFilter, ReviewTransport, SenderChoice } from '../sender-review/types';
import { blankView, type SavedViewTransport } from '../saved-views/types';
import type { Settings, SettingsTransport } from '../settings/types';
import { INBOX_FIXTURES } from './inbox.fixtures';
import type { InboxTransport, MessageRowDto, SearchQueryDto } from './types';

function toBadges(msg: EmailMessage): TriageBadge[] {
	const badges: TriageBadge[] = [];
	if (msg.extractions.otp) badges.push({ OtpCode: { code: msg.extractions.otp.code } });
	if (msg.extractions.boleto) {
		badges.push({
			FebrabanBoleto: {
				digitable_line: msg.extractions.boleto.digitable_line,
				amount_cents: msg.extractions.boleto.amount_cents,
				due_date_iso: msg.extractions.boleto.due_date_iso
			}
		});
	}
	if (msg.extractions.pix) badges.push({ PixPayload: { payload: msg.extractions.pix.payload } });
	if (msg.extractions.nfe)
		badges.push({ NfeInvoice: { access_key: msg.extractions.nfe.access_key } });
	if (msg.extractions.tracking) {
		badges.push({
			TrackingCode: {
				carrier: msg.extractions.tracking.carrier,
				code: msg.extractions.tracking.code
			}
		});
	}
	if (msg.extractions.unsubscribe) {
		badges.push({
			UnsubscribeAvailable: {
				supports_one_click: msg.extractions.unsubscribe.supports_one_click
			}
		});
	}
	return badges;
}

function toRow(msg: EmailMessage): MessageRowDto {
	return {
		id: msg.id,
		account_id: msg.account_id,
		folder: msg.folder,
		uid: Math.max(1, INBOX_FIXTURES.findIndex((f) => f.id === msg.id) + 1),
		subject: msg.subject,
		sender: `${msg.sender_name} <${msg.sender_email}>`,
		received_at_epoch_ms: new Date(msg.received_at_iso).getTime(),
		is_read: !msg.unread,
		is_starred: msg.starred,
		snooze_until_epoch_ms: msg.snoozed_until_iso ? new Date(msg.snoozed_until_iso).getTime() : null,
		has_attachments: msg.attachments_meta.length > 0,
		category_badge: msg.category,
		tags: [...msg.tags],
		folder_override: null,
		triage_badges: toBadges(msg)
	};
}

function matchesSearch(msg: EmailMessage, q: string): boolean {
	if (!q) return true;
	const haystack = [
		msg.sender_name,
		msg.sender_email,
		msg.subject,
		msg.snippet,
		msg.category,
		...msg.tags,
		msg.extractions.otp?.code ?? '',
		msg.extractions.tracking?.code ?? '',
		msg.extractions.boleto?.digitable_line ?? ''
	]
		.join(' ')
		.toLowerCase();
	return haystack.includes(q);
}

function matchesShellScope(msg: EmailMessage, scope: ScopeFilter): boolean {
	const senderGov = appStore.senderGovernance.find((s) => s.sender_email === msg.sender_email);
	const effectiveStatus = senderGov?.status ?? msg.sender_status;

	if (scope.kind === 'inbox' && msg.folder !== 'INBOX') return false;
	if (scope.kind === 'starred' && !msg.starred) return false;
	if (scope.kind === 'snoozed' && !msg.snoozed_until_iso) return false;
	if (
		scope.kind === 'folder' &&
		(msg.account_id !== scope.account_id || msg.folder !== scope.folder)
	) {
		return false;
	}
	if (scope.kind === 'new_senders' && effectiveStatus !== scope.status) return false;
	if (scope.kind === 'category' && msg.category !== scope.category) return false;
	if (scope.kind === 'saved_view') {
		const view = appStore.savedViews.find((v) => v.id === scope.view_id);
		if (view?.category && msg.category !== view.category) return false;
		if (view?.has_boleto && !msg.extractions.boleto) return false;
	}
	return true;
}

export interface DemoBackend {
	inboxTransport: InboxTransport;
	readerTransport: ReaderTransport;
	settingsTransport: SettingsTransport;
	getMessages: () => EmailMessage[];
	ensureMessage: (msg: EmailMessage) => void;
	markBoletoPaid: (id: string, paid: boolean) => void;
}

export function createDemoBackend(): DemoBackend {
	let messages = $state<EmailMessage[]>(structuredClone(INBOX_FIXTURES));
	const categoryRevisions = new Map<string, number>();

	let demoSettings: Settings = {
		ai: {
			enabled: true,
			provider: 'Ollama',
			endpoint: 'http://127.0.0.1:11434',
			model: 'llama3.2'
		},
		tokenizer: 'Unicode61RemoveDiacritics',
		has_api_key: false,
		model_cached: false,
		experimental_semantic_search: false
	};

	function filterRows(query: SearchQueryDto, extraFilter?: (m: EmailMessage) => boolean): MessageRowDto[] {
		const q = query.query.trim().toLowerCase();
		return messages
			.filter((msg) => {
				if (extraFilter && !extraFilter(msg)) return false;
				if (!extraFilter) {
					if (query.scope && query.scope !== 'GlobalAllMail') {
						const { account_id, view } = query.scope.AccountView;
						if (msg.account_id !== account_id) return false;
						if (view === 'Inbox' && msg.folder !== 'INBOX') return false;
						if (view === 'Starred' && !msg.starred) return false;
						if (view === 'Snoozed' && !msg.snoozed_until_iso) return false;
						if (typeof view === 'object' && 'Folder' in view && msg.folder !== view.Folder) return false;
					} else if (!matchesShellScope(msg, appStore.activeScope)) {
						return false;
					}
				}
				return matchesSearch(msg, q);
			})
			.slice(query.offset, query.offset + query.limit)
			.map(toRow);
	}

	const review: ReviewTransport = {
		preferences: async (account_id) => ({
			account_id,
			revision: 1,
			enabled: true,
			hide_in_inbox: false,
			initializing: false
		}),
		savePreferences: async (value) => value,
		metadata: async (messageId) => {
			const msg = messages.find((m) => m.id === messageId);
			const gov = msg
				? appStore.senderGovernance.find((s) => s.sender_email === msg.sender_email)
				: undefined;
			const status = gov?.status ?? msg?.sender_status ?? 'Approved';
			const state: SenderChoice['state'] =
				status === 'Pending' ? 'Pending' : status === 'Declined' ? 'Declined' : 'Accepted';
			return {
				account_id: msg?.account_id ?? 'acct_work_01',
				sender: msg?.sender_email ?? 'sender@example.com',
				state,
				revision: 1
			} satisfies SenderChoice;
		},
		list: async () => [],
		save: async (choice) => {
			const mappedStatus =
				choice.state === 'Pending'
					? 'Pending'
					: choice.state === 'Declined'
						? 'Declined'
						: 'Approved';
			appStore.setSenderStatus(choice.sender, mappedStatus);
			messages = messages.map((m) =>
				m.sender_email === choice.sender ? { ...m, sender_status: mappedStatus } : m
			);
			toast.success(`Sender ${choice.sender} marked as ${choice.state}`);
			return { ...choice, revision: choice.revision + 1 };
		},
		search: async (query, state: ReviewFilter) =>
			filterRows(query, (msg) => {
				const gov = appStore.senderGovernance.find((s) => s.sender_email === msg.sender_email);
				return (gov?.status ?? msg.sender_status) === state;
			})
	};

	const categories: CategoryTransport = {
		metadata: async (messageId) => {
			const msg = messages.find((m) => m.id === messageId);
			const effective = (msg?.category ?? 'Personal') as Category;
			const rev = categoryRevisions.get(messageId) ?? 0;
			return {
				automatic: effective,
				effective,
				source: rev > 0 ? 'Message' : 'Automatic',
				reason: 'Fallback',
				version: 1,
				durable: true,
				sender: msg?.sender_email ?? null,
				correction_revision: rev,
				sender_revision: 0
			} satisfies CategoryMetadata;
		},
		correct: async (messageId, revision, category) => {
			const nextCat = category ?? 'Personal';
			messages = messages.map((m) => (m.id === messageId ? { ...m, category: nextCat } : m));
			const nextRev = revision + 1;
			categoryRevisions.set(messageId, nextRev);
			return {
				automatic: nextCat,
				effective: nextCat,
				source: category ? 'Message' : 'Automatic',
				reason: 'Fallback',
				version: 1,
				durable: true,
				sender: messages.find((m) => m.id === messageId)?.sender_email ?? null,
				correction_revision: nextRev,
				sender_revision: 0
			} satisfies CategoryMetadata;
		},
		list: async () => [],
		save: async (v) => v,
		search: async (query, category) => filterRows(query, (msg) => msg.category === category)
	};

	const savedViews: SavedViewTransport = {
		list: async () =>
			appStore.savedViews.map((v) => ({
				...blankView(v.query, 'Lexical', 'GlobalAllMail'),
				id: v.id,
				name: v.name,
				revision: 1
			})),
		save: async (v) => ({ ...v, revision: v.revision + 1 }),
		remove: async () => {},
		search: async (id, _rev, limit, offset) => {
			const view = appStore.savedViews.find((v) => v.id === id);
			return filterRows({ query: view?.query ?? '', mode: 'Lexical', limit, offset }, (msg) => {
				if (view?.category && msg.category !== view.category) return false;
				if (view?.has_boleto && !msg.extractions.boleto) return false;
				return true;
			});
		}
	};

	const conversations: ConversationTransport = {
		progress: async () => ({ indexed: messages.length, total: messages.length, ready: true }),
		search: async (query) => {
			const filtered = filterRows(query);
			const buckets = new Map<string, MessageRowDto[]>();
			for (const row of filtered) {
				const msg = messages.find((m) => m.id === row.id);
				const tid = msg?.thread_id ?? row.id;
				const list = buckets.get(tid) ?? [];
				list.push(row);
				buckets.set(tid, list);
			}
			return {
				generation: 'demo-gen-1',
				rows: Array.from(buckets.entries()).map(([conversation_id, group]) => ({
					conversation_id,
					representative: group[0],
					matching_count: group.length,
					total_count: group.length,
					unread_count: group.filter((r) => !r.is_read).length
				}))
			};
		},
		members: async (_q, _ctx, conversationId) => {
			const group = messages.filter((m) => m.thread_id === conversationId).map(toRow);
			return {
				generation: 'demo-gen-1',
				rows: group.map((message) => ({ message, matching: true, pending: false })),
				total: group.length
			};
		},
		preview: async (_q, _ctx, id, _all, action) => {
			const group = messages.filter((m) => m.thread_id === id || m.id === id).map(toRow);
			return {
				token: `demo-token-${id}`,
				generation: 'demo-gen-1',
				action,
				messages: group,
				outside_count: 0
			};
		},
		apply: async () => [],
		status: async (messageIds) =>
			messageIds.map((message_id) => ({
				message_id,
				pending: false,
				failed: false,
				attempts: 0
			}))
	};

	const inboxTransport: InboxTransport = {
		conversations,
		review,
		categories,
		savedViews,
		listFolders: async (accountId) => {
			const acct = appStore.accounts.find((a) => a.account_id === accountId);
			return (acct?.folders ?? []).map((f) => ({
				account_id: accountId,
				folder_name: f.id,
				display_name: f.label,
				delimiter: '/',
				role: f.id.toLowerCase(),
				is_selectable: true,
				total_messages: messages.filter((m) => m.account_id === accountId && m.folder === f.id)
					.length,
				unread_messages: messages.filter(
					(m) => m.account_id === accountId && m.folder === f.id && m.unread
				).length
			}));
		},
		search: async (query) => filterRows(query),
		subscribe: async () => () => {},
		triage: async (mutation) => {
			for (const id of mutation.message_ids) {
				const action = mutation.action;
				messages = messages.map((m) => {
					if (m.id !== id) return m;
					if (action === 'MarkRead') return { ...m, unread: false };
					if (action === 'MarkUnread') return { ...m, unread: true };
					if (action === 'Archive') return { ...m, folder: 'Archive', snoozed_until_iso: null };
					if (action === 'Trash') return { ...m, folder: 'Trash', snoozed_until_iso: null };
					if (action === 'Star') return { ...m, starred: true };
					if (action === 'Unstar') return { ...m, starred: false };
					if (action === 'Unsnooze') return { ...m, snoozed_until_iso: null };
					if (typeof action === 'object' && 'Snooze' in action) {
						return {
							...m,
							snoozed_until_iso: new Date(action.Snooze.until_epoch_ms).toISOString()
						};
					}
					if (typeof action === 'object' && 'Tag' in action) {
						return { ...m, tags: [...new Set([...m.tags, action.Tag])].sort() };
					}
					if (action === 'OneClickUnsubscribe' && m.extractions.unsubscribe) {
						return {
							...m,
							extractions: {
								...m.extractions,
								unsubscribe: { ...m.extractions.unsubscribe, unsubscribed: true }
							}
						};
					}
					return m;
				});
			}
		}
	};

	const readerTransport: ReaderTransport = {
		body: async (messageId) => {
			const msg = messages.find((m) => m.id === messageId);
			if (!msg) throw new Error('Message not found');
			const badges = toBadges(msg);
			return {
				id: msg.id,
				subject: msg.subject,
				sender: `${msg.sender_name} <${msg.sender_email}>`,
				recipients: appStore.accounts.find((a) => a.account_id === msg.account_id)?.address ?? '',
				headers: `From: ${msg.sender_name} <${msg.sender_email}>\nSubject: ${msg.subject}`,
				text_plain: msg.text_fallback,
				html_raw: msg.html_raw,
				triage_result: {
					category: msg.category,
					badges
				},
				triage_badges: badges,
				attachments_meta: msg.attachments_meta.map((att) => ({
					part_id: att.id,
					filename: att.filename,
					mime_type: att.mime_type,
					size_bytes: att.size_bytes,
					content_id: null,
					is_inline: false
				}))
			} satisfies MessageDetailDto;
		},
		attachment: async (req) => {
			const msg = messages.find((m) => m.account_id === req.account_id);
			if (msg) {
				messages = messages.map((m) =>
					m.id === msg.id
						? {
								...m,
								attachments_meta: m.attachments_meta.map((a) =>
									a.id === req.part_id ? { ...a, downloaded: true } : a
								)
							}
						: m
				);
			}
			const sample = new TextEncoder().encode(`Synthetic attachment part ${req.part_id}\n`);
			return { bytes: Array.from(sample) };
		},
		saveAttachment: async (req) => {
			const msg = messages.find((m) => m.account_id === req.account_id);
			if (msg) {
				messages = messages.map((m) =>
					m.id === msg.id
						? {
								...m,
								attachments_meta: m.attachments_meta.map((a) =>
									a.id === req.part_id ? { ...a, downloaded: true } : a
								)
							}
						: m
				);
			}
			const sample = new TextEncoder().encode(`Synthetic attachment part ${req.part_id}\n`);
			return { bytes_written: sample.byteLength };
		},
		summarize: async (messageId) => {
			const msg = messages.find((m) => m.id === messageId);
			if (!msg) return 'Local Summary: Message unavailable.';
			if (msg.extractions.boleto) {
				return `Local Summary: Monthly utility invoice from ${msg.sender_name} due ${msg.extractions.boleto.due_date_iso}. Extracted 47-digit FEBRABAN line and attached PDF ready for on-demand fetch.`;
			}
			if (msg.extractions.otp) {
				return `Local Summary: One-time authentication code (${msg.extractions.otp.code}) from ${msg.sender_name}. Expires in 10 minutes; no attachments.`;
			}
			if (msg.extractions.tracking) {
				return `Local Summary: Shipment dispatched via ${msg.extractions.tracking.carrier} (${msg.extractions.tracking.code}) with NF-e #${msg.extractions.nfe?.invoice_number ?? 'attached'}.`;
			}
			return `Local Summary: ${msg.snippet}`;
		},
		categoryMetadata: categories.metadata,
		correctCategory: categories.correct
	};

	const settingsTransport: SettingsTransport = {
		get: async () => ({ ...demoSettings }),
		save: async (ai, key) => {
			demoSettings = { ...demoSettings, ai: { ...ai }, has_api_key: Boolean(key) };
		},
		tokenizer: async (tokenizer) => {
			demoSettings = { ...demoSettings, tokenizer };
		},
		setExperimentalSemanticSearch: async (enabled) => {
			demoSettings = { ...demoSettings, experimental_semantic_search: enabled };
		},
		download: async () => {
			demoSettings = { ...demoSettings, model_cached: true };
		},
		summarize: async (id, consent, expected) =>
			consent && expected.enabled ? (await readerTransport.summarize?.(id, consent, expected)) ?? '' : ''
	};

	return {
		inboxTransport,
		readerTransport,
		settingsTransport,
		getMessages: () => messages,
		ensureMessage: (msg) => {
			if (!messages.some((m) => m.id === msg.id)) {
				messages = [msg, ...messages];
			}
		},
		markBoletoPaid: (id, paid) => {
			messages = messages.map((m) =>
				m.id === id && m.extractions.boleto
					? { ...m, extractions: { ...m.extractions, boleto: { ...m.extractions.boleto, paid } } }
					: m
			);
		}
	};
}
