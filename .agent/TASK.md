# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log`. User requests during conversation
> take precedence — report discrepancies before acting.

---

## Active Task

### 📌 Task [02.3]: Real-time IMAP `IDLE` Manager, Adaptive Background Polling Worker & RFC 8058 One-Click Unsubscribe HTTP POST Executor (`crates/mail-engine`)

- **Description:** Implement the background `IDLE` manager with adaptive 14-minute heartbeats (`DONE` -> `IDLE`) to keep NAT connections alive, graceful fallback to adaptive polling when `IDLE` is unsupported or dropped by firewalls, and the asynchronous RFC 8058 One-Click Unsubscribe HTTP POST client (`List-Unsubscribe-Post: List-Unsubscribe=One-Click`).
- **Systems Involved:** `crates/mail-engine`
- **Action Type:**
  - [ ] Read-only / Documentation
  - [x] Source code changes
- **Status:** READY FOR PLANNING
  *(Workflow: `READY FOR PLANNING` → `PLANNING` on presenting plan → approval → `RUNNING`)*

### Acceptance Criteria
- [ ] Implement `IdleManager` running in background `tokio` tasks with 14-minute renewal window and `SyncEventSink` notification loop.
- [ ] Implement adaptive exponential backoff polling for servers lacking `IDLE` support or on persistent disconnects.
- [ ] Implement RFC 8058 One-Click Unsubscribe HTTP POST executor (`reqwest` with `List-Unsubscribe=One-Click` form body) with strict domain validation.
- [ ] Unit and mock tests verifying `IDLE` keep-alive transitions, adaptive polling intervals, and RFC 8058 HTTP POST execution using RFC 2606 synthetic URIs.

---

## Completed Tasks Log

| Task | Title | Commit(s) | Date |
|---|---|---|---|
| [00.0] | Initial ADD greenfield governance setup (`AGENTS.md`, `NOTES.md`, `decisions.md`) | `00b19e9` | 2026-10-29 |
| [00.1] | Workspace Scaffold & Core Crate Boundaries (`Cargo.toml` + `apps/desktop`) | `8ccaea0` | 2026-10-29 |
| [01.1] | OS Keyring Integration & Argon2id/AES-GCM Fallback Vault Cipher (`crates/vault`) | `0259096` | 2026-10-29 |
| [01.2] | SQLite WAL + Full-DB SQLCipher Schema, Migrations, Configurable FTS5 & sqlite-vec KNN (`crates/storage`) | `ecb7cad` | 2026-10-30 |
| [02.1] | Zero-Copy RFC 5322 / MIME Parser with Threading Headers, RFC 8058 & Lazy Attachment Metadata (`crates/mail-engine`) | `c808414` | 2026-10-30 |
| [02.2] | Agnostic `ImapConnector` with Provider Quirk Normalization (`pimalaya`), Multi-Auth & Incremental `UIDVALIDITY`/`CONDSTORE` Sync (`crates/mail-engine`) | `8290883` | 2026-10-30 |

---

## Backlog (Upcoming, in priority order)

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
