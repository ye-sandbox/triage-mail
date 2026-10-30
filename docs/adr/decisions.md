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
