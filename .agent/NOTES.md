# NOTES.md — Active Contracts and Runtime Memory

> Stores the active runtime contracts (WHAT interacts with WHAT), live communication topology, and critical gotchas.
> Historical architectural decisions live in [`docs/adr/decisions.md`](../docs/adr/decisions.md).

---

## Active Contracts

- **IMAP parser regression contract ([02.6]):** FETCH literal payloads remain opaque
  bytes across fragmented reads; completion requires balanced control-chunk parentheses.
  Invalid/overflowing literal markers fail. BODYSTRUCTURE consumes one complete root,
  uses checked complete CRLF literal ranges and preserves quoted UTF-8. Public trait,
  IPC and storage contracts remain unchanged. Bounded offline property coverage and
  its limits are recorded in k-034.

- **Pre-Approved Tasks (`APPROVED`), Parallel Waves (`[PARALLEL: Wx]`) & `task-queue` Skill:** Tasks finalized during interactive planning use `Status: APPROVED` in `.agent/TASK.md` (with independent disjoint-file tasks tagged `[PARALLEL: Wx]` + `Paths:`), allowing fresh sessions (`proceed`) or isolated worker subagents and multi-account quota failover (`task-queue` skill on `proceed queue`) to transition directly to `RUNNING` without a redundant `PLANNING` pause, halting only when ambiguities, unapproved dependencies, or blockers require human intervention.


### 1. Rust Workspace Crate Boundaries (`Cargo.toml`)

All core domain crates MUST remain pure Rust (zero dependency on `tauri`) so the engine can run headless (CLI/daemon) or embedded in `apps/desktop/src-tauri`:

| Crate | Responsibility | Public Contract / Traits | Internal Dependencies |
|---|---|---|---|
| `crates/vault` | OS Keyring (`keyring`) & encrypted fallback vault (`argon2` + `aes-gcm` / `chacha20poly1305`) | `SecretStore` trait, `VaultKey`, `MasterCipher` | None (leaf crate, `#![forbid(unsafe_code)]`) |
| `crates/storage` | Single-file SQLite WAL + Full-DB SQLCipher + Configurable `FTS5` (default `unicode61 remove_diacritics 2`) + `sqlite-vec` (`int8` quantized KNN + pre-filtering) + IMAP `UIDVALIDITY`/`MODSEQ` sync state | `MailRepository`, `SearchEngine` (`HybridQuery` -> `<15ms`), `MigrationRunner` | `crates/vault` (`#![deny(unsafe_code)]` with isolated `#[allow(unsafe_code)]` only on `sqlite-vec` FFI init) |
| `crates/mail-engine` | Agnostic IMAP transport (`ImapConnector` trait), incremental sync (`UIDVALIDITY`/`CONDSTORE`), transparent multi-auth (App Passwords, OAuth2 Public PKCE, user-supplied BYO OAuth `client_id`/`client_secret`, Proton Bridge), RFC 8058 One-Click Unsubscribe, `mail-parser` | `ImapConnector`, `IdleManager`, `MimeExtractor`, `SyncEventSink` trait | `crates/vault` (`#![forbid(unsafe_code)]`) |
| `crates/triage-ai` | 3-Tier Triage: Tier 1 (Regex 2FA/OTP, Pix EMV/keys, FEBRABAN Boletos with exact BRL value & due date, NF-e metadata, Tracking, `List-Unsubscribe` + RFC 8058 One-Click), Tier 2 (Local CPU ONNX embeddings downloaded on demand to `$XDG_DATA_HOME`), Tier 3 (Ollama/BYOK HTTP) | `DeterministicTriage`, `EmbeddingProvider` trait, `LlmSummarizer` trait, `TriageEventHook` trait | None (pure data in/out, `#![forbid(unsafe_code)]`) |
| `crates/sync-runtime` | Shared single-account sync-pass coordinator; host-owned storage/transport/hooks | Additive `SyncHost`, `sync_account_once` | `mail-engine`, `storage`; no desktop/Tauri dependency |
| `apps/desktop/src-tauri` | Tauri v2 IPC bridge, system tray, native window lifecycle | Typed `#[tauri::command]` handlers & `tauri::ipc::Channel<SyncEvent>` | `mail-engine`, `storage`, `vault`, `triage-ai` |
| `apps/cli` | Offline cached-mail inspection; no workers or remote actions | `accounts`, `search`, `show`; bounded JSON and fixed exit codes | `storage`, `vault`; no desktop/Tauri dependency |

### 2. Tauri v2 IPC & Streaming Event Contracts (`apps/desktop`)

| IPC Command / Channel | Direction | Payload Schema | Performance / Security Invariant |
|---|---|---|---|
| `cmd_list_accounts` / `cmd_add_account` | UI $\rightarrow$ Rust | `AccountConfigDto` (supports App Password, OAuth2 Public PKCE, BYO-OAuth credentials, Proton Bridge; never returns raw secrets to UI) | Secrets stored immediately in `vault` (`keyring` or Argon2 fallback); UI only sees opaque `account_id` |
| `cmd_discover_email_provider` | UI $\rightarrow$ Rust | `email: String, online?: bool` $\rightarrow$ `DiscoveredProviderDto { domain, provider_name, imap_host?, imap_port, use_ssl, recommended_auth, oauth_endpoints?, help_url?, source }` | Offline by default; explicit online action sends domain only to ISPDB with a 3s deadline and 64 KiB cap; no automatic probe |
| `cmd_search_messages` | UI $\rightarrow$ Rust | `SearchQueryDto { query, mode: Lexical \| Semantic \| Hybrid, limit, offset, scope? }` $\rightarrow$ `Vec<MessageRowDto>` | Filtered semantic $\le 15$ms, lexical $\le 40$ms, global semantic $\le 50$ms, hybrid $\le 60$ms (ADR-008); mailbox budgets remain unmet in [04.1.7] |
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
| Native ONNX runtime requires explicit separate installation | Keep native libraries and weights outside the desktop binary; verify pinned platform hashes before loading | Native packaging/integration validation on additional platforms |

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
  Tier 2 learned CPU ONNX semantic search (legacy wire name retained).
- When `experimental_semantic_search` is false (default), `cmd_search_messages` rejects
  `Semantic` and `Hybrid` queries with `IpcError::InvalidRequest`.
- In UI, match options are hidden by default and, when enabled, honestly labeled as
  "Words", "Meaning (local)", and "Words + meaning (local)", with EN/PT-BR labels.
- In sync worker, `SyncProgressEvent.embedding_errors_count: usize` exposes failed embedding telemetry
  instead of silently discarding errors.
- Add `cmd_set_tokenizer(tokenizer: FtsTokenizerMode)` -> `()` as a separate explicit rebuild
  action. AI configuration and search changes have independent persistence/error boundaries.
- Add `cmd_download_model(request: { model_url, tokenizer_url, consent })` -> `()`.
  Only explicit consent permits HTTPS artifact requests; credentials, fragments, redirects,
  inherited proxies and retries are forbidden. Downloads are bounded and cached under the
  existing XDG model directory. Only the pinned model/tokenizer revision is accepted; genuine inference additionally requires the separately installed verified native runtime ([03.10]).
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

## Application Title Bar ([04.9])

- **Injectable Window Adapter (`src/lib/window/types.ts`, `transport.ts`):** `WindowTransport` wraps `@tauri-apps/api/window` (`getCurrentWindow`) behind `isNativeTauriWindow()` (`'__TAURI_INTERNALS__' in window`). Browser preview renders zero desktop controls and reserves no header height gap. `subscribeState` reconciles `onResized`/`onFocusChanged` and disposes late-resolving listeners if unsubscribed before registration resolves.
- **Themed & Localized Title Bar (`src/lib/window/TitleBar.svelte`):** 40px persistent header (`--toolbar`, `--text`, `--secondary`, `--separator`), static `'triage-mail'` brand (never exposes subjects, addresses, or account metadata), primary-pointer drag region (`data-tauri-drag-region`) excluding interactive buttons, double-click maximize toggle, 40x40px Minimize / Maximize-or-Restore (`aria-pressed`) / Close (destructive hover/active) controls, reactive EN/PT-BR labels, and polite status error announcement on rejected actions without optimistic state corruption.
- **Global Desktop Shell & Capabilities (`+layout.svelte`, `app.css`, `tauri.conf.json`, `capabilities/main.json`):** `.desktop-shell` (`100dvh` flex column) mounts `TitleBar` above `.desktop-shell-content` (`flex-1 min-h-0`); `InboxView`, `SettingsView`, and `OnboardingView` use `h-full min-h-0 flex-1` to fill remaining viewport height without overflow. Main window enables `"decorations": false` (`1280x820`, min `900x600`, `resizable: true`) with minimal explicit ACL permissions (`allow-start-dragging`, `allow-minimize`, `allow-toggle-maximize`, `allow-close`, `allow-is-maximized`, `allow-is-focused`).

## Unified Settings Modal & Modular Navigation ([04.1])

- **Persistent Inbox Session & Sibling Overlay (`DesktopApp.svelte`, `InboxSession.svelte`, `SettingsDialog.svelte`):** Ready inbox stays permanently mounted behind a dim backdrop overlay (`bg-black/60 backdrop-blur-xs`); opening/closing settings never unmounts `InboxController`, drops IMAP IDLE connections, duplicates sync progress channels, or clears mailbox/search state.
- **Footer & Toolbar Gear Entry Points (`Sidebar.svelte`, `SearchToolbar.svelte`, `Icon.svelte`):** Sidebar footer renders an accessible Settings button (`role="button" aria-haspopup="dialog"`); SearchToolbar renders a responsive gear fallback when the sidebar is collapsed or on compact screens (`max-md:hidden`), ensuring settings remains reachable in all viewports without duplicate controls.
- **Typed Modular Architecture (`SettingsDialog.svelte`, `lib/settings/modules/`):** Settings tabs (`'general' | 'accounts' | 'search' | 'ai' | 'privacy'`) compose isolated sub-components (`GeneralModule`, `AccountsModule`, `SearchModule`, `AiModule`, `PrivacyModule`) sharing `SettingsTransport` and `AccountController` without whole-view remounts.
- **Unsaved Form & Pending Operation Guard (`guards.ts`):** `SettingsFormGuard` tracks transient `dirty` and `busy` states without caching credentials in drafts or storage; intercepts close button, Escape key, backdrop clicks, and tab switching; surfaces inline Stay / Discard alerts; blocks dismissal while async downloads/rebuilds/saves are in-flight.
- **Live Search Preference Re-synchronization:** Changes to experimental semantic search or tokenizer rebuilds in `SearchModule` immediately update `inbox.setExperimentalSemanticSearch(enabled)` upon dialog close/reload without requiring app restart.
- **Bilingual EN/PT-BR Localization (`translations.ts`, `types.ts`):** All modal headings, tab titles, close buttons, and dirty-form prompts maintain strict 1:1 bilingual parity with zero missing keys.

## Account Mailboxes Navigation & Migration v2 ([04.1])

- **Mailbox Scope Contract (`SearchQueryDto`):**
  - Add optional `scope: Option<MailboxScopeDto>` (default `None` for backwards-compatible global All Mail queries across all accounts).
  - Schema:
    - `MailboxScopeDto::GlobalAllMail` (or `None`).
    - `MailboxScopeDto::AccountView { account_id: String, view: MailboxViewDto }`.
    - `MailboxViewDto`: `Inbox`, `Starred`, `Snoozed`, `Folder(String)`.
  - Effective folder visibility in SQLite: evaluates `COALESCE(override.folder, membership.folder)` where `override` is from `message_folder_overrides` and `membership` is from `message_folder_memberships`.
  - Active Snooze Rule: `Inbox` filters out messages where `snooze_until_epoch_ms > now_utc_ms()`. When expired, messages automatically reappear in `Inbox` without moving or modifying flags.
  - Archive or Trash mutations immediately set `snooze_until_epoch_ms = NULL` in `messages`.
- **Append-Only Migration v2 (`crates/storage`):**
  - Table `folders`: `(account_id, folder_name)` PK, `display_name TEXT NOT NULL`, `delimiter TEXT`, `role TEXT NOT NULL DEFAULT 'other'`, `is_selectable INTEGER NOT NULL DEFAULT 1`, `total_messages INTEGER`, `unread_messages INTEGER`, `updated_at_epoch_ms INTEGER NOT NULL`.
  - Table `message_folder_memberships`: `(account_id, folder, uid)` PK, `message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE`, `created_at_epoch_ms INTEGER NOT NULL`.
  - Column additions on `messages`: `is_starred INTEGER NOT NULL DEFAULT 0`, `snooze_until_epoch_ms INTEGER`, `provider_message_id TEXT`. Indices on `(account_id, is_starred, received_at_epoch_ms DESC)`, `(snooze_until_epoch_ms)`, and `(account_id, provider_message_id)`.
  - Table `account_avatars`: `account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE`, `avatar_bytes BLOB NOT NULL`, `mime_type TEXT NOT NULL`, `width INTEGER NOT NULL`, `height INTEGER NOT NULL`, `updated_at_epoch_ms INTEGER NOT NULL`.
- **Same-Account Deduplication Engine:**
  - Gmail: Provider message ID (`X-GM-MSGID`) checked before storing bodies/embeddings. Existing message record only acquires an additional row in `message_folder_memberships` for the newly synced label/folder.
  - Other IMAP: Fallback deduplication by `(account_id, message_id_header, received_at_epoch_ms)`.
  - Segregation Invariant: Deduplication NEVER occurs across distinct accounts.
  - Per-folder UID mapping is preserved in `message_folder_memberships` so that IMAP operations (`STORE`, `MOVE`, `BODY.PEEK[part]`) execute against the correct `folder` and `uid`.
- **Zero-Network Local Avatars:**
  - File picker reads local raster images only (JPEG/PNG/WebP, max 2MB, max 1024x1024).
  - Native worker decodes, strips EXIF/metadata, resizes to 128x128 thumbnail, and stores encrypted in `account_avatars`.
  - Accounts without custom images use deterministic initials and accent colors. Third-party fetches (Gravatar, OAuth avatars) are strictly forbidden under Rule P0.

## Starred & Local Snoozed Contracts ([04.1.3])

- **Starred Flag Synchronization:**
  - `TriageActionDto::Star` and `Unstar` commit optimistic local updates (`is_starred = 1/0`) and enqueue durable `RemoteMutation` records.
  - Desktop worker executes IMAP `UID STORE <uid> +/-FLAGS.SILENT (\Flagged)` via `ImapMutation::Star(bool)`.
  - Inbound background flag delta updates via `commit_sync_batch` update `is_starred` using `MessageFlagsDelta` while guarding pending local mutations (`p.payload_json LIKE '%Star%' OR p.payload_json LIKE '%Unstar%'`).
- **Local Snooze Lifecycle & Invariants:**
  - `TriageActionDto::Snooze { until_epoch_ms }` natively validates deadlines: strictly rejects past timestamps (`until_epoch_ms <= now`) or deadlines beyond 1 year in the future (`until_epoch_ms > now + 365 days`).
  - Snooze is 100% local metadata (`messages.snooze_until_epoch_ms`): it never generates remote outbox mutations or network requests.
  - Archive or Trash mutations immediately clear `snooze_until_epoch_ms = NULL`.
  - Explicit `TriageActionDto::Unsnooze` clears `snooze_until_epoch_ms = NULL`.
  - `reconcile_expired_snoozes(now_ms)` runs on startup, during periodic outbox loops, and can be invoked manually; it resets expired deadlines to `NULL` and emits `SyncProgressEvent(ResyncRequired)` to refresh reactive views.
  - Remote sync message upsert queries preserve `messages.snooze_until_epoch_ms` so remote refreshes never wipe local deadlines.
  - `MessageRowSummary` / `MessageRowDto` and `MessageDetailDto` additively expose `#[serde(default)] is_starred: bool` and `#[serde(default)] snooze_until_epoch_ms: Option<i64>`.

## Account-Scoped Search ([04.1.4])

- IPC scope aliases the headless storage `MailboxScope` / `MailboxView` types, retaining
  externally tagged serde enums: `"GlobalAllMail"` or
  `{ "AccountView": { "account_id": "opaque-id", "view": "Inbox" } }`; explicit folders
  use `{ "Folder": "server-path" }`. Omitted/null scope preserves existing global search;
  serialization omits absent scopes to retain the original legacy request shape.
- `SqliteMailStore::search_scoped` extends search without changing `HybridQuery` or the
  `SearchEngine` trait. Unknown accounts, conflicting account filters, and unavailable or
  nonselectable folders fail closed; IPC returns `InvalidRequest`. Inbox resolves an
  advertised inbox role before the case-insensitive canonical INBOX fallback.
- Scope validation, branch ranking, RRF fusion, and final hydration share one read
  transaction and one UTC timestamp. Membership uses account-bound EXISTS with effective
  pending folder overrides, avoiding duplicate rows from provider labels. Canonical
  message folder/UID and attachment identity remain unchanged.
- Inbox excludes deadlines strictly greater than the request timestamp; Snoozed includes
  only those active deadlines. Explicit folders, Starred, and global All Mail retain
  snoozed messages. Expiration affects visibility without requiring reconciliation first.
- Blank and lexical eligibility precede LIMIT/OFFSET. Scoped semantic queries rank all
  eligible existing vectors by exact cosine distance, then message ID; orphan/missing
  embeddings cannot consume page slots. Both hybrid branches apply scope before their
  existing candidate window and RRF fusion. Unscoped optimized paths are unchanged.
- Existing semantic consent gates remain in force. Scoped latency is measured in the
  mailbox validation report; unresolved ADR-008 budgets keep feature closure [04.1.7] open.

## Private Local Account Photos ([04.1.5])

- Separate IPC commands: `cmd_get_account_avatar(accountId)` returns `AccountAvatar | null`,
  `cmd_set_account_avatar(accountId, bytes)` returns the normalized avatar, and
  `cmd_remove_account_avatar(accountId)` returns unit. `AccountAvatar` uses snake_case
  `{ bytes: number[], mime_type: string, width: number, height: number }`. Existing account
  configuration DTOs and message pages never carry image bytes or local paths.
- All commands initialize the existing encrypted runtime first and validate account
  identity. Unknown accounts return `NotFound`; decoding failures return fixed
  `InvalidRequest` errors without paths, metadata, or decoder diagnostics.
- Desktop-only `image` decoding runs on the existing blocking worker: PNG/JPEG/WebP,
  nonempty input <= 2 MiB, strict decoder width/height <= 1024, and a 32 MiB best-effort
  decoder allocation limit. Resize/crop to 128x128 and encode a fresh RGBA PNG, stripping
  original metadata. Validation precedes SQL upsert, preserving the prior photo on failure.
- Storage helpers reuse Migration v2's encrypted `account_avatars` table without schema
  or repository trait changes. Upserts are account-bound; deletion is idempotent.
- AccountList composes AccountPhoto with a separate injectable AvatarTransport/controller.
  Only normalized thumbnails become in-memory data URLs, compatible with the existing
  CSP. Raw files are never previewed; filenames/paths never cross IPC. No photos or
  addresses are copied into localStorage. Disposal clears previews and ignores late
  responses; busy operations prevent duplicate UI writes. Fallback initials and colors
  derive locally from display labels and opaque account IDs. Labels support EN/PT-BR.

## Account Mailbox Navigation & View Controls ([04.1.6])

- `InboxSession` passes its authoritative account-controller list into the mounted inbox.
  `Sidebar` composes account disclosures, normalized local thumbnails, and account-scoped
  Inbox/Starred/Snoozed views. Discovered selectable `junk`/`spam`, archive, and trash
  folders use their exact server paths; remaining selectable folders live under Other
  folders. Headers only toggle expansion; they never change mailbox selection.
- Frontend search scope mirrors the existing externally tagged Rust enums. Global
  requests omit scope. Search, load-more, and refresh send scope to native search rather
  than filtering local pages. Scope changes invalidate generations, reset selection,
  reader, scroll, pagination, and feedback. Late search/refresh/page/mutation results
  cannot replace rows or feedback in another scope. Canonical IMAP identity is untouched.
- `Mailboxes` keeps catalog metadata in memory with per-account request revisions,
  retains the last successful catalog on failures, and invalidates removed accounts.
  Sync refreshes reload catalogs. Only opaque expansion IDs are saved under
  `triage-mail.mailbox-expansion`; photos, addresses, messages, and selected scope are
  never persisted there. Closing settings reloads normalized sidebar thumbnails.
- Explicit star/unstar and snooze/unsnooze actions are available in the message toolbar
  and command palette. Local date/time input becomes UTC epoch milliseconds; native
  deadline validation remains authoritative. Archive/trash clear optimistic snoozes;
  rejected writes roll back only within their original generation. Local-only actions
  have local-only feedback. The reader shows star and active snooze metadata.
- A bounded timer refreshes at the next visible snooze deadline, while window focus and
  native ResyncRequired events refresh expired eligibility after suspension. No frontend
  timer moves mail or changes read flags. EN/PT-BR labels cover added views and controls.
- [04.1.7] still owns full visual/browser verification and scoped performance measurement;
  component regressions and the existing 100k-message DOM-window test are not timing evidence.

## Mailbox Validation Gate ([04.1.7])

- Correlated Inbox/folder predicates now require `idx_memberships_message_id`; Snoozed
  branches choose the account/deadline index using bounded index-only counts in the same
  snapshot, avoiding other accounts' dense snoozes for tiny accounts. Migration v2 supplies the indices.
  Private SQL construction is shared with plan regressions. No schema, IPC or trait changed.
- Final 100k-message/200k-membership run passes 56 complete-order/count/identity workloads
  (1,680 warm samples). Blank Inbox p95 is 35.373ms. Snoozed semantic improves from
  23.540ms to 16.702ms but misses 15ms; Inbox semantic is 106.144ms. Full distributions
  and lexical/hybrid misses are in `docs/performance/mailbox-validation.md`.
- Native fixture now exits 0 after all 12 first-page and both 180-frame scroll reports.
  Inbox semantic/hybrid and Archive semantic/hybrid miss the 100ms submission-to-frame
  budget. Both scroll phases mount <=18 rows; preloaded/paged frame p95 is 23/22ms, which
  does not establish 60fps. The feature remains open; no timing target was weakened.
- Scoped scalar cosine preserves SQL NULL-first distance order for zero-norm stored/query
  vectors. A finite account-KNN frontier can omit a zero-norm vector and is insufficient
  as an exactness certificate. The attempted shortcut was removed and a regression retained;
  see k-021 before revising semantic ranking or the accepted embedding domain.
- **Closure ([04.1.7]):** closed with known deviations by user decision on 2026-11-02. Scoped
  lexical/semantic/hybrid and native first-page budgets remain unmet and 60fps is unestablished;
  see "Closure with known deviations" in `docs/performance/mailbox-validation.md`. Targets are
  unchanged; revisit under [01.1] and a future native UI performance task.


## Explicit Provider Discovery ([02.1])

- `mail_engine::discovery` is headless and injectable through `DiscoveryTransport`.
  Discovery never authenticates, initializes storage/vault, or starts sync workers.
  Unknown domains return `source: Unresolved` and `imap_host: null`; no guessed host
  is presented as a successful discovery. Static aliases match exact normalized domains.
- `cmd_discover_email_provider(email, online?)` defaults to offline. Explicit online
  lookup contacts only `https://autoconfig.thunderbird.net/v1.1/{domain}`. No redirects,
  inherited proxies, full addresses, credentials, retries, or recursive discovery.
  The connection-time resolver validates the exact public-only address set.
- XML is capped at 64 KiB and depth 16. Only IMAP with `socketType: SSL` and valid DNS
  host/u16 port is accepted. DTDs, unknown entities and host templates containing any
  user-specific address part are rejected. XML never supplies trusted OAuth endpoints
  or help URLs. OAuth metadata comes only from audited offline presets.
- `cmd_probe_email_provider(host)` is a separate explicit confirmation action, restricted
  to public DNS hosts on port 993 with a three-second deadline. It uses one validated
  DNS snapshot, verified TLS and a bounded IMAP OK greeting; no commands or credentials
  are sent. Probe success is not proof of sign-in. Manual Proton loopback remains separate.
- Email blur debounces offline lookup by 200ms. Input changes, manual edits, preset
  selection, credential entry and teardown invalidate pending results. No credential
  enters reactive state/storage; actual detached inputs are cleared on mode changes.
  Microsoft setup requires the user's own transient client ID and reuses native PKCE
  cancellation/staging-vault behavior. EN/PT-BR labels remain paired.
- User approved the narrow P0 discovery-network exception and user-provided Microsoft
  client ID scope on 2026-11-02. No first-use shared application registration is implied.

## Binary Candidate Evaluation ([01.1])

- ADR-011 retains exact production int8 cosine search after evaluating ADR-008's
  fixed-budget sign-code proposal. Experimental vec0 bit[384] indexing exists only
  in an offline disposable SQLCipher example; schema v2, IPC and traits are unchanged.
- Exact reranking cannot recover omitted cosine neighbors. Hamming ties can exceed
  sqlite-vec's 4,096 cap; neither a full page nor a finite reranked distance gap proves
  coverage. Zero norms retain k-021 NULL-first scoped ordering. See k-023.
- Reproduce with `cargo run --offline --release -p storage --example binary_quantization`;
  adversarial probes use `cargo test --offline -p storage --example binary_quantization`.
  Raw latency/equivalence data and measurement limits live in
  `docs/performance/binary-quantization.md`. Existing mailbox/native deviations remain.

## Local Rule Engine ([03.5] / ADR-012)

- `triage_ai::rules` owns version-1 Rule/Condition/RuleAction, MessageFacts and RulePlan.
  Matching is all AND optional-any AND no-except, over one immutable snapshot. Missing
  facts never satisfy positive predicates. Text is case-insensitive literal contains;
  domains are exact unless DomainSuffix explicitly includes subdomains. One parsed
  mailbox only; display names never match sender predicates. Sender identity is unauthenticated.
  Amount/date predicates independently test any extracted boleto fact, using inclusive
  integer BRL-cent/ISO-date comparisons; they do not assert a shared boleto across predicates.
- Ordered by position then stable rule ID; distinct tags/star/archive accumulate, matching
  stop rules skip later rules. Star precedes archive in the shared action plan. Existing
  local mutation primitives preserve canonical folder/UID, clear snoozes on archive,
  and queue existing RemoteMutation payloads; no unsubscribe/trash/inverse rule actions.
- Migration v3 adds `local_rules`, `rule_deleted_ids`, `rule_baselines`, `rule_events`, `rule_jobs` and a pending
  job index. First SELECT frontier per folder/UIDVALIDITY persists before ingestion;
  `IncrementalSyncResult.server_uidnext` is additive core metadata, not an IPC change.
  Unknown frontier fails closed. Imports/rebuilds never enqueue history, including resume.
  New canonical inserts atomically snapshot rule revisions and enqueue eligibility with
  checkpoint commit; new memberships, flags and body hydration never enqueue.
- Job completion/local actions/outbox entries share one transaction. Scheduled rules
  must still have identical enabled revisions; edits/reorders invalidate pending versions.
  Account-bound hashed event identities survive cache deletion/MOVE without COPYUID;
  processed jobs clear snapshot definitions, while tombstones remain until account deletion.
  See k-025 for conservative identity and initialization limitations.
- Additive IPC: `cmd_list_rules`, `cmd_save_rule(rule)` -> saved incremented revision,
  `cmd_delete_rule(id,revision)`, `cmd_reorder_rules(revisions: [id,revision][])`,
  `cmd_simulate_rules(ids,draft?)` -> `[message_id,RulePlan][]`, and
  `cmd_rule_draft(messageId,ruleId)` -> unsaved disabled draft. Native validation and
  compare-and-swap reject malformed/stale requests; deleted IDs cannot be reused. Simulations never write/deliver.
- Settings Rules module supports CRUD, disable/reorder, condition explanations and
  simulation of the selected inbox message. Message action creates an editable draft;
  it does not save/enable it. EN/PT-BR labels, account selectors, dirty/busy modal guards
  and unmount cleanup apply. No mail/rule facts enter browser persistence.
- All core code stays headless; desktop composes RuleObserver through TriageEventHook
  once per committed batch and retries bounded durable jobs in the existing outbox loop.
  Errors emit fixed MutationFailed feedback. No network policy exception is introduced.

## Consent-Bound Alerts ([03.6] / ADR-013)

- `triage_ai::alerts` owns destination/field/outcome contracts, a headless DeliveryAdapter
  and bounded allowlist serialization. `RuleAction::Alert(String)` is additive to v1 rules.
  The planner deduplicates destinations; rule order/stop semantics remain authoritative.
- Migration v4 adds `alert_destinations`, `alert_deleted_ids`, `alert_secret_intents`,
  `alert_dedup` and `alert_jobs`; rule_jobs gains default-empty destination revision/event
  snapshots. Pre-upgrade jobs cannot send alerts. New ingestion freezes enabled destination
  revisions; enqueue and rule completion/mutations share one transaction. Automatic markers
  survive history pruning until account/destination deletion; ID tombstones prevent reuse.
- `cmd_list_alert_destinations`, `cmd_save_alert_destination(destination,secret?,consent)`,
  `cmd_delete_alert_destination(id,revision)`, `cmd_alert_history`, `cmd_test_alert(id,revision,consent)`,
  `cmd_replay_alert(ids,destination,revision,consent)` and `cmd_local_alert_available` are additive.
  DTOs use snake_case fields and existing camelCase invoke argument conventions. Complete
  URL/bearer material is input-only with fixed deserialization errors; list/save returns safe
  metadata/public origin and presence flags. Enabling confirms origin and selected fields.
- Desktop gates CRUD/dispatch against concurrent configuration changes; stale test/replay
  confirmation fails. Vault immutable secret revisions are registered in SQL before writing,
  referenced atomically and cleaned when unreferenced. Cleanup retries after restart.
  Changing any destination revision cancels pending work; re-enabling never replays it.
- Queue leases/rate gates persist across restart. One start/ten seconds/destination; desktop
  runs one globally serial attempt per worker tick, <=10s each. Up to eight attempts/24h,
  bounded jitter/backoff and Retry-After. 2xx means accepted, transport/408/429/5xx retry,
  redirects/other 4xx fail. Event ID/Idempotency-Key stays stable; duplicate receipt is possible.
- HTTPS public DNS/443 only; pinned validated address sets, verified TLS1.2+, no inherited
  proxy or redirects. DNS lookup occurs only during consented dispatch. Responses are not
  read into payload/history. Full URL and auth live only in vault; no private-network bypass.
- Payload defaults to schema/event/time/truncation. Optional fields are allowlisted; bodies
  bounded to 6,000 bytes, excerpts 512, attachment/extraction collections 16. Raw HTML/headers,
  OTP/Pix badges and binary attachments never export implicitly or trigger lazy fetches.
  Terminal jobs clear payload; history is <=30 days/1,000 records per destination, DTO <=200.
  Account/message removal deletes jobs; compact automatic deduplication survives message removal.
- EN/PT-BR Alerts settings supports destination CRUD/consent, field previews, explicit test/replay
  confirmation and safe history. Rule editor selects destinations, including removal of stale
  references. Credentials remain detached inputs with cleanup; modal dirty/busy guards persist.
  Local notices use Linux D-Bus and generic text only; unavailable/denied platforms fail visibly.
  Runtime must stay unlocked/active. No runnable headless CLI or provider-specific connector added.

## Telegram and Discord Connectors ([03.7] / ADR-014)

- Additive `DestinationKind::Telegram` / `Discord` reuse v4 JSON destinations, immutable
  vault revisions, ingestion snapshots, rules, queue, consent, tests/replay and history.
  No migration, existing command argument or trait signature changes. Input-only secret
  additionally accepts `{ token, chat_id }`; existing `{ url, bearer }` still works.
- Typed Telegram input normalizes to the existing vault-only envelope: fixed sendMessage
  URL containing token plus numeric chat ID in the bearer slot. Never send that slot as
  Authorization. Discord uses a canonical discord.com/api/webhooks URL with no bearer.
  Kind changes require complete new material; preserving secrets is same-kind only.
  List/save responses contain only fixed public origins and credential-presence flags.
- Explicit recipient/field confirmation is required on enablement; replacement/edit
  resets UI consent and invalidates pending revisions. Secret inputs stay detached and
  clear on submit, kind switch and teardown. No setup/save contacts a provider; no chat
  discovery, username routing, automatic migration, remote revocation or attachments.
- One consent-filtered JSON text per event; conservative UTF-16 limits 3500/1800 with
  visible truncation. Telegram plain text disables previews; Discord disables mentions
  and embeds. Pure formatting/response classification stays in triage-ai. Response
  buffering is bounded to 64KiB, transient/zeroized; no response text escapes dispatch.
- P0's approved provider exception retains TLS/public-only pinned DNS/443/no proxy/no
  redirects and total ten-second attempt timeout. DNS deadline is 3s, <=32 addresses.
  Retry delays and backoff persist next_start across every destination of the same
  provider conservatively, including shared credentials; no secret fingerprint in SQL.
- Existing acceptance/duplicate/active-runner limits apply. Revoke credentials manually
  in BotFather/Discord; local disable/delete does not revoke them or recall messages.
  Offline validation does not establish live provider interoperability. See k-027.

## Saved Searches and Virtual Folders ([04.10] / ADR-015)

- Append-only migration v5 adds `saved_views` and `saved_view_deleted_ids`. Version-1
  definitions contain ID/revision/name, persisted query/mode, account selection, optional
  existing mailbox scope and typed AND filters. Empty account selection means all;
  missing explicit accounts/scopes fail closed at save/search, never broaden. Listing
  and deletion remain possible. Revision-zero creates; updates/delete compare-and-swap;
  deleted identities cannot be reused. At most 200 definitions; no account limit added.
- Additive IPC: `cmd_list_saved_views`, `cmd_save_saved_view(view)`,
  `cmd_delete_saved_view(id,revision)`, `cmd_search_saved_view(id,revision,limit,offset)`.
  Search resolves stored text/mode before embedding and rechecks revision during execution.
  Existing SearchQueryDto, MailboxScope, HybridQuery and SearchEngine are unchanged.
- Scoped ranking now internally accepts general SQL eligibility while retaining original
  mailbox SQL/account index behavior. Saved predicates precede branch LIMIT/OFFSET and
  hybrid candidate windows. Definition validation, eligibility/ranking/hydration share one
  read snapshot and UTC clock. Sender parsing reuses triage-ai; missing/invalid extraction
  facts fail closed. Registered deterministic SQL closure is removed after each search.
  Tags use exact all-required matching; sender domains are exact, unauthenticated.
- Date bounds refer to one valid extracted boleto date; relative upcoming windows include
  today through N days, default 30, in UTC calendar dates. No payment inference. Semantic
  consent/model gates and exact cosine NULL/tie behavior remain authoritative. No vectors,
  credentials or mail facts enter browser persistence; definitions stay SQLCipher-only.
- EN/PT-BR sidebar CRUD/starter drafts use a guarded Bits UI dialog. Active edits reset
  search generation/selection/reader/pagination; late reads cannot overwrite another view.
  Deleting active views returns to All Mail. Existing sync/triage/focus refresh applies;
  a rearmed UTC-midnight timer updates date membership. No rules, moves or deliveries run.
- Synthetic large-fixture measurements are in `docs/performance/saved-views.md`; narrow
  starred workloads do not establish arbitrary-predicate latency or native 60fps. Existing
  [04.1.7] known deviations and budgets remain unchanged. See k-028 for lifecycle/SQL details.

## Due-Date Dashboard & Scheduled Reminders ([04.11] / ADR-016)

- Append-only migration v6 adds `reminder_configs`, `reminder_config_deleted_ids`, `reminder_jobs`, and `due_date_dismissals`.
- Version-1 `ScheduledReminderConfig` models have revision optimistic locks, maximum 50 rules, trigger variant `DueDate` (`days_before` <= 365, `target_hour` 0..23, optional `min_amount_cents`) or `UnreadInterval` (`interval_hours` 1..8760, optional `min_amount_cents`), account filter, and 1..32 destination IDs matching configured alert destinations.
- Civil date arithmetic uses Howard Hinnant's algorithm (`day_number` / `date_from_day_number`) without timezone or system clock dependencies for civil dates, and maps to target UTC seconds using local timezone offset (`local_tz_offset_seconds()`).
- Invariant: Extraction never implies payment. Extracted dates from boletos and invoices categorize strictly as `Overdue`, `DueToday`, `Upcoming`, or `Dismissed`. Non-payment disclaimer is mandatory and prominent in both EN and PT-BR.
- Reminders dispatch reuses the existing alert pipeline (`alert_jobs`), benefiting from rate-limiting, backoff, vault credential isolation, and configured destinations (Local, Webhook, Telegram, Discord). Reminders delayed > 15 minutes due to asleep runners are tagged `[Delayed Reminder]`.
- Reading a message cancels all its pending `UnreadInterval` reminder jobs immediately (`cancel_reminders_for_read_messages`). Explicit dismissal records in `due_date_dismissals` and suppresses all pending reminder jobs for that message.
- Additive IPC commands: `cmd_list_due_date_items(accountId)`, `cmd_dismiss_due_date_item(messageId)`, `cmd_list_reminder_configs`, `cmd_save_reminder_config(config)`, `cmd_delete_reminder_config(id, revision)`.
- UI in `apps/desktop` provides Due-Date Dashboard modal, KPI summary cards (Overdue, Due Today, Upcoming), status filter tabs, 1-click clipboard barcode copy with feedback, direct navigation to email view, and Settings integration (`RemindersModule`) with full EN / PT-BR parity.

## Local Automatic Categories ([03.8] / ADR-017)

- Existing category wire values remain; Notifications is additive. Pure bounded local
  classification returns version/reason. Priority: Security > Financial > Shopping >
  Newsletter > Notifications > Personal fallback. Personal does not imply a human sender.
- Append-only v7 stores message_categories, category_corrections and category_sender_overrides.
  Effective priority is message correction > account/exact-sender override > automatic.
  Nullable resets keep revisions against ABA; sender normalization reuses rules::sender_address.
  Badges remain independent. Canonical IMAP identity and existing traits/DTO shapes remain.
- Corrections use account-bound hashed provider ID, else valid Message-ID + received time.
  Record-only fallback corrections are removed with the message; reliable identities survive
  cache deletion until account removal. First assignment stays stable through MOVE/hydration.
  Missing/changed identities after re-ingestion cannot recover prior corrections. See k-029.
- Ingestion refreshes automatic metadata atomically; fact changes invalidate its version.
  Active unlocked desktop backfills <=300 cached rows per local batch, yielding between batches independently
  of the outbox cadence, resumably, without
  rules/reminders/delivery/network work. Until processed, history uses legacy automatic
  badges and sender overrides await normalized metadata. Errors emit MutationFailed.
- Additive IPC: cmd_category_metadata(messageId), cmd_correct_category(messageId,revision,category?),
  cmd_list_category_overrides(offset?) (<=200 active rows), cmd_save_category_override(value)
  (null category removes), cmd_search_category(query,category). Native revisions reject stale
  writes. Category SQL eligibility precedes pagination/both hybrid windows, shares snapshots
  with hydration, and respects existing semantic opt-in. Saved filters add default-null category.
- EN/PT-BR category controls use native result membership and existing virtualization.
  Per-message corrections and separately consented sender overrides disclose provenance,
  automatic reasons and record-only limits. Settings manages sender removal; saved current
  search retains category. Scope/view changes invalidate late reads; no browser persistence.
  Categories never move mail, hide extraction badges, train models or execute automation.

## New-Sender Review ([04.12] / ADR-018)

- Append-only v8 adds sender_review_preferences, sender_review_choices,
  sender_review_messages and sender_review_frontiers. All are SQLCipher-local and
  account-bound; choices survive cache deletion until account removal. Existing core
  traits, IPC DTOs and earlier migrations retain their contracts.
- Shared rules::sender_address parses exactly one mailbox and ASCII-lowercases it;
  display names/domains/provider aliases never substitute for exact identity. Invalid,
  ambiguous or oversized identities remain visible without sender-wide actions.
  Familiarity never authenticates the author. Known is history, not explicit acceptance.
- Default disabled; enabled review-only is default. Cached history seeds in <=300-row
  local batches before activation. First post-enable per-folder UIDVALIDITY/UIDNEXT
  frontiers persist before ingestion; interrupted history, new folders and rebuilds
  seed Known. Unknown UIDNEXT seeds conservatively until a usable frontier is captured.
  New canonical arrivals above established frontiers insert Pending only while active.
  Every observation is insert-only for sender choices; history cannot erase a decision.
- Revision-checked Accepted/Declined/Pending choices affect existing/future exact-sender
  mail in one account. Disable immediately bypasses filtering and new observations seed
  Known; re-enable seeds intervening history, retaining existing choices and Pending.
  Duplicate memberships, flags, hydration and MOVE cannot create another review choice.
- Additive IPC: cmd_review_preferences(accountId), cmd_save_review_preferences(value),
  cmd_review_metadata(messageId), cmd_list_sender_choices(accountId,offset),
  cmd_save_sender_choice(value), cmd_search_sender_review(query,reviewState).
  Review state is Pending/Declined for search; query scope supplies account only.
  Native SQL eligibility precedes pagination and both hybrid windows in one snapshot.
- Explicit per-account Inbox hiding excludes Pending/Declined only in ordinary Inbox
  search. All Mail, other mailboxes, categories, saved searches and review bypass it.
  Nothing blocks delivery, deletes/moves mail, changes categories, or gates independent
  rules/reminders/alerts. Initialization never fetches data or executes automation.
- EN/PT-BR navigation/actions/settings retain virtualization, sandboxed rendering,
  request invalidation, exact-sender/account consent, dirty/busy guards and cleanup.
  No review/mail facts enter browser persistence. Existing search/native budgets and
  known deviations remain; synthetic timings are not a native 60fps claim.


## Conversation Grouping and Snapshot Triage ([04.13] / ADR-019)

- Migration v9 adds normalized conversation_inputs, indexed conversation_members,
  dirty-account staging, opaque epoch, expiring conversation_previews and safe
  conversation_attempts. Existing schemas/traits/message IPC remain unchanged.
- Header IDs are case-sensitive conservative ASCII, <=998 bytes; references <=64/64KiB.
  No subject/sender/account-crossing fallback. Duplicate IDs cannot resolve links;
  missing ancestors are account-local placeholders. Iterative components retain every
  canonical record, including cycles/singletons. Group IDs can merge/split/change.
- Backfill stages <=300 headers per tick, then atomically consolidates one complete
  dirty account using all compact inputs. Final consolidation is linear account-wide
  work under the storage lock. Check progress.ready, not just a zero staging count.
  Global grouping waits for all accounts; account views check relevant readiness.
- Additive IPC: cmd_conversation_progress(accountId?), cmd_search_conversations(query,context),
  cmd_conversation_members(query,context,id,offset), cmd_preview_conversation(query,context,id,all,action),
  cmd_apply_conversation(token), cmd_conversation_status(ids). Context carries source,
  scope and optional generation. Member pages <=100; group pages <=500. Saved definitions
  resolve natively before embedding. Existing semantic opt-in gates remain authoritative.
- Native filtering/ranking precedes group pagination. Best eligible message represents
  each group; hybrid group RRF uses complete eligible branch rankings, while existing
  message search retains its candidate windows. Hydrate final pages only; matching,
  cached and unread counts remain distinct. Outside-view members are explicitly labeled.
- Preview defaults to matching records, all-cached requires selection. Native immutable
  IDs/action, five-minute expiration and generation checks reject stale confirmation.
  Consumption/check/local mutations/unread-reminder cancellation/outbox inserts share
  one transaction. >500 fails without truncation. Generations include UTC day and snooze
  boundaries as well as database/vector changes. Unrelated writes can invalidate them.
- EN/PT-BR optional grouped mode retains virtualization and per-message lazy sandboxed
  readers. No automatic thread read or browser persistence. Only read/unread/archive/tag
  conversation actions; no thread unsubscribe/trash/rule replay. Remote operations retry
  independently; per-message failure/attempt metadata has no provider response details.
  Existing performance targets/deviations remain; see k-031 for indexing and snapshot risks.

## Alert Digests and Quiet Hours ([03.9] / ADR-020)

- Append-only v10 adds alert_schedule_policies/items/cursor; rule_jobs gains default-empty
  schedule_revisions and alert_jobs gains nullable schedule_revision and default-false urgent.
  Existing destination DTO, Alert(String), IPC commands and delivery trait stay compatible.
- Pure alerts::schedule contracts accept ScheduleCalendar. Desktop resolves System Local
  or UTC using chrono; no named-zone selector/dependency/download. Quiet hours are [start,end),
  overnight allowed, equal endpoints invalid. DST gaps advance, overlaps choose earlier;
  repeated-hour quiet boundaries still gate the later occurrence. Unsealed dates recompute.
- Per-destination Immediate/Hourly/Daily policies use CAS for destination and policy revisions.
  Ingestion snapshots policy revisions; policy edits cancel old automatic work and stale
  rule jobs cannot enqueue. No history replay. Additive UrgentAlert(String) needs destination
  urgent_bypass permission; otherwise routine. Ordered/stop-filtered urgent wins once.
- Worker stages <=32 destinations round-robin and <=1,000 items each, with overflow counts.
  Missed slots coalesce into one immutable summary; unresolved summaries block additional
  sealing. <=50 entries/64 KiB from selected-field snapshots only, with omitted/total counts.
  Age uses oldest constituent, never seal time; unsent items expire after 24h. Stable civil
  watermarks suppress repeated/rolled-back slots, conservatively expiring those items.
- Message/account removal cancels whole unsent summaries containing them, clearing payloads;
  no surviving-item replay. Read/dismiss/reminder config edits cancel relevant staged/sealed
  reminders. Test/replay bypass schedules with visible confirmation; rate/consent gates remain.
- Additive IPC: cmd_alert_schedule(id), cmd_save_alert_schedule(policy,consent),
  cmd_preview_alert_schedule(policy). Read returns policy and safe deferred/summarized/expired/
  overflow counts. Preview performs no delivery. EN/PT-BR Alerts editor and urgent rule actions
  retain guards and stale/unmount cleanup. No browser persistence. See k-032.
- Native claim_alert_with_calendar gates actual retry candidates independently of staging
  batches. Headless hosts must supply their calendar for local-zone policies; legacy
  claim_alert uses the UTC adapter and fails closed for local-zone quiet hours. Existing network/privacy/lease/history limits remain.

## Encrypted Backup and Portable Definitions ([01.4] / ADR-021)

- Additive headless vault::backup authenticated record streams and storage::{backup,
  portable} APIs. No migration, dependency or existing trait/signature changes.
  `TMBACK01` fixes Argon2id/AES-GCM parameters; encrypted sequence/final records bind
  order and completeness. Encrypted manifest version1/schema10 only; reject unknowns.
  Bounds: 64MiB records, 64GiB native input, ten million rows, 4MiB portable JSON.
- One read transaction snapshots committed WAL data across explicit allowlisted tables.
  No plaintext database staging, vault/source keys, credentials, connection configs,
  secret URLs, vault references or pending work/history/previews. Cached mail/metadata,
  local state/definitions and tokenizer persist. Derived indexes rebuild as applicable;
  models/external attachments/browser preferences are omitted. See docs/backup.md.
- Restore only before runtime initialization into an absent app-data directory.
  recovery_gate serializes initialization/recovery. Fresh private sibling staging
  database/vault authenticate/validate before durable directory publication. Failures
  preserve existing installations; crash-abandoned sibling stages stay inert. Recovery
  marker forces fallback-only vault unlock with the backup passphrase, bypassing keyring.
- Rules/reminders/destinations/sender review stay disabled; AI/network consent and
  urgent bypass reset. Queues never replay. Cached-only AccountConfigDto list entries
  use empty imap_host (existing connected entries unchanged); workers skip them.
  Explicit add_account with exact recovered email preserves its ID/cache and attaches
  new credentials. Existing connected IDs remain non-replaceable.
- Portable JSON placeholders require complete explicit account/destination mappings;
  unknown targets/mailbox scopes fail closed. Native preview comparison guards file
  changes. Atomic append uses fresh IDs/revisions, disabled rules, no historical jobs.
  Names/query/predicate text is readable and can contain private data; disclose it.
- Additive IPC: cmd_choose_transfer_file(save), cmd_write_backup(path,passphrase),
  cmd_export_definitions(path), cmd_preview_definitions(path),
  cmd_import_definitions(path,mapping,expected,consent), cmd_recovery_available,
  cmd_restore_backup(path,passphrase,consent). Passphrase input uses fixed errors and
  zeroizing backend memory; no passphrase is returned or persisted in browser state.
  Native paths refuse occupied outputs, symlinks and traversal. Linux FileChooser
  portal uses existing zbus; unavailable/other-platform selection reports failure.
- EN/PT-BR Settings/startup controls preserve transient credentials, busy/dirty guards,
  preview/reference confirmation and teardown invalidation. No provider validation or
  network call. See k-033; offline tests do not establish live portal interoperability.

## Learned CPU ONNX Embeddings ([03.10] / ADR-022)

- Additive OnnxEmbeddingProvider retains EmbeddingProvider and legacy exported projection
  APIs. Production composes only the learned provider, with no implicit fallback. ort
  rc.10/tokenizers 0.21.4 have automatic-download/default features disabled; CPU runtime
  1.22.0 initializes with telemetry off, from an explicitly installed verified library.
- Model/tokenizer and platform-library SHA-256 manifests are compiled in. MiniLM revision
  751bff37182d3f1213fa05d7196b954e230abad9 yields finite float32 [1,tokens,384], masked
  mean/L2/absmax int8 output. Inputs are UTF-8 bounded to 32KiB/256 tokens. One session
  serializes inference with 2 intra-op/1 inter-op threads. No inference path fetches files.
- Append-only v11 adds embedding_generation and embedding_work plus source invalidation
  triggers/index. Activation removes unversioned vectors; generation/epoch/source-version
  CAS rejects stale results after hydration/deletion/rebuild. Restart resumes completed
  work. Eight-document blocking batches run only under semantic opt-in, yield separately
  from automation loops and never execute rules/reminders/deliveries or fetch mail.
- All semantic entry points gate compatible complete managed indexes in the ranking
  snapshot. New pending work temporarily gates semantic availability. Managed global
  hybrid uses existing sequential exact ranking within one transaction; legacy unmanaged
  headless parallel behavior remains. Existing exactness/latency targets are unchanged.
- Additive cmd_embedding_state and cmd_rebuild_embeddings(consent) expose only safe
  statuses/counts and explicit local rebuild. Existing DTO/IPC/experimental wire names
  remain. EN/PT-BR controls show progress/missing-or-invalid files/failures, retain
  busy/confirmation guards and invalidate polling/action completions on teardown.
- Missing model/runtime cannot block lexical startup. Failed work requires confirmed
  rebuild; disable prevents further commits, re-enable resumes. Backup v1 retains its
  schema-10 allowlist, excluding v11 derived state; restored mail needs fresh indexing
  and semantic consent. No backup format/migration rewrite or automation replay.
- Model/runtime files are not bundled; explicit setup and source licenses/hashes are
  in docs/onnx.md. Native execution was validated on Linux x64 only; other platform
  hashes do not establish native interoperability. See k-035 for loader and lifecycle
  gotchas. Existing search/UI performance deviations and language-quality limits remain.

## Offline Cached-Mail CLI ([00.3] / ADR-023)

- `apps/cli` builds `triage-mail-cli` without Tauri/GTK/WebKit, using existing core
  dependencies only. It does not start workers, load inference, fetch data or deliver.
- Explicit existing `--data-dir` only. Default keyring read never provisions a key;
  `--unlock-stdin` uses only existing fallback vault and bounded non-terminal UTF-8
  input with zeroized buffers. Recovery marker forbids keyring mode. Additive
  `vault::existing::ExistingVaultReader` bounds reads to 16 MiB without persistence.
- Additive `storage::cached::CachedMailReader` owns a private READONLY/query_only
  connection. Exact schema-11 migration sequence validation and reads share snapshots.
  No migration/journal-mode change/immutable flag. Existing lexical ranking and stored
  tokenizer remain authoritative; no implicit mailbox/review/category eligibility.
- `accounts` returns opaque IDs with pagination. `search` returns existing summaries
  with exact optional account scope. `show` reads only 64 KiB cached plain-text bytes,
  omits incomplete final UTF-8 scalars and reports truncation/null when unavailable.
  No mark-read or HTML conversion. No account limit; page limit 1..500.
- Responses serialize fully before stdout within 4 MiB including LF; diagnostics
  never echo private values. Output limits do not cap internal search/hydration memory.
  Direct path symlinks are refused, without claiming race-proof filesystem isolation.
  WAL/SHM files and directory access may be needed; pagination across invocations is
  not a stable snapshot. See docs/cli.md and k-036 for boundaries and exit codes.

## Offline CLI Container ([00.4])

- `scripts/build-cli-image.sh` builds the locked CLI offline on Linux x86_64 GNU,
  packages it from scratch with an explicit four-library ELF allowlist and license
  notices, and deletes its private generated context. No pulls, package downloads,
  source/data/vault context, desktop/ONNX runtime or shell entrypoint. Host/toolchain
  library identity affects the resulting image; this is not a reproducible build.
- Default UID/GID 65532; run with a nonzero authorized host UID/GID, read-only root
  and data mount, network none, no capabilities/new privileges, no Docker logs and
  bounded private tmpfs. Only existing fallback vault via stdin, no D-Bus/keyring
  socket or secret environment/argument. Existing CLI/schema contracts are unchanged.
- Readable initialized WAL/SHM sidecars are required for container WAL reads. Missing
  SHM can fail with exit 4 under the read-only mount; never repair with writable data
  or immutable mode. Opt-in synthetic Docker test proves command parity, recovery
  unlock, refusals, WAL visibility and durable/logical-state preservation. Native
  readers can update SHM read marks; do not assert its bytes stay unchanged.
- Local validation is Linux x86_64/Docker Desktop only; no registry publication,
  active synchronization/daemon/automation or cross-platform guarantee. See docs/cli.md
  and k-037 for build, runtime and sidecar restrictions.

## Shared Sync Orchestration ([00.5] / ADR-024)

- Additive pure Rust `sync-runtime::{SyncHost, sync_account_once}` binds one account
  through the host. Reuses existing mail-engine/storage DTOs without changing their
  traits or migrations. No Tauri/GTK/WebKit dependency, bootstrap or spawned workers.
- Coordinator owns folder selection/fallback, existing 120-second remote deadline,
  batch sequencing and reset/checkpoint construction. Catalog save is best effort;
  remote UIDVALIDITY zero fails. Each commit completes before the next checkpoint.
- Desktop host retains gate/blocking execution, baseline/frontier-before-ingestion,
  atomic message/flag/checkpoint write and original rule/reminder/index/event order.
  Rule invalidation events remain independent of final buffered success progress.
- Cancellation preserves committed work; already running host blocking operations
  may continue. Neither the entire pass nor host storage/discovery is newly bounded.
  No active CLI/daemon, process ownership or writable container contract is added.
  See k-038 for ordering and cancellation limits; offline CLI contracts remain intact.
