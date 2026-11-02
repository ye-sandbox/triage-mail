---
author_type: human
author: Architecture Team
reviewed_by: Human Lead
date: 2026-10-29
status: accepted
type: architectural-decision-log
---

# Architectural Decisions Log

> 🏛️ **Progressive Disclosure:** Historical and cumulative decisions live here so AI agents do not load them into working context on every interaction.
> Active runtime contracts stay in [`.agent/NOTES.md`](../../.agent/NOTES.md).

---

## Cumulative Decisions

### [2026-10-29] ADR-001: Tauri v2 Pure Native Stack (Rust + SvelteKit 5 SPA)

- **Context:** Traditional desktop email clients built on Electron or bundled with Python/Node sidecars consume hundreds of megabytes of idle RAM and produce bloated binaries (>150 MB), degrading responsiveness when indexing 100k+ emails locally.
- **Decision:** Adopt Tauri v2 with a pure Rust backend (`tokio`) and a SvelteKit 5 SPA frontend (`@sveltejs/adapter-static`). Keep the core desktop binary lean and load ONNX embedding model weights lazily on demand into the user's local data directory (`$XDG_DATA_HOME/triage-mail/models`).
- **Alternatives Considered:**
  - *Electron + Node.js:* Rejected due to Chromium bundle bloat and high memory baseline.
  - *Tauri + Python sidecar (for AI/Embeddings):* Rejected due to packaging complexity, cold-start latency, and high memory overhead.
  - *Bundling static ONNX weights inside the executable:* Rejected to keep the core binary small and allow users to configure or swap embedding models.
- **Consequences:** All IMAP networking, MIME parsing, SQLite/SQLCipher storage, and ONNX embedding inference execute natively in Rust crates (`mail-engine`, `storage`, `vault`, `triage-ai`).

### [2026-10-29] ADR-002: Headless-Capable Cargo Workspace, Stalwart `mail-parser` & Agnostic `ImapConnector`

- **Context:** While the primary MVP target is the desktop GUI (`apps/desktop`), users and power operators may want to run the synchronization and triage engine headless (e.g., inside a homelab Docker container or CLI) in the future. Additionally, Rust IMAP libraries (`async-imap`, `imap-next`) evolve rapidly and email providers exhibit incompatible protocol quirks (Gmail virtual folders/`X-GM-EXT-1`, Outlook `XOAUTH2` SASL framing and `CONDSTORE` gaps, Proton Mail Bridge loopback TLS), while offering diverse authentication paths (App Passwords, OAuth2 Public PKCE, user-supplied BYO-OAuth credentials, or local bridges).
- **Decision:** Structure the repository as a Cargo Workspace where `crates/mail-engine`, `crates/storage`, `crates/vault`, and `crates/triage-ai` have zero dependencies on `tauri`. Standardize RFC 5322 / MIME parsing on **Stalwart Labs' `mail-parser`** crate for zero-copy header/threading extraction (`Message-ID`, `In-Reply-To`, `References`, `List-Unsubscribe-Post`) and lazy attachment part indexing. Encapsulate IMAP transport strictly behind an agnostic `ImapConnector` trait with incremental sync state (`UIDVALIDITY`, `UIDNEXT`, `CONDSTORE`/`HIGHESTMODSEQ`) and provider quirk normalization inspired by **`pimalaya` (`email-lib`)**, supporting maximum transparent authentication flexibility: App Passwords, standard IMAP, OAuth2 PKCE (`client_id` public flow or user-provided BYO `client_id`/`client_secret` stored in `vault`), and Proton Mail Bridge (`127.0.0.1`).
- **Alternatives Considered:**
  - *Monolithic `src-tauri` crate:* Rejected because embedding `tauri::AppHandle` inside IMAP or storage routines prevents headless CLI/container compilation.
  - *Legacy/custom MIME parsers or full `RFC822` fetches:* Rejected in favor of Stalwart's battle-tested zero-copy `mail-parser` combined with lightweight `BODYSTRUCTURE` + text/HTML part fetching.
  - *Leaking underlying IMAP crate types or ignoring provider quirks:* Rejected to protect domain code from upstream crate churn and normalize Gmail/Outlook/Proton differences inside `ImapConnector` (see `.agent/knowledge/k-002-imap-provider-quirks-and-mime-parsing.md`).
- **Consequences:** Core crates communicate progress via Rust traits (`SyncEventSink`), which `src-tauri` implements using `tauri::ipc::Channel`.

### [2026-10-29] ADR-003: Single-File SQLCipher Storage with Configurable FTS5 + `sqlite-vec` and Lazy Attachments

- **Context:** Storing 100k+ messages locally requires sub-15ms hybrid search (lexical + semantic) and strong encryption at rest without managing external database daemons or exhausting disk space with binary attachments.
- **Decision:** Use SQLite in WAL mode with full-database encryption via **SQLCipher (AES-256)** (avoiding redundant column-level encryption that would break FTS indexing). Default lexical search to `FTS5` with `unicode61 remove_diacritics 2` for body content (configurable by the user, with optional `trigram` on headers/subjects) and embedded `sqlite-vec` with scalar quantization (`int8`) and metadata/lexical pre-filtering for `< 15ms` KNN vector search. Enforce **Lazy Attachments**: background sync downloads only headers, flags, and text/HTML bodies; binary attachments are fetched on-demand via `IMAP BODY.PEEK`.
- **Alternatives Considered:**
  - *Column-level `aes-gcm` encryption on message bodies:* Rejected because encrypting individual text columns prevents SQLite `FTS5` from indexing message text.
  - *External Qdrant/Postgres/Meilisearch:* Rejected to preserve single-file local-first simplicity and zero external runtime daemons.
  - *Full `RFC822` eager sync:* Rejected because downloading multi-megabyte attachments in background saturates bandwidth and bloats the SQLite file.
- **Consequences:** Database keys are provisioned via the OS keyring (`crates/vault` using `keyring` + `argon2` + `aes-gcm` encrypted fallback vault for headless environments). `crates/storage` uses `#![deny(unsafe_code)]` with an isolated `#[allow(unsafe_code)]` block strictly for `sqlite3_auto_extension` (`sqlite-vec` FFI registration), while all other crates enforce `#![forbid(unsafe_code)]`.

### [2026-10-29] ADR-004: 3-Tier Local-First AI Pipeline, One-Click Unsubscribe & Strict HTML Sandboxing

- **Context:** Triage extraction (2FA codes, Pix/Boletos, NF-e, package tracking, categorization) must be instantaneous and work 100% offline with zero cost, while semantic search and summarization should scale from local CPU to optional LLMs without compromising privacy.
- **Decision:** Implement a 3-Tier AI architecture in `crates/triage-ai`:
  1. *Tier 1 (Deterministic - Zero Cost):* Fast Rust regex/heuristics for immediate extraction of 2FA/OTP codes, FEBRABAN Boletos (including deterministic extraction of exact **BRL amount** and **due date** from the 47/48-digit barcode factor), Pix EMV payloads/keys, NF-e/invoice metadata, tracking numbers, and `List-Unsubscribe` + RFC 8058 `List-Unsubscribe-Post` (`One-Click` HTTP POST) headers.
  2. *Tier 2 (Local CPU Embeddings):* `fastembed-rs` / `ort` running quantized embedding models (`all-MiniLM-L6-v2` or `bge-small` downloaded on demand to user data dir) in background threads.
  3. *Tier 3 (Optional Generative LLM):* Async HTTP client (`reqwest`) connecting to local Ollama (`http://127.0.0.1:11434`) or explicit user-configured BYOK providers.
  For email rendering, enforce `DOMPurify` + `<iframe sandbox="">` with scripts disabled, remote tracking pixels blocked by default, and a normalized base CSS stylesheet for dark-mode contrast and layout safety.
- **Alternatives Considered:**
  - *Cloud-only LLM triage:* Rejected due to privacy violations, latency, and API token cost on high email volume.
- **Consequences:** Zero external telemetry or unauthorized requests leave the machine.

### [2026-10-29] ADR-005: GNU AGPLv3 Licensing & Lean Event Hook Extensibility

- **Context:** The project is developed initially in a private repository for MVP validation before transitioning to public open-source. It must guarantee user freedom while preventing third parties from offering closed-source commercial derivatives.
- **Decision:** License under **GNU AGPLv3**. Enforce strict Git hygiene from commit zero (zero hardcoded secrets, tokens, or personal email addresses in tracked files or tests; user-provided OAuth credentials live exclusively in `vault` / untracked local config). Expose a single clean `TriageEventHook` / `SyncEventSink` trait boundary for post-triage extensibility without premature cloud-sync traits or artificial account limits.
- **Alternatives Considered:**
  - *MIT / Apache-2.0:* Rejected because third parties could wrap the engine into proprietary SaaS or closed commercial forks without contributing back.
- **Consequences:** Every commit is audited for secret/PII cleanliness so the full git history can be made public safely.


### [2026-10-31] ADR-006: Derived Account-Partitioned Vector Index (Superseded by ADR-007)

- **Context:** Correct account-aware scalar vector ranking at 100k messages takes about 380ms.
  Filtering global KNN output underfills pages; per-message vector lookups defeat contiguous scans.
- **Decision:** Approved [90.3] adds migration v5 with `vec_messages_by_account`, using opaque
  account IDs as vec0 partition keys and unchanged int8[384] cosine vectors. Preserve legacy
  `vec_messages` as authoritative. Backfill only live messages transactionally; embedding writes
  update both tables together and message triggers reconcile insertion/deletion/account changes.
  Use partition-constrained KNN, with the existing exact tie/deep-page fallback. Keep IPC and
  core trait signatures unchanged. Use a measured 64MiB connection-local SQLite page-cache target.
- **Alternatives Considered:** Replacing the legacy vector table would break its established
  schema. Global overfetch without a coverage proof loses eligible neighbors. Exact scalar account
  scans preserve correctness but incur many point lookups. External/vector sidecars violate the
  single-file native design; approximation changes ranking semantics.
- **Consequences:** Duplicate encrypted vectors increase disk size, write work and migration
  time. Schema/backfill failure rolls back atomically; orphans do not enter the derived index.
  No vector shadow-table access, new network behavior, FFI or dependency is required. Cache_size
  is a soft per-connection target; process memory and migration growth must be measured separately.
  Old binaries must not write a v5 database because they cannot maintain the derived index.

### [2026-10-31] ADR-007: Pre-Release Schema Consolidation & Persisted Triage (Supersedes ADR-006)

- **Context:** Prior to the initial `v0.1.0` release, schema migrations v1–v5 accumulated evolutionary
  intermediate states: v2 FTS, v3 initial vec0, v4 desktop outbox/connections, and v5 derived
  `vec_messages_by_account`. ADR-006 introduced vector index duplication solely to preserve an unreleased
  v4 development schema. Furthermore, triage categories and badges were recalculated on demand for
  every message detail request, and visible row badges required fetching full message bodies over IPC.
- **Decision:**
  1. *Schema Governance Policy:* P1 schema immutability applies strictly to released schemas. Prior to
     the `v0.1.0` release tag, development schemas may be consolidated because local development databases
     can be cleanly recreated from IMAP sync. From release tag `v0.1.0` onward, all migrations MUST be
     strictly append-only (`v2`, `v3`, etc.).
  2. *Canonical v1 Schema Consolidation:* Collapse migrations v1–v5 into a single canonical v1 schema.
     Replace the duplicate `vec_messages` / `vec_messages_by_account` tables with a single account-partitioned
     `vec_messages` virtual table (`account_id TEXT PARTITION KEY`), dropping the derived table, reconciliation
     triggers, and migration backfill logic entirely.
  3. *Persisted Triage Ingestion & Additive DTO:* Persist both `category_badge TEXT` and `triage_badges TEXT`
     at message ingestion time in SQLite. Hydrate `triage_badges` into `MessageRowSummary` and expose them
     additively on `MessageRowDto`, allowing message list views to render badges immediately without
     expensive full-body IPC round-trips or on-demand regex re-analysis.
- **Alternatives Considered:**
  - *Keep append-only v1–v5 into v0.1.0 release:* Rejected because shipping duplicate vector indices,
    backfill overhead, and four synchronization triggers permanently locks avoidable storage and CPU waste
    into the public schema foundation.
  - *Keep on-demand triage re-calculation:* Rejected because parsing regex/heuristics on every message detail
    and fetching bodies over IPC for list badges introduces unnecessary IPC serialization and CPU overhead.
- **Consequences:** All database tables and indices are created in a single migration v1 transaction. Storage
  footprint is reduced by eliminating vector table duplication. Development databases must be recreated from
  IMAP sync. Verified 100k global and account-filtered vec0 KNN searches against the exact reference oracle.

### [2026-10-31] ADR-008: Concurrency Architecture & Empirical Latency Target Redefinition

- **Context:**
  1. *Physical Limits of Global Exact Vector Search:* Scanning 100,000 `int8[384]` embeddings on CPU requires streaming 38.4 MB of raw vector payloads through memory and SQLCipher AES-256 page decryption. On a single modern core, this linear exact scan physically requires ~42–46ms warm. A blanket `<15ms` target across all query modalities (lexical, global semantic, hybrid) was set before 100k native measurements and is physically infeasible for global exact scans without indexing or lossy approximations.
  2. *Correction of Concurrency Invariant:* The [90.4] rationale claimed that connection pooling was unsafe for SQLite ACID semantics. That assertion was incorrect. In SQLite WAL mode (`PRAGMA journal_mode = WAL;`), multiple read-only connections are fully ACID-compliant, providing concurrent thread-safe snapshot isolation without write contention or read-lock blocking.
  3. *Hybrid Latency Bottleneck:* In hybrid search, running lexical (FTS5 BM25 ~32ms) and semantic (`vec_messages` ~46ms) candidate collection sequentially on a single connection resulted in ~79ms p50 latency.
- **Decision:**
  1. *Affirm WAL Concurrency Model & Dual-Connection Parallel Hybrid Search:* Affirm that concurrent read connections in SQLite WAL mode are completely safe, isolated, and ACID-compliant. Enable an optional secondary read connection in `SqliteMailStore` for file-backed databases, executing lexical and semantic hybrid candidate collection concurrently via `execute_hybrid_search_parallel` using `std::thread::scope`. When a secondary connection is unavailable (e.g. in-memory hermetic tests) or locked, gracefully fall back to sequential execution.
  2. *Redefined Latency Targets for v0.1.0 (p95 warm @ 100,000 messages on reference hardware):*
     - Filtered semantic search (25k partition): $\le 15$ ms (observed ~12.2 ms p50 / 12.9 ms p95).
     - Lexical search (common 99k matches): $\le 40$ ms (observed ~31.9 ms p50 / 37.3 ms p95).
     - Global semantic search (100k exact scan): $\le 50$ ms (observed ~42.7 ms p50 / 47.8 ms p95).
     - Hybrid search (lexical + semantic + RRF + hydration): $\le 60$ ms concurrent (observed ~52.4 ms p50) / $\le 95$ ms sequential.
     - UI First Page (including IPC serialization + WebKit virtual DOM): $\le 100$ ms.
  3. *Page Cache Memory Overhead Budget:* Empirically establish that each active SQLCipher connection configured with a 64 MiB soft page cache target (`cache_size = -65536`) consumes at most ~25.5 MiB of net additional resident memory (VmRSS) under 100k vector workloads, maintaining overall process RSS under ~105 MiB.
  4. *v0.1 Strict Scope Boundaries:* Strictly prohibit approximate nearest neighbor (ANN) graph indexing (HNSW, IVF, etc.) in v0.1 to avoid index corruption risks, memory bloat, and non-deterministic recalls in local desktop installations.
  5. *v0.2 Roadmap Evaluation:* Evaluate 1-bit scalar quantization (binary vectors with bitwise POPCNT Hamming distance for 48-byte candidate filtering) followed by int8 dot-product reranking of the top 200 candidates to achieve sub-15ms global semantic search in v0.2.
     **Evaluation outcome (2026-11-02):** ADR-011 declines fixed-budget BQ under the existing exact cosine/oracle contract; latency targets and concurrency decisions remain unchanged.
- **Alternatives Considered:**
  - *Keep blanket <15ms requirement for global exact search:* Rejected as physically impossible on CPU for exact linear 100k scans without approximations or caching.
  - *Adopt ANN vector graphs (e.g. HNSW) for v0.1:* Rejected due to complexity, memory bloat, write amplification during background sync, and risk of index corruption in local-first desktop apps.
  - *Keep hybrid search strictly sequential on 1 connection:* Rejected because concurrent execution reduces hybrid wall-clock latency by 33.7% (~26.6ms savings) for only ~25.5 MiB of additional memory.
- **Consequences:** Provides honest, falsifiable, and empirically grounded performance contracts across all search modalities. File-backed hybrid search runs in ~52ms p50 instead of ~79ms. Single-connection and in-memory stores safely fall back to sequential execution. All trait boundaries and DTOs remain strictly preserved.

### [2026-10-31] ADR-009: In-House Asynchronous IMAP Client & Protocol Architecture

- **Context:**
  1. *Safety Invariant & Zero-Unsafe Boundary:* `crates/mail-engine` mandates `#![forbid(unsafe_code)]` under P2 memory safety rules. Legacy upstream crates such as `async-imap` and their historical transitive dependencies incorporate unsafe blocks, unmaintained futures abstractions, and rigid connection structures.
  2. *Lazy Attachment Invariant & RFC 3501 BODYSTRUCTURE:* `async-imap` is oriented around monolithic macro commands (e.g. `FETCH RFC822`), which downloads full MIME envelopes including large binary attachments during bulk synchronization. This directly violates the Lazy Attachment invariant, saturating RAM and SQLCipher disk storage when synchronizing 100k+ messages.
  3. *SASL XOAUTH2 Challenge-Abort & Token Refresh:* Production IMAP servers (Google Gmail, Microsoft 365) return `+ <base64 error>` challenge lines upon token expiration and wait for client continuation. Per RFC 6749 and provider specifications, the client must transmit `\r\n` CRLF to abort the challenge before reading the tagged `NO`. Generic IMAP libraries hang or panic on unhandled continuation lines.
  4. *Localhost Proton Mail Bridge TLS Support:* Proton Mail Bridge operates on `127.0.0.1:1143` with self-signed local TLS certificates. Off-the-shelf IMAP connectors either fail to connect due to strict CA validation or introduce insecure blanket certificate verification bypasses.
  5. *Incremental Sync Watermarks & IDLE Integration:* Background synchronization requires tight coupling with `UIDVALIDITY`, `UIDNEXT`, `CONDSTORE`/`HIGHESTMODSEQ`, batched window commits (300 UIDs/batch), and non-blocking `IDLE` keep-alive loops communicating via pure Rust `SyncEventSink` channels.
- **Decision:**
  Implement a purpose-built, lightweight, asynchronous IMAP protocol client natively in `crates/mail-engine` using `tokio::net::TcpStream`, `tokio-rustls` (supporting TLS 1.3 and TLS 1.2), and Stalwart Labs' zero-copy `mail-parser`:
  1. *Zero Unsafe Code:* Entire protocol implementation in `crates/mail-engine` adheres strictly to `#![forbid(unsafe_code)]`.
  2. *Two-Phase Lazy Sync:* Implemented zero-unsafe RFC 3501 `BODYSTRUCTURE` parser (`bodystructure.rs`). Phase 1 requests `UID FETCH ... (BODYSTRUCTURE BODY.PEEK[HEADER])` to index attachment metadata without downloading payload bytes. Phase 2 requests only candidate leaf text sections (`BODY.PEEK[1]` or `BODY.PEEK[<sec>.MIME] BODY.PEEK[<sec>]` $\le 2$ MiB). Large binary attachments remain on the server and are retrieved on demand via `BODY.PEEK[<part_id>]`.
  3. *SASL XOAUTH2 Challenge Abort & Refresh:* `protocol.rs` detects `+ ` continuation lines on `AUTHENTICATE` commands, writes `\r\n` CRLF to complete the challenge abort, surfaces `EngineError::AuthenticationFailed`, and invokes `TokenRefresher` to refresh the access token via HTTPS, updates `vault` atomically, and reconnects transparently.
  4. *Scoped Loopback TLS Pinning:* Allows self-signed TLS certificates strictly when the destination host is loopback (`127.0.0.1` or `::1`), preserving strict CA validation and TLS 1.3/1.2 security for all remote endpoints.
  5. *Batched Resilient Commits:* Enforces batched initial sync (`DEFAULT_SYNC_UID_BATCH_SIZE = 300`) committing messages and watermarks sequentially to SQLCipher so interrupted sync resumes without data loss or re-downloading.
- **Alternatives Considered:**
  - *Continue using `async-imap`:* Rejected due to full `RFC822` memory bloat, lack of zero-copy `BODYSTRUCTURE` part extraction, unhandled SASL XOAUTH2 continuation aborts, and upstream dependency stagnation.
  - *Adopt `imap-next`:* Evaluated as an emerging library, but rejected for v0.1.0 due to alpha API volatility, lack of mature IDLE and SASL XOAUTH2 challenge-abort integration, and complex framing ergonomics.
- **Consequences:** Eliminates external `async-imap` crate dependency. Guarantees `#![forbid(unsafe_code)]` throughout `crates/mail-engine`. Enforces the Lazy Attachment invariant with zero attachment download during initial sync, handles OAuth2 token refresh gracefully, and provides native loopback Proton Mail Bridge support.

### [2026-11-01] ADR-010: Account Mailboxes Navigation, Append-Only Migration v2, and Deduplication Architecture

- **Context:**
  1. *Global All Mail vs Per-Account Scoped Views:* Currently, `InboxController` and storage assume a global unified view across all accounts, with `SearchQueryDto` setting `account_filter: None`. Supporting per-account navigation (Inbox, Starred, Snoozed, Archive, Trash, Spam, custom folders) requires additive, type-safe mailbox scoping without breaking backwards compatibility for global search or pagination.
  2. *Folder Hierarchy & Catalog Synchronization:* While IMAP folders are discovered via `LIST "" *`, the engine previously synchronized only `INBOX`. Supporting hierarchical mailboxes requires persisting folder trees, delimiters, special-use roles (`\Inbox`, `\Sent`, `\Drafts`, `\Archive`, `\Trash`, `\Junk`), and selectable flags (`\Noselect`), scheduling bounded background sync per folder without spawning excessive idle connections.
  3. *Starred Flags & Local Snooze Deadlines:* Messages need bidirectional synchronization of IMAP `\Flagged` (Starred) and local client-side UTC deadlines (Snooze). Snoozed messages must temporarily disappear from `Inbox` until the UTC deadline passes, without moving the message on the remote IMAP server. Archive or Trash operations must clear active snoozes immediately.
  4. *Multi-Folder Membership & Same-Account Gmail Deduplication:* In Gmail, a message exists in `[Gmail]/All Mail` and carries multiple labels (e.g. `INBOX`, `Starred`). Storing duplicate copies of message bodies, HTML, and vector embeddings bloats SQLCipher and distorts global search. However, per-folder UIDs must be preserved for IMAP mutations (`STORE`, `MOVE`, `BODY.PEEK`). Deduplication must use immutable provider identifiers (`X-GM-MSGID`) or message header hash strictly within the same account—never across distinct accounts.
  5. *Local Account Photos & Privacy Invariant:* Displaying avatars in the sidebar must strictly respect zero-telemetry rules. Avatars must be imported from local raster images (JPEG/PNG/WebP, max 2MB, max 1024x1024), stripped of EXIF metadata, re-encoded to fixed thumbnails (128x128), and stored in encrypted SQLCipher. Gravatar, OAuth profile fetches, or external HTTP lookups are strictly forbidden.
- **Decision:**
  1. *Additive Schema Migration v2:* Maintain strict append-only migration governance (ADR-007) by introducing Migration v2:
     - `folders`: persists `(account_id, folder_name)` as primary key, `display_name`, `delimiter`, `role` (`inbox`, `starred`, `sent`, `drafts`, `archive`, `trash`, `spam`, `other`), `is_selectable`, `total_messages`, `unread_messages`, and `updated_at_epoch_ms`.
     - `message_folder_memberships`: maps `(account_id, folder, uid)` to `message_id`, preserving per-folder UID mapping while referencing a single canonical message record.
     - Column additions to `messages`: `is_starred INTEGER NOT NULL DEFAULT 0`, `snooze_until_epoch_ms INTEGER`, and `provider_message_id TEXT`.
     - `account_avatars`: stores normalized encrypted thumbnail blobs, MIME type, dimensions, and update timestamp keyed by `account_id`.
  2. *Mailbox Scoping & Search Contract:* Extend `SearchQueryDto` with optional `scope: Option<MailboxScopeDto>` (defaulting to `None` for global All Mail). `MailboxScopeDto` represents either `GlobalAllMail` or `AccountView { account_id, view: MailboxViewDto }` (`Inbox`, `Starred`, `Snoozed`, `Folder(String)`). Effective folder membership evaluates `COALESCE(override.folder, membership.folder)` against active snooze criteria (`snooze_until_epoch_ms IS NULL OR snooze_until_epoch_ms <= ?`).
  3. *Same-Account Deduplication Engine:* Ingesting a message checks `(account_id, provider_message_id)`. If already present, skips body/embedding storage and inserts only into `message_folder_memberships`. Fallback for non-Gmail IMAP uses `(account_id, message_id_header, received_at_epoch_ms)`. Cross-account deduplication is strictly prohibited.
  4. *Two-Way Flag & Snooze Mutation Contract:* Extend `TriageActionDto` with `Star`, `Unstar`, `Snooze(i64)`, and `Unsnooze`. Star mutations queue an IMAP `+FLAGS (\Flagged)` update in `pending_mutations`. Moving a message to Archive or Trash automatically clears any pending `snooze_until_epoch_ms`.
  5. *Zero-Network Local Avatars:* Avatars are processed via native image decoding, resizing, and stripping of EXIF data before encryption in `account_avatars`. Accounts without custom images use deterministic initials and accent colors.
- **Alternatives Considered:**
  - *Full message re-download per folder (No dedup):* Rejected due to disk and bandwidth multiplication (Gmail users often have 3x duplicate message counts across labels).
  - *Deduplicate across accounts with matching Message-ID:* Rejected as a catastrophic privacy and data isolation violation; each account must remain completely segregated.
  - *Remote server snooze extensions (IMAP METADATA / custom flags):* Rejected due to lack of provider standardization; local UTC deadlines provide 100% reliable provider-agnostic behavior.
  - *Gravatar / External avatar lookups:* Rejected under Rule P0 (Zero Telemetry & External Network Isolation).
- **Consequences:** Unlocks expandable per-account mailbox navigation in the desktop sidebar while preserving sub-15ms search and database integrity. Adds Migration v2 idempotently. Preserves complete backwards compatibility for existing IPC callers and All Mail queries.

### [2026-11-02] ADR-011: Retain Exact Cosine Search After Binary Candidate Evaluation

- **Context:** [01.1] evaluates ADR-008's proposed 48-byte sign-code candidate scan and
  exact int8 reranking over the existing 100k-message SQLCipher mailbox fixture.
  The current contract requires complete ordered-page oracle equivalence, not only
  plausible recall. Scoped search also accepts zero-norm vectors (NULL-first cosine
  ordering), missing embeddings, orphan vectors and arbitrarily large ties (k-021).
- **Decision:** Retain production exact int8 cosine ranking. Reject fixed Hamming
  budgets (including the proposed 200) as an exact replacement. Magnitude information
  is absent from sign codes: an omitted binary candidate can be the unique exact
  nearest neighbor. Reranking retained candidates does not prove coverage. Widening
  cannot fix every case: sqlite-vec 0.1.9 caps k at 4,096 and Hamming tie groups can be
  larger. Deep requests beyond this cap require existing exact fallback. Keep the
  experiment in an offline example with a disposable index in the encrypted file;
  no released schema, IPC, trait, production dispatcher or latency target changes.
- **Evidence:** [Reproduction, latency/recall tables and oracle probes](../performance/binary-quantization.md),
  [raw samples](../performance/binary-quantization.csv). Seed 4242 global BQ-200 p95 is
  21.226ms with 56/200 oracle hits; BQ-4096 is 156.361ms with 166/200. All 270 global
  warm BQ samples across three query seeds fail ordered equality. Full-page equality and page-set
  recall are reported separately; empty pages and exact fallback do not count as
  successful binary acceleration. Fixture vectors do not establish learned semantic recall.
- **Alternatives Considered:**
  - *Adopt 200/800/4,096 candidates based on observed latency or full output pages:*
    rejected because these cannot guarantee ordered oracle equivalence.
  - *Silently use raw int8 dot products or discard zero norms:* rejected because
    unnormalized cosine and accepted input ordering would change.
  - *Run a full exact scan to verify every binary result:* preserves coverage but
    retains exact scan cost; no sub-15ms certified path is established here.
  - *Separate approximate product mode:* requires explicit approval of new semantics
    and representative learned-embedding recall evaluation; outside this investigation.
- **Consequences:** ADR-008's performance targets and concurrency decisions remain.
  Its v0.2 binary stage is evaluated and declined under current exactness requirements.
  [04.1.7] scoped/native/frame-cadence deviations remain unresolved. Future work should
  investigate exact contiguous eligible scans or admissible residual bounds, including
  NULL norms, lifecycle correctness, unlimited ties, encrypted storage and memory cost.
  Any persisted index requires a separately reviewed append-only migration. No ANN graph,
  dependency, unsafe code, user-data access or external network behavior is introduced.

### [2026-11-02] ADR-012: Shared Local Rules and Durable Ingestion Eligibility

- **Context:** [03.5] needs a shared evaluator for automatic filters and future alerts,
  with identical mutation-free simulation and protection against initial-history floods,
  duplicate folder memberships, interrupted imports and feedback from IMAP mutations.
- **Decision:** Keep typed rules and an immutable-facts evaluator in `triage-ai`, encrypted
  definitions/queue orchestration in `storage`, and existing observer/outbox adapters in
  desktop. Preserve existing traits and IPC; add commands and append-only migration v3.
  Rules use all/any/except literal predicates, integer BRL cents and calendar dates.
  Order by position then stable ID; union tags/star/archive, stop after a matching stop
  rule, and issue flags before archive moves. Never execute scripts or download attachments.
- **Eligibility:** Persist each folder generation's first SELECT UIDNEXT frontier before
  ingestion. Missing UIDNEXT fails closed with the maximum frontier. First imports and
  UIDVALIDITY rebuilds suppress history across all batches/restarts. Existing installations
  establish a baseline on their first post-upgrade sync. New canonical messages above
  the frontier atomically acquire a job and a snapshot of rule revisions with ingestion.
  Editing, disabling, deleting or reordering a scheduled revision invalidates it; unchanged
  scheduled enabled rules remain eligible. Saving never creates jobs for stored mail.
  Deleted rule ID tombstones prevent revision reuse and stale-write ABA races.
- **Recovery:** Job completion, local mutations and existing IMAP outbox entries commit
  together. Account-bound SHA-256 identity tombstones outlive message-cache deletion
  (including MOVE without COPYUID) to suppress feedback loops. Provider ID is preferred,
  then Message-ID plus received timestamp; absent those, hash original headers/subject/body.
  This conservative fallback can suppress identical headerless copies. Tombstones remain
  until account removal; no arbitrary time expiry silently permits replay. Simulation
  shares facts/evaluation but never enqueues, marks, mutates or delivers anything.
- **Alternatives:** A frontend-only evaluator cannot guarantee simulation equivalence;
  in-memory duplicate markers fail on restart; post-commit-only eligibility can lose mail
  between checkpoint commit and observer execution. Changing existing traits or introducing
  network alert adapters is unnecessary for this task and remains separately scoped.
- **Consequences:** Rule processing requires an active runner. Up to 200 rules, 64 flat
  conditions/rule, 16 actions/rule and 100 selected simulation messages bound work/IPC;
  these are definition/work limits, with no account cap. Existing schema v1/v2 and search
  behavior remain unchanged. No notifications, external delivery or history replay is added.

### [2026-11-02] ADR-013: Consent-Bound Durable Alert Deliveries

- **Context:** [03.6] extends the shared local rule planner with optional notifications
  and generic webhooks, without initial-history floods, secret persistence outside vault
  or a GUI dependency in core. Arbitrary HTTP receivers cannot provide exactly-once delivery.
- **Decision:** Add `RuleAction::Alert(destination_id)` to version-1 rules and append-only
  migration v4. Freeze enabled destination revisions alongside rule revisions at ingestion;
  finish rule jobs, local actions, IMAP outbox entries and alert enqueue in one transaction.
  A destination edit/disable cancels old pending jobs; enabling never replays old events.
  Account-bound event/destination deduplication markers outlive pruned execution history.
  Existing [03.5] baselines and conservative identity limits still apply.
- **Secrets/consent:** Stage immutable vault-only URL/bearer revisions using SQL intent
  records before vault writes. Commit only opaque references and public origins to SQL;
  clean unreferenced staged secrets after interruption. Explicit enablement confirms the
  displayed public origin and exported fields. Test/replay confirmation carries the
  destination revision; stale confirmation fails. Replay selects messages, creates fresh
  event IDs and never executes mutation actions. Simulation remains free of deliveries.
- **Network:** The user approved a narrow P0 exception: public DNS HTTPS port 443 only,
  verified TLS 1.2+, exact validated DNS snapshots, no redirect/proxy inheritance, and a
  ten-second dispatch deadline. Private/mixed/reserved addresses, IP literals, URL userinfo
  and fragments are rejected. No live development deliveries are authorized by this change.
- **Delivery:** SQL leases survive restart with stable event IDs/Idempotency-Key. Default
  rate is one start/ten seconds/destination; desktop dispatches one job per worker tick,
  globally serially. Retry transport failures, 408/429 and 5xx with bounded jitter/backoff
  and Retry-After; cap at eight attempts and 24 hours. All other non-2xx outcomes are terminal.
  Expired/canceled/completed payloads are cleared. Retain terminal history at most 30 days
  and 1,000 rows/destination; history DTOs expose only IDs, statuses, counts and timestamps.
- **Disclosure:** Default webhook payload is schema/event/time/truncation metadata. Optional
  fields are individually selected; bodies/excerpts may contain sensitive data. Structured
  extraction exports only boleto amounts/dates, never raw OTP/Pix badges. Plain-text body
  export is capped at 6,000 UTF-8 bytes, excerpts at 512; metadata collections at 16 items.
  JSON remains <=64 KiB with explicit truncation. Missing bodies/attachments are never fetched.
  Local notices are generic, with no mail fields. Linux uses the session notification service;
  absent/denied service and other operating systems return unavailable. Desktop-shell acceptance
  is not proof of visibility, and lock-screen display is controlled by the operating system.
- **Consequences:** Timeouts/crashes can duplicate accepted requests, so receivers should
  deduplicate by event ID. Disable waits for an already-started bounded attempt; no recall
  promise. An active unlocked runner is required. Core contracts permit a separate headless
  composition but no CLI runner, Telegram/Discord adapter, digest or reminder is delivered.
  Message/account removal cancels related pending work; cache removal after MOVE without
  COPYUID can therefore suppress an undelivered alert. Settings explain export and lifecycle
  limits. Existing IPC, trait signatures, migrations v1-v3 and search contracts are preserved.

### [2026-11-02] ADR-014: Telegram and Discord on the Shared Alert Queue

- **Context:** [03.7] adds guided provider destinations without a second rule evaluator,
  delivery queue, credential store or automatic setup network traffic.
- **Decision:** Add Telegram/Discord destination enum variants to existing JSON metadata.
  No migration or trait/command signature changes are needed. Existing webhook input
  remains compatible; its input-only deserializer also accepts typed `{ token, chat_id }`.
  Telegram normalizes into the existing vault-only URL/bearer envelope: fixed Bot API
  sendMessage URL containing the token, and numeric chat ID in the bearer slot. That
  slot never becomes HTTP authorization for Telegram. Discord uses the existing URL
  envelope with no bearer. Kind changes cannot preserve another kind's saved material.
- **Consent/network:** Existing destination/field/revision consent and secret-intent recovery
  apply. Telegram has a fixed api.telegram.org sendMessage route; Discord accepts canonical
  discord.com/api/webhooks paths only, with no query/thread overrides. Numeric chat IDs
  are required. No discovery/getUpdates, automatic chat migration, remote revocation or
  live development request is introduced. The user approved the narrow P0 extension.
  Public-only pinned DNS, verified TLS 1.2+, port 443, no redirects/proxies and ten-second
  total dispatch deadlines remain mandatory. DNS is bounded to three seconds/32 addresses.
- **Formatting:** Headless pure helpers serialize the already consent-filtered JSON into
  one provider text message. Conservative UTF-16 ceilings are 3,500/1,800 respectively;
  truncation is visible, with no splitting or attachments. Telegram has no parse_mode
  and disables link previews; Discord sets empty allowed_mentions.parse and SUPPRESS_EMBEDS.
- **Outcomes:** Provider response buffers are transient, capped at 64 KiB and zeroized.
  Parse only acceptance/error codes and retry delays; descriptions, chat/message objects
  and response text never reach logs, IPC or history. Telegram requires ok:true for 2xx
  acceptance. Discord's default webhook 2xx/204 is receiver acceptance only. Existing
  HTTP retry classes, attempt/age caps and bounded delays apply. A retry persists a
  conservative shared cooldown for all destinations of that provider using existing
  next_start metadata, without storing secret fingerprints. Other provider kinds remain
  independent. Global serialization remains in the existing worker.
- **Consequences:** No provider idempotency or human-visibility guarantee; retry ambiguity
  can duplicate messages. Disable/delete cannot recall accepted messages. Credential
  revocation remains manual in BotFather/Discord. Payloads are readable JSON text and
  provider truncation may cut JSON mid-field. Offline tests establish local behavior,
  not live receiver interoperability. Digests, quiet hours, reminders and CLI remain deferred.
