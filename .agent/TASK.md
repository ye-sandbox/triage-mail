# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log`. User requests during conversation
> take precedence — report discrepancies before acting.

---

## Active Task

### 📌 Task [02.1]: Zero-Copy RFC 5322 / MIME Parser with Threading Headers, RFC 8058 & Lazy Attachment Part Metadata (`crates/mail-engine`)

- **Description:** Implement zero-copy email header and body parsing using Stalwart Labs' `mail-parser`. Extract threading headers (`Message-ID`, `In-Reply-To`, `References`), RFC 8058 One-Click unsubscribe metadata (`List-Unsubscribe`, `List-Unsubscribe-Post`), sender/recipient envelopes, plain-text/HTML bodies, and lazy attachment part metadata (MIME type, filename, size, `BODY.PEEK` section ID) without eagerly downloading binary payloads into memory.
- **Systems Involved:** `crates/mail-engine`
- **Action Type:**
  - [ ] Read-only / Documentation
  - [x] Source code changes
- **Status:** READY FOR PLANNING
  *(Workflow: `READY FOR PLANNING` → `PLANNING` on presenting plan → approval → `RUNNING`)*

### Acceptance Criteria
- [ ] Stalwart `mail-parser` integrated into `crates/mail-engine` with `#![forbid(unsafe_code)]`.
- [ ] Zero-copy extraction of threading headers: RFC 5322 `Message-ID`, `In-Reply-To`, and multi-ID `References`.
- [ ] Deterministic detection and extraction of RFC 8058 `List-Unsubscribe` URLs (mailto / https) and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`.
- [ ] Lazy attachment part metadata extraction (`part_id`, `filename`, `content_type`, `size_bytes`, `content_id`, `is_inline`) without downloading binary bodies.
- [ ] Comprehensive unit tests using synthetic RFC 2606 fixtures (`user@example.com`, `billing@example.org`) verifying header parsing, threading tree reconstruction, and multipart MIME extraction.

---

## Completed Tasks Log

| Task | Title | Commit(s) | Date |
|---|---|---|---|
| [00.0] | Initial ADD greenfield governance setup (`AGENTS.md`, `NOTES.md`, `decisions.md`) | `00b19e9` | 2026-10-29 |
| [00.1] | Workspace Scaffold & Core Crate Boundaries (`Cargo.toml` + `apps/desktop`) | `8ccaea0` | 2026-10-29 |
| [01.1] | OS Keyring Integration & Argon2id/AES-GCM Fallback Vault Cipher (`crates/vault`) | `0259096` | 2026-10-29 |
| [01.2] | SQLite WAL + Full-DB SQLCipher Schema, Migrations, Configurable FTS5 & sqlite-vec KNN (`crates/storage`) | `ecb7cad` | 2026-10-30 |

---

## Backlog (Upcoming, in priority order)

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
