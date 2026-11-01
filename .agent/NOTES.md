# NOTES.md — Active Contracts and Runtime Memory

> Stores the active runtime contracts (WHAT interacts with WHAT), live communication topology, and critical gotchas.
> Historical architectural decisions live in [`docs/adr/decisions.md`](../docs/adr/decisions.md).

---

## Active Contracts

- **Pre-Approved Tasks (`APPROVED`), Parallel Waves (`[PARALLEL: Wx]`) & `task-queue` Skill:** Tasks finalized during interactive planning use `Status: APPROVED` in `.agent/TASK.md` (with independent disjoint-file tasks tagged `[PARALLEL: Wx]` + `Paths:`), allowing fresh sessions (`proceed`) or isolated worker subagents and multi-account quota failover (`task-queue` skill on `proceed queue`) to transition directly to `RUNNING` without a redundant `PLANNING` pause, halting only when ambiguities, unapproved dependencies, or blockers require human intervention.


### 1. Rust Workspace Crate Boundaries (`Cargo.toml`)

All core domain crates MUST remain pure Rust (zero dependency on `tauri`) so the engine can run headless (CLI/daemon) or embedded in `apps/desktop/src-tauri`:

| Crate | Responsibility | Public Contract / Traits | Internal Dependencies |
|---|---|---|---|
| `crates/vault` | OS Keyring (`keyring`) & encrypted fallback vault (`argon2` + `aes-gcm` / `chacha20poly1305`) | `SecretStore` trait, `VaultKey`, `MasterCipher` | None (leaf crate, `#![forbid(unsafe_code)]`) |
| `crates/storage` | Single-file SQLite WAL + Full-DB SQLCipher + Configurable `FTS5` (default `unicode61 remove_diacritics 2`) + `sqlite-vec` (`int8` quantized KNN + pre-filtering) + IMAP `UIDVALIDITY`/`MODSEQ` sync state | `MailRepository`, `SearchEngine` (`HybridQuery` -> `<15ms`), `MigrationRunner` | `crates/vault` (`#![deny(unsafe_code)]` with isolated `#[allow(unsafe_code)]` only on `sqlite-vec` FFI init) |
| `crates/mail-engine` | Agnostic IMAP transport (`ImapConnector` trait), incremental sync (`UIDVALIDITY`/`CONDSTORE`), transparent multi-auth (App Passwords, OAuth2 Public PKCE, user-supplied BYO OAuth `client_id`/`client_secret`, Proton Bridge), RFC 8058 One-Click Unsubscribe, `mail-parser` | `ImapConnector`, `IdleManager`, `MimeExtractor`, `SyncEventSink` trait | `crates/vault` (`#![forbid(unsafe_code)]`) |
| `crates/triage-ai` | 3-Tier Triage: Tier 1 (Regex 2FA/OTP, Pix EMV/keys, FEBRABAN Boletos with exact BRL value & due date, NF-e metadata, Tracking, `List-Unsubscribe` + RFC 8058 One-Click), Tier 2 (Local CPU ONNX embeddings downloaded on demand to `$XDG_DATA_HOME`), Tier 3 (Ollama/BYOK HTTP) | `DeterministicTriage`, `EmbeddingProvider` trait, `LlmSummarizer` trait, `TriageEventHook` trait | None (pure data in/out, `#![forbid(unsafe_code)]`) |
| `apps/desktop/src-tauri` | Tauri v2 IPC bridge, system tray, native window lifecycle | Typed `#[tauri::command]` handlers & `tauri::ipc::Channel<SyncEvent>` | `mail-engine`, `storage`, `vault`, `triage-ai` |

### 2. Tauri v2 IPC & Streaming Event Contracts (`apps/desktop`)

| IPC Command / Channel | Direction | Payload Schema | Performance / Security Invariant |
|---|---|---|---|
| `cmd_list_accounts` / `cmd_add_account` | UI $\rightarrow$ Rust | `AccountConfigDto` (supports App Password, OAuth2 Public PKCE, BYO-OAuth credentials, Proton Bridge; never returns raw secrets to UI) | Secrets stored immediately in `vault` (`keyring` or Argon2 fallback); UI only sees opaque `account_id` |
| `cmd_search_messages` | UI $\rightarrow$ Rust | `SearchQueryDto { query, mode: Lexical \| Semantic \| Hybrid, limit, offset }` $\rightarrow$ `Vec<MessageRowDto>` | Filtered semantic $\le 15$ms, lexical $\le 40$ms, global semantic $\le 50$ms, hybrid $\le 60$ms (ADR-008) |
| `cmd_triage_action` | UI $\rightarrow$ Rust | `TriageMutationDto { message_ids, action: MarkRead \| Archive \| Trash \| Tag(String) \| OneClickUnsubscribe }` | Optimistic local SQLite update + queued async IMAP `STORE FLAGS` sync (or RFC 8058 HTTP POST on explicit unsubscribe) |
| `cmd_get_message_body` | UI $\rightarrow$ Rust | `MessageId` $\rightarrow$ `MessageDetailDto { headers, text_plain, html_raw, triage_badges, attachments_meta }` | HTML sanitized in UI with `DOMPurify` + `<iframe sandbox="">` + normalized base dark-mode CSS |
| `cmd_fetch_attachment` | UI $\rightarrow$ Rust | `AttachmentRequestDto { account_id, folder, uid, part_id }` $\rightarrow$ `AttachmentBytesDto` | **Lazy Attachment Invariant:** Fetched on-demand via `IMAP BODY.PEEK[section]` |
| `Channel<SyncProgressEvent>` | Rust $\rightarrow$ UI | `SyncProgressEvent { account_id, state, new_messages_count, triage_alerts, embedding_errors_count }` | Non-blocking streaming updates from `tokio` IMAP `IDLE` workers |

### 3. Extensibility Trait Boundary

To keep the core 100% free and open-source (AGPLv3) with zero artificial limits while supporting optional post-triage hooks:
- `TriageEventHook` / `SyncEventSink` trait: Clean observer boundary invoked after message ingestion and triage extraction (used by the desktop UI notification channel and extensible for future local/webhook automations without premature cloud-sync abstractions).

---

## Gotchas & Pitfalls

- **Transparent Multi-Mode Authentication (OAuth2 vs App Passwords):** Providers like Gmail require a `client_secret` and paid CASA verification for public desktop OAuth2 apps, whereas Microsoft supports public PKCE (`client_id` only). `crates/mail-engine` MUST transparently support all modes for the user: App Passwords, Public OAuth2 PKCE, and user-provided BYO OAuth (`client_id` + optional `client_secret` stored in `crates/vault`), while ensuring zero credentials are ever hardcoded or committed to git.
- **Proton Mail Bridge (Localhost IMAP):** Proton Bridge runs on `127.0.0.1:1143` using STARTTLS or a self-signed local TLS certificate. `crates/mail-engine` MUST only allow custom root/self-signed certificate pinning when the IMAP host is strictly loopback (`127.0.0.1` / `::1`), never for remote hosts.
- **IMAP Provider Quirks & Zero-Copy MIME (`pimalaya` & Stalwart `mail-parser`):** Normalize provider-specific IMAP differences (Gmail `[Gmail]/All Mail` deduplication & `XOAUTH2`, Outlook `CONDSTORE` fallbacks, Proton Bridge `IDLE` behavior) behind `ImapConnector` following `pimalaya/email-lib` patterns, and parse MIME structures zero-copy using Stalwart Labs' `mail-parser` (see `.agent/knowledge/k-002-imap-provider-quirks-and-mime-parsing.md`).
- **SQLite FTS5 + SQLCipher + `sqlite-vec` Amalgamation:** Use full-database SQLCipher encryption (never encrypt text columns individually, as that breaks `FTS5`). Default `FTS5` tokenizer to `unicode61 remove_diacritics 2` (configurable by user) and register `sqlite-vec` via `sqlite3_auto_extension` inside an isolated `#[allow(unsafe_code)]` block in `crates/storage` (`#![deny(unsafe_code)]`).
- **ONNX Model Weights on Demand:** Do not bundle multi-megabyte ONNX model files inside the compiled binary; download/cache quantized models lazily in the user data directory (`$XDG_DATA_HOME/triage-mail/models`).
- **Git History Scrubbing (Private $\rightarrow$ Public Transition):** Every test fixture for MIME parsing, 2FA/Boleto/NF-e extraction, and IMAP responses MUST use synthetic RFC 2606 domains (`user@example.com`, `billing@example.org`) and synthetic tokens.

---

## Deliberate Technical Debt

| Debt | Rationale | Revisit When |
|---|---|---|
| Outbound SMTP / Email Composer omitted | MVP v0.1 is strictly "Triage-First" (Inbound reading, search, and triage) | Post-v0.1 (`v0.2+`) after inbound sync and hybrid search reach stability |
| Local embedding runtime uses deterministic projections, without ONNX inference | Existing provider seeds projections from cached artifacts; [04.7] discloses this limitation instead of implying learned semantics | Tier 2 ONNX runtime integration |

## Desktop IPC Implementation Details ([04.1])

- The six established commands retain snake_case DTO fields and externally tagged Rust enums.
  Tauri invoke argument names use its default camelCase (`messageId` for message detail).
  `cmd_add_account` takes separate `config: AccountConfigDto` and input-only `credentials`
  (`AccountAuthMaterial` wire representation), and returns only an opaque account ID string.
  An empty input ID requests a generated opaque ID; nonempty IDs cannot contain email addresses.
  `cmd_list_accounts` returns safe `AccountConfigDto` metadata without consulting credential values.
- `MessageRowDto` aliases `storage::MessageRowSummary`. `MessageDetailDto` preserves the scaffold's
  `id`, `subject`, `sender`, and `triage_result`, and adds contracted `headers` (raw header string)
  and `triage_badges`. `AttachmentBytesDto` is `{ bytes: Vec<u8> }`.
- `cmd_subscribe_sync(onProgress: Channel<SyncProgressEvent>)` registers one subscription per local
  webview. Replacement/disconnection does not stop mail ingestion. Lag emits `ResyncRequired` so
  consumers refetch authoritative data. Other states are `Syncing`, `Idle`, `Retrying`,
  `MutationPending`, and `MutationFailed`. Alerts are `Vec<TriageBadge>` with bounded previews.
- SQLCipher migration v4 adds `account_connections`, `pending_mutations`, and
  `message_folder_overrides` without altering existing tables. Endpoint JSON is serialized only
  from the secret-free configuration DTO. Remote mutation payloads contain message identity and
  action only; local updates and outbox writes commit together. Pending read actions survive
  background flag reconciliation. Remote commands check UIDVALIDITY before using a queued UID.
- Runtime initialization provisions the SQLCipher key through `SecretStore`; a missing key for
  an existing database fails closed. `cmd_unlock_vault(passphrase)` explicitly enables the
  Argon2id/AES-GCM fallback when the OS keyring is unavailable. Passphrases and incoming auth
  buffers are zeroized and never appear in responses, events, errors, or configuration JSON.
- Headless `mail_engine::mutations` exports UIDVALIDITY-checked STORE/MOVE and lazy attachment
  helpers with pure Rust request/outcome types. Desktop DTOs adapt to these helpers; existing
  `ImapConnector`, `SyncEventSink`, `TriageEventHook`, and repository trait signatures are unchanged.
- Workers use incremental sync, IDLE with heartbeat renewal, and adaptive polling fallback.
  Network operations never hold the local SQLite mutation lock. Native transport is injectable
  for offline service tests; the default `native-runtime` feature runs the actual Tauri app.
- Archive/trash discover server special-use folders and require IMAP MOVE; unsupported servers
  retain their queued action and emit `MutationFailed`. There is no unsafe blanket-expunge fallback.
  OAuth account registration accepts the existing vault auth material, including acquired tokens;
  a provider-specific authorization UI remains separate. Embeddings use the existing provider
  implementation and cached model configuration; IPC never downloads models or calls an LLM.
- RFC 8058 requests validate the actual DNS address set supplied to the HTTP connector,
  reject non-public/mixed answers, disable inherited proxies and redirects, and retain HTTPS
  verification. Desktop production never enables the loopback test exception (see `k-009`).

## Account Onboarding IPC ([04.6])

- Existing account/unlock commands and DTOs retain their wire representation.
- Add `cmd_authorize_account(config, authorization, onReady: Channel<()>)` -> opaque account ID. Input-only
  authorization is `{ authorization_endpoint, token_endpoint, scopes, client_id, client_secret? }`.
  The command is explicitly initiated by the user, launches the system browser, binds an
  ephemeral `127.0.0.1` PKCE callback, exchanges the code natively, and stores tokens in vault.
  Only user-configured HTTPS authorization/token endpoints are contacted; no discovery,
  redirects, inherited proxies, analytics, or automatic retries. Client credentials are
  staged in vault during callback wait and removed on completion/cancellation/failure.
- Add `cmd_cancel_authorization()` scoped to the invoking webview. One authorization runs
  per webview, bounded to five minutes. Cancellation/window destruction drops the listener
  and any pending exchange; once account persistence starts it completes atomically.
  The UI refreshes accounts after cancellation to reconcile a concurrent successful commit.
  Native `onReady` acknowledgement enables the cancel control only after session registration;
  a disposed UI cancels on late acknowledgement, avoiding cancellation-before-registration races.
- OAuth error/response payloads are fixed `IpcError` variants; no URL, code or token returns
  to the UI. Endpoint settings are submitted explicitly with each authorization attempt.
  Token refresh remains outside this onboarding command and is not performed automatically.
- Frontend credentials live only in transient unbound form inputs and invoke arguments;
  they are cleared on submission and teardown, never in Runes/controller state or storage.

## Unified Inbox UI ([04.2])

- `src/lib/inbox/InboxController` owns paged rows, selection and sync refresh state behind an
  injectable `InboxTransport`. It uses the existing snake_case DTOs without backend changes.
- IPC pages contain 200 summaries; the visible list renders fixed 88px rows plus five rows
  of overscan per edge. Counts describe loaded summaries (`+` while more pages may exist),
  because the established IPC response does not expose a total count.
- Search changes reset selection/scroll and invalidate older requests. Sync refreshes replace
  authoritative loaded pages, retain surviving selection and anchor the first visible message.
  Subscription teardown silences late callbacks; ingestion remains owned by the native runtime.
- Search is debounced by 200ms. Words/Meaning/Words + meaning map to Lexical/Semantic/Hybrid.
  Missing embeddings produce a fixed local-model message; the UI never downloads a model.
- `InboxView` provides the adaptive sidebar/list/reader shell with system light/dark appearance.
  The secure reader is [04.3]; the keyboard triage palette and extraction badges are [04.4].

## Inbox Motion ([04.2.1])

- Motion uses shared CSS timing/easing tokens (120ms feedback, 220ms view changes) with no
  animation dependencies. Sidebar collapse makes its retained contents inert immediately;
  the toolbar exposes `aria-expanded`. Compact back navigation restores list keyboard focus.
- Overview replacement uses one keyed content subtree with entrance-only motion; outgoing
  message content never remains mounted during transitions. Virtual row insertion/removal and
  scroll offsets remain immediate; only row background/color feedback transitions.
- All motion is CSS-owned. `prefers-reduced-motion: reduce` disables spatial, repetitive and
  feedback animation/transition rules, including when the preference changes mid-animation.

## Secure Message Reader ([04.3])

- `reader/transport.ts` adapts existing body/attachment commands; invocation uses `messageId`
  for details and `{ request }` with snake_case attachment identity. No backend changes.
- The reader subtree is keyed by message/account/folder/UID identity. Effect cleanup invalidates
  late body responses; attachment teardown suppresses late saves. Errors use fixed UI strings.
- DOMPurify uses an HTML tag/attribute allowlist. Sender styles, navigation URLs, SVG, media,
  forms and alternate resource attributes are removed. Only raster data images survive by
  default; CID attachments remain lazy. A leading CSP blocks scripts, connections and remote
  resources, with normalized dark CSS inside an empty-sandbox, no-referrer srcdoc iframe.
- Attachment bytes are requested only on a user click, saved via an outer-webview Blob download
  with an octet-stream MIME type and normalized basename, and never rendered. Duplicate clicks
  are guarded. Object URLs expire after 60 seconds and are also released on teardown.
- Synthetic browser QA verified the save flow and compact navigation; Cargo workspace check/test/
  fmt/clippy and Prettier (`prettier-plugin-svelte`) passed.

## Keyboard Triage and Extraction UI ([04.4])

- `InboxTransport.triage` sends `{ mutation: { message_ids, action } }` to the existing
  `cmd_triage_action`; unit actions remain strings and tags use `{ Tag: string }`.
- The palette uses bits-ui 2.19.5 (user-approved exception to the skill's v1 reference).
  Ctrl/Cmd+K opens it; R marks read, Shift+U unread, E archives, Delete trashes and T adds
  a local tag. Single-key actions ignore editable targets, repeats and IME composition.
- One-click unsubscribe is enabled only by supported extraction metadata and requires an
  explicit palette selection. No preview, shortcut or retry automatically sends its POST.
- One UI mutation runs at a time. It invalidates earlier reads and defers sync refresh until
  settled. Rejected IPC writes roll back local feedback; background remote failures retain
  durable local mutations and surface the existing sync status rather than undoing the outbox.
- Folder feedback never replaces canonical row folder/UID identity. Summary fields `tags`
  (sorted unique strings, default empty) and `folder_override` (nullable string, default null)
  expose persisted local metadata additively. Storage hydrates ranked pages from existing
  tables; no schema or trait signature changes. Desktop search preserves canonical identity.
- Accepted search/refresh pages replace feedback with authoritative metadata; optimistic writes
  remain immediate and rejected mutations roll back. MOVE completion clears the override and
  updates canonical folder/UID together. Stale pages cannot restore older feedback.
- Visible rows request extraction metadata through the existing local detail command. The
  shared loader permits four concurrent requests, cancels obsolete queued callbacks, caches
  at most 400 badge entries (never bodies), and refetches after authoritative sync revisions.
  Reader badges show full values; compact list badges expose values through accessible labels.

## Settings and AI Consent IPC ([04.7])

- Add `cmd_get_settings()` -> `SettingsDto { ai: { enabled, provider: Ollama | Byok,
  endpoint, model }, tokenizer, has_api_key, model_cached, experimental_semantic_search }` and
  `cmd_save_settings(settings: AiSettings, apiKey)` -> `()`. Native vault atomically stores
  AI metadata with its endpoint-bound API key; responses contain only secret-free metadata.
  Empty key preserves an existing key only for the same provider/endpoint; saving Ollama
  clears it. Saving performs no network request. A disabled setting revokes future dispatch.
- Add `cmd_set_experimental_semantic_search(enabled: bool)` -> `()` to enable/disable
  Tier 2 feature-hash semantic projection search.
- When `experimental_semantic_search` is false (default), `cmd_search_messages` rejects
  `Semantic` and `Hybrid` queries with `IpcError::InvalidRequest`.
- In UI, match options are hidden by default and, when enabled, honestly labeled as
  "Words", "Semantic projection (Experimental)", and "Words + semantic projection (Experimental)".
- In sync worker, `SyncProgressEvent.embedding_errors_count: usize` exposes failed embedding telemetry
  instead of silently discarding errors.
- Add `cmd_set_tokenizer(tokenizer: FtsTokenizerMode)` -> `()` as a separate explicit rebuild
  action. AI configuration and search changes have independent persistence/error boundaries.
- Add `cmd_download_model(request: { model_url, tokenizer_url, consent })` -> `()`.
  Only explicit consent permits HTTPS artifact requests; credentials, fragments, redirects,
  inherited proxies and retries are forbidden. Downloads are bounded and cached under the
  existing XDG model directory. This is artifact caching, not an ONNX inference implementation.
- Add `cmd_summarize_message(messageId, consent, expected: AiSettings)` -> summary string. Native code reads the
  selected stored subject/plain body, checks enabled settings, per-call consent and equality
  with the displayed settings snapshot, retrieves
  the key from vault and calls the existing LLM client. No background summarization or probing.
  Errors remain fixed `IpcError` variants. Summaries are untrusted text, never HTML.
- Add storage tokenizer inspection/reconfiguration methods without schema-version changes.
  Recreate the existing external-content FTS table and rebuild from messages in one transaction;
  retain triggers and infer persisted tokenizer from sqlite_master on reopen.
- Remote-image consent belongs to the current message identity only, defaults blocked and
  is never persisted as a global allow setting. Sanitization and the empty sandbox remain.

## Persisted Inbox and Search Ranking ([90.2])

- Blank/whitespace lexical requests list persisted summaries by received time descending,
  then message ID ascending. Account restrictions, pagination and existing tag/folder metadata
  apply; canonical IMAP folder/UID remain unchanged. Hybrid without text or a vector also lists.
- Lexical candidates rank by BM25 then message rowid. Semantic candidates rank by exact cosine
  distance then message ID, with account eligibility applied before pagination. Unfiltered vec0
  top-k expands across ties; exact ranking handles deep pages, oversized ties and orphan vectors.
- Hybrid retains the existing per-branch `max(limit+offset, 50)` candidate window and RRF
  constant 60; equal fused scores use message ID. This stabilizes a given request, not ranking
  across changing candidate-window sizes or concurrent ingestion. Final summaries/metadata are
  hydrated once after fusion. IPC DTOs, schema versions and trait signatures are unchanged.
- Storage rejects pagination beyond SQLite integer bounds before query execution. Details and
  query-plan constraints live in k-015; target latency remains subject to measured evidence.


## Pre-Release Schema Consolidation & Persisted Triage ([01.3] / ADR-007)

- **Schema Governance Policy:** P1 schema immutability applies strictly to released schemas. Prior to
  `v0.1.0` release, intermediate evolutionary migrations v1–v5 are collapsed into a canonical v1 schema.
  From tag `v0.1.0` onward, all migrations MUST be append-only (`v2`, `v3`, etc.).
- **Canonical Schema v1:**
  - `accounts`, `folder_sync_state`, `attachments`, `tags`, `account_connections`, `pending_mutations`,
    `message_folder_overrides`.
  - `messages`: includes `category_badge TEXT` and `triage_badges TEXT NOT NULL DEFAULT '[]'`.
  - `messages_fts`: FTS5 external content table + triggers (`messages_ai`, `messages_ad`, `messages_au`).
  - `vec_messages`: single vec0 table with `message_id TEXT PRIMARY KEY`, `account_id TEXT PARTITION KEY`,
    and `embedding_int8 int8[384] distance_metric=cosine`.
- **Elimination of Derived Index Duplication:** Dropped `vec_messages_by_account`, reconciliation triggers,
  and migration backfill. Embedding replacement writes to `vec_messages` directly.
- **Persisted Triage & Zero-Body List Badges:** `worker.rs` populates `category_badge` and `triage_badges`
  at ingestion into `messages`. `MessageRowSummary` / `MessageRowDto` exposes `triage_badges: Vec<TriageBadge>`
  additively. UI list renders badges directly without calling `cmd_get_message_body` or transferring full
  bodies over IPC. Message detail uses persisted triage metadata, avoiding dynamic regex re-computation.

## Exact Search Formulation & Latency ([90.4])

- Lexical ranking uses a non-materialized CTE (`WITH ranked AS (...)`) to avoid temporary B-tree
  materialization pages while preserving exact BM25 score ordering and deterministic rowid ties.
- Lexical, blank-inbox, and vector queries use dedicated query branches with exact parameter sets
  to eliminate dead nullity checks (`?2 IS NULL` / `(?2 IS NULL OR account_id=?2)`).
- Hybrid candidate fusion pre-allocates scores maps and uses cached prepared statements for
  final-page summary and metadata hydration. All schema versions, traits, and DTOs remain unchanged.

## Lazy Body Fetch & Batched Initial Sync ([02.4])

- **Two-Phase Fetch Protocol (`protocol.rs`):**
  - Wire command strictly forbids `BODY.PEEK[TEXT]`, `BODY[]`, or `RFC822` during bulk sync.
  - Phase 1: `UID FETCH <uids> (UID MODSEQ FLAGS BODYSTRUCTURE BODY.PEEK[HEADER])` downloads structure and headers only.
  - RFC 3501 `BODYSTRUCTURE` parser (`bodystructure.rs`) extracts attachment metadata without downloading payload bytes and identifies candidate leaf text sections (`text/plain`, `text/html` <= 2MB).
  - Phase 2: `UID FETCH <uids> (BODY.PEEK[1])` or `(BODY.PEEK[<sec>.MIME] BODY.PEEK[<sec>])` fetches only the targeted leaf text sections and synthesizes entities zero-copy.
  - Attachment bodies are fetched strictly on-demand via `IMAP BODY.PEEK[<part_id>]`.
- **Batched Initial Sync:**
  - `DEFAULT_SYNC_UID_BATCH_SIZE: u32 = 300` limits window chunks during initial sync.
  - Desktop background worker commits each 300-message batch and updates the `FolderSyncState.uidnext` watermark in SQLCipher before requesting the subsequent batch. If interrupted, already ingested batches remain durable.

## OAuth Token Refresh & Orphan Staging Cleanup ([02.5])

- **Vault Extension:**
  - `AccountAuthMaterial::OAuth2PublicPkce` and `AccountAuthMaterial::OAuth2ByoCredentials` include `#[serde(default, skip_serializing_if = "Option::is_none")] token_endpoint: Option<String>`.
  - Helper methods: `token_endpoint(&self)`, `refresh_token(&self)`, `client_id(&self)`, `client_secret(&self)`, and atomic token updater `update_tokens(&mut self, access_token: String, refresh_token: Option<String>) -> bool`.
- **SASL XOAUTH2 Protocol Challenge Abort:**
  - Per RFC 6749 and Google/Microsoft IMAP XOAUTH2 specifications, if authentication fails, IMAP servers send `+ <base64 error>` and await client continuation.
  - The client must reply with `\r\n` to complete the challenge abort, after which the server emits the tagged `NO`.
  - `mail-engine::protocol::execute_command` detects `+ ` continuation on `AUTHENTICATE` commands and transmits `\r\n` before reading the tagged response, returning `EngineError::AuthenticationFailed`.
- **Automatic Token Refresh on Auth Failure:**
  - In `DefaultImapConnector::connect_and_authenticate_as`: if `try_authenticate` returns `EngineError::AuthenticationFailed`, and the stored auth material has both a `token_endpoint` and `refresh_token`, the connector invokes `TokenRefresher` to exchange the refresh token for a new access token (and optional rotated refresh token), persists the updated auth material in `SecretStore` atomically, establishes a fresh TLS session, and retries authentication once.
- **P0 Network Policy on Token Refresh:**
  - Enforced via `HttpTokenRefreshTransport` and `validate_token_endpoint`: scheme must strictly be HTTPS, URL length bounded to 2048 chars, no embedded credentials, queries, or fragments, `no_proxy()`, `Policy::none()` (no redirects), timeout 30s, and response body bounded to 64KB.
- **Deterministic Staging Key & Startup Hygiene (k-010):**
  - OAuth onboarding uses a deterministic vault key: `staging_oauth_vault_key() -> VaultKey::for_account("staging", "oauth-pending")`.
  - On application startup, `DesktopService::new` invokes `clean_orphaned_staging(vault)` to remove any pending staging secret left over from process termination or crashes, preventing secret accretion in OS keyring backends.

## Concurrency Architecture & Empirical Latency Targets ([90.5] / ADR-008)

- **SQLite WAL Concurrency & Snapshot Isolation:**
  - Multiple read connections in SQLite WAL mode are completely thread-safe and ACID-compliant without reader-writer or reader-reader locking contention.
  - `SqliteMailStore::open` initializes an optional secondary read connection for disk-backed databases (`secondary_conn: Option<Mutex<Connection>>`).
  - `execute_hybrid_search_parallel` executes lexical search on connection 1 and semantic search on connection 2 concurrently via `std::thread::scope`, returning fused RRF results with 100% exact equivalence to sequential execution.
  - In-memory databases or contended connections gracefully fall back to sequential execution.
- **Empirical Page Cache Overhead:**
  - Secondary read connection adds ~25.5 MiB of resident memory (VmRSS) under 100k vector scans at 64 MiB soft cache target (`cache_size = -65536`).
  - Total process RSS remains ~102.7 MiB (well below 110 MiB budget).
- **Redefined Latency Targets for v0.1.0 (p95 warm @ 100k messages on reference hardware):**
  - Filtered semantic: $\le 15$ ms (observed ~12.2 ms p50 / 12.9 ms p95).
  - Lexical search: $\le 40$ ms (observed ~31.9 ms p50 / 37.3 ms p95).
  - Global semantic search: $\le 50$ ms (observed ~42.7 ms p50 / 47.8 ms p95).
  - Hybrid search: $\le 60$ ms concurrent (observed ~52.4 ms p50) / $\le 95$ ms sequential.
  - UI First Page (including IPC + WebKit virtual DOM): $\le 100$ ms.
- **v0.1 vs v0.2 Boundaries:**
  - Strictly NO approximate nearest neighbor (ANN) graph indexing (HNSW, IVF) in v0.1.
  - Binary quantization (1-bit scalar quantization with Hamming distance / bitwise POPCNT candidate pre-filtering) + int8 cosine reranking of top 200 candidates scheduled for v0.2 evaluation to reach sub-15ms global queries.

## Account Onboarding Help Guide & Apple-Style Language Selector ([04.8])

- **Reactive i18n Store (`src/lib/i18n/`):** Pure Svelte 5 rune-based localization module supporting `en` (English) and `pt-BR` (Português do Brasil). Preferences are persisted to `localStorage` with fallback to browser language. Typed translation dictionary avoids external dependencies.
- **Minimalist Apple-Style Language Selector (`LanguageSelector.svelte`):** Segmented pill control (`EN` | `PT`) with translucent backdrop blur, subtle Apple dark/light surface tokens, instant active feedback, accessible `role="radiogroup"` / `aria-checked` bindings, and zero layout shift. Exposed in Onboarding and Settings.
- **In-App Account Help Guide & Provider Presets (`AccountForm.svelte`, `AccountHelpGuide.svelte`, `presets.ts`):**
  - Interactive disclosure guide explaining all 4 login methods: App Passwords (2FA prerequisite, IMAP enabling, common provider links/hosts), OAuth2 Public PKCE (Microsoft Entra / Azure app registration, loopback `127.0.0.1` redirect), OAuth2 BYO Credentials (Google Cloud Console project, Gmail API, `client_id` + `client_secret`), and Proton Mail Bridge (local `127.0.0.1:1143` IMAP, paid Proton account, bridge password alert).
  - Each authentication tab includes a structured field checklist ("What to fill in the form") and numbered provider-specific step-by-step playbooks explaining how to configure and obtain credentials.
  - Quick-fill provider presets (`Gmail`, `Gmail OAuth`, `Outlook`, `iCloud`, `Yahoo`, `Fastmail`, `Proton`) isolated in `presets.ts` to auto-populate host, port, OAuth endpoints/scopes, and recommended auth mode in 1 click.
- **Documentation Perimeter:** Comprehensive Account Setup & Authentication guides added in `README.md` and `README.pt-br.md` maintaining 1:1 bilingual parity and zero heading emojis per `readme-standard`.

