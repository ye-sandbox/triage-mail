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
