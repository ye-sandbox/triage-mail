# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log`. User requests during conversation
> take precedence — report discrepancies before acting.

---

## Active Task

### 📌 Task [01.2]: SQLite WAL + Full-DB SQLCipher Schema, Migrations, Configurable FTS5 & sqlite-vec KNN (`crates/storage`)

- **Description:** Implement the encrypted SQLite repository with SQLCipher, WAL mode, pragmas (`temp_store = MEMORY`), dynamic schema migrations, configurable lexical FTS5 (default `unicode61 remove_diacritics 2`), `sqlite-vec` (`int8` quantized KNN vector index with pre-filtering), and persistent IMAP folder `UIDVALIDITY`/`MODSEQ` sync state tracking.
- **Systems Involved:** `crates/storage`
- **Action Type:**
  - [ ] Read-only / Documentation
  - [x] Source code changes
- **Status:** RUNNING

### Acceptance Criteria
- [ ] Full-database SQLCipher encryption initialized with key material derived from `crates/vault`.
- [ ] SQLite WAL mode and foreign key constraints enabled with migrations for accounts, folders, messages, attachments, tags, and sync state.
- [ ] Configurable lexical `FTS5` virtual table with BM25 ranking (default `unicode61 remove_diacritics 2`, optional `trigram`).
- [ ] Isolated `#[allow(unsafe_code)]` block in `crates/storage` strictly for `sqlite-vec` C-extension registration (`sqlite3_auto_extension`) while retaining `#![deny(unsafe_code)]` for the crate.
- [ ] Unit tests verifying encrypted DB round-trip, FTS5 lexical search, vector cosine/KNN similarity, and migration execution.

---

## Completed Tasks Log

| Task | Title | Commit(s) | Date |
|---|---|---|---|
| [00.0] | Initial ADD greenfield governance setup (`AGENTS.md`, `NOTES.md`, `decisions.md`) | `00b19e9` | 2026-10-29 |
| [00.1] | Workspace Scaffold & Core Crate Boundaries (`Cargo.toml` + `apps/desktop`) | `8ccaea0` | 2026-10-29 |
| [01.1] | OS Keyring Integration & Argon2id/AES-GCM Fallback Vault Cipher (`crates/vault`) | `0259096` | 2026-10-29 |

---

## Backlog (Upcoming, in priority order)

- [ ] **[01.2]** SQLite WAL + Full-DB SQLCipher schema, migrations, configurable FTS5 index (default `unicode61 remove_diacritics 2`), `sqlite-vec` (`int8` quantized KNN + pre-filtering) & IMAP `UIDVALIDITY`/`MODSEQ` sync state — `crates/storage`
- [ ] **[02.1]** Zero-copy RFC 5322 / MIME parser (Stalwart `mail-parser`) with threading headers (`Message-ID`, `In-Reply-To`, `References`), `List-Unsubscribe-Post` (RFC 8058), & lazy attachment part metadata — `crates/mail-engine`
- [ ] **[02.2]** Agnostic `ImapConnector` with provider quirk normalization (`pimalaya/email-lib` patterns), transparent multi-auth (App Passwords, OAuth2 Public PKCE, user-supplied BYO OAuth `client_id`/`client_secret`, Proton Bridge) & incremental `UIDVALIDITY`/`CONDSTORE` sync — `crates/mail-engine`
- [ ] **[02.3]** Real-time IMAP `IDLE` manager, adaptive background polling worker & RFC 8058 One-Click Unsubscribe HTTP POST executor — `crates/mail-engine`
- [ ] **[03.1]** Tier 1 Deterministic Triage: Regex/heuristic extractors for 2FA/OTP, FEBRABAN Boletos (exact BRL amount & due date), Pix EMV/keys, NF-e metadata, Tracking codes, `List-Unsubscribe` (RFC 8058), and category classification — `crates/triage-ai`
- [ ] **[03.2]** Tier 2 Local CPU Embeddings (`fastembed-rs` / `ort`) background indexer with on-demand model cache in `$XDG_DATA_HOME/triage-mail/models` — `crates/triage-ai`
- [ ] **[03.3]** Tier 3 Optional LLM Summarizer client (Local Ollama `127.0.0.1:11434` & BYOK providers) — `crates/triage-ai`
- [ ] **[04.1]** Tauri v2 typed IPC commands & real-time sync `Channel` bridge — `apps/desktop/src-tauri`
- [ ] **[04.2]** Unified Inbox UI with 60fps Virtual Scrolling (`100k+` messages) & instant hybrid search bar (`<15ms`) — `apps/desktop`
- [ ] **[04.3]** Sandboxed Email Viewer (`DOMPurify` + `<iframe sandbox="">` + normalized base dark-mode CSS + remote pixel blocker) & on-demand `BODY.PEEK` attachment downloader — `apps/desktop`
- [ ] **[04.4]** Keyboard-driven Command Palette (Mark read/unread, Archive, Trash, Local Tags, One-Click Unsubscribe, IMAP flag sync) & Triage Badges UI — `apps/desktop`

---

## Release / Cycle Wrap-up (Not the next task)

Release/tag only with explicit human request. When triggered, the ID is `[99.1]`. Do not number feature, hygiene, or CI tasks as `99.x`. Do not calculate next task ID from this section.

---

## Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Post-triage webhook & automation consumers implementing `TriageEventHook`
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
