# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log`. User requests during conversation
> take precedence — report discrepancies before acting.

---

## Active Task

### 📌 Task [04.4]: Keyboard-driven Command Palette & Triage Badges UI (`apps/desktop`)

- **Description:** Implement the keyboard-driven Command Palette (Mark read/unread, Archive, Trash, Local Tags, RFC 8058 One-Click Unsubscribe, two-way IMAP flag sync via `cmd_triage_action`) and render extracted Tier 1 Triage Badges (`2FA/OTP`, `FEBRABAN Boleto`, `Pix`, `NF-e`, `Tracking`, `UnsubscribeAvailable`) in the inbox list and message reader.
- **Systems Involved:** `apps/desktop`, existing `apps/desktop/src-tauri` IPC contracts (`cmd_triage_action`, `TriageBadge`)
- **Action Type:**
  - [ ] Read-only / Documentation
  - [x] Source code changes
- **Status:** DONE
- **Next Task:** None queued. Optimization or hardening work awaits a user request.

### Acceptance Criteria
- [x] Keyboard-accessible Command Palette (`⌘K` / `Ctrl+K` and shortcut triggers) for Mark Read/Unread, Archive, Trash, Local Tags, and RFC 8058 One-Click Unsubscribe using `cmd_triage_action`.
- [x] Optimistic UI updates and resilient error/sync reconciliation with `InboxController` and reader states.
- [x] Accessible Triage Badges UI for OTP codes, FEBRABAN Boletos (BRL amount & due date), Pix payloads, NF-e access keys, tracking codes, and one-click unsubscribe availability.
- [x] Strictly enforce Svelte 5 Runes (`$props`, `$state`, `$derived`, `$effect`), Apple HIG restraint, reduced-motion compliance, and full repository Definition of Done.

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
| [02.3] | Real-time IMAP `IDLE` Manager, Adaptive Background Polling Worker & RFC 8058 One-Click Unsubscribe HTTP POST Executor (`crates/mail-engine`) | `f828829` | 2026-10-30 |
| [03.1] | Tier 1 Deterministic Triage: Regex/Heuristic Extractors for 2FA/OTP, FEBRABAN Boletos, Pix, NF-e, Tracking & Classification (`crates/triage-ai`) | `0644f9e` | 2026-10-30 |
| [03.2] | Tier 2 Local CPU Embeddings (`fastembed-rs` / `ort`) background indexer with on-demand model cache in `$XDG_DATA_HOME/triage-mail/models` (`crates/triage-ai`) | `6ae5e3d` | 2026-10-30 |
| [03.3] | Tier 3 Optional LLM Summarizer client (Local Ollama `127.0.0.1:11434` & BYOK providers) (`crates/triage-ai`) | `dee6a05` | 2026-10-30 |
| [04.1] | Tauri v2 Typed IPC, Secret-Isolated Account Management, Durable Triage Outbox & Real-time Sync Channel Bridge | `ca79844` | 2026-10-30 |
| [04.2] | Apple HIG-Informed Unified Inbox, Virtualized Message List, Typed Search & Sync Refresh | `16df93b` | 2026-10-30 |
| [04.2.1] | Accessible Apple-Style Inbox Motion | `6820c4b` | 2026-10-30 |
| [04.3] | Sandboxed Email Viewer & On-Demand Attachment Downloader (`apps/desktop`) | `8164ca2`, `2142c9e` | 2026-10-30 |
| [04.4] | Keyboard-driven Command Palette & Triage Badges UI (`apps/desktop`) | `78016a4` | 2026-10-30 |


### Validation ([04.1])
- Rust workspace check, tests (141 passed), formatting and Clippy (`-D warnings`): passed.
- Frontend check (zero errors/warnings), unit tests (2 passed) and SPA build: passed.
- Svelte runes invariant, tracked-file secret/PII scan and `git diff --check`: passed.
- Native IPC mock invocation, typed channel, vault rollback/redaction, encrypted outbox restart,
  transactional ingestion, MOVE identity rebasing and lazy BODY.PEEK regression tests: passed.
- Linux build prerequisites were extracted into a temporary user-owned sysroot; no system-wide
  installation or machine-specific repository configuration was added. Existing socket-based
  tests ran with loopback binding permitted. No live email providers were contacted.

### Validation ([04.2])
- Rust workspace check, tests (141 passed), formatting and Clippy (`-D warnings`): passed.
- Frontend check (zero errors/warnings), unit/component tests (9 passed), Prettier and SPA build: passed.
- Svelte runes invariant, tracked/new-file secret/PII scan and `git diff --check`: passed.
- Tests cover stale search responses, wire-mode forwarding, debouncing, pagination overlap,
  deferred sync refresh, selection retention, fixed error redaction, bounded DOM and scroll anchoring.
- Browser QA exercised the actual adaptive inbox shell with 100,000 synthetic message summaries.
  A temporary fixture advanced scrollTop by 40,000px per animation frame for 180 frames.
  At a reported 1200×750 CSS viewport, 179 intervals measured median 16.7ms and p95 16.8ms
  (approximately 60fps), with at most 18 mounted message rows. This is browser-fixture evidence,
  not a native WebKit or all-hardware guarantee. The temporary fixture route was removed.
- Desktop selection and compact list/detail/back navigation were verified in the browser.
  The production route handles a non-Tauri browser without issuing native IPC calls.
- Existing storage tests retain the `<15ms` hybrid-search target; no IPC/schema/core changes.
- Loopback-only Rust mocks ran with socket binding permitted. Dependencies were installed from
  cached packages; no live mail provider, model download or AI endpoint was contacted.
- Recorded the Vite peer-snapshot/type mismatch and browser-resolution workaround in `k-007`.
- No CLI flags, environment variables or documented validation commands changed.

### Validation ([04.2.1])
- User approved the motion implementation plan on 2026-10-30.
- Rust workspace check, tests (141 passed), formatting and Clippy (`-D warnings`): passed.
- Frontend check (zero errors/warnings), tests (11 passed), Prettier and SPA build: passed.
- Svelte runes invariant, secret/PII scan and `git diff --check`: passed.
- Component regressions cover rapid sidebar toggles/inert navigation, immediate overview
  replacement with a single subtree and keyboard focus restored after compact back navigation.
- Browser QA verified 220ms compact forward/back animations, immediate hidden sidebar semantics,
  and reduced-motion rules. A temporary fixture activated the actual compiled reduced-motion
  media rules and observed sidebar transition `0s`, overview animation `none` and activity
  animation `none`. This emulated CSS preference rules without changing the user's OS settings.
- With motion enabled, the 100,000-message browser fixture retained at most 20 mounted rows;
  180 animation frames measured median 16.7ms and p95 16.8ms (approximately 60fps).
  This is fixture/browser evidence, not a native WebKit or all-hardware guarantee.
- Removed the temporary fixture route. No dependencies, IPC contracts, schemas, CLI flags,
  environment variables or documented validation commands changed. No live providers contacted.

### Validation ([04.3])
- Rust workspace check, tests (141 passed), formatting (`cargo fmt --all -- --check`) and Clippy (`-D warnings`): passed.
- Frontend check (zero errors/warnings), unit/component tests (20 passed), Prettier (`prettier-plugin-svelte`) and SPA build: passed.
- Svelte runes invariant, tracked/new-file secret/PII scan and `git diff --check`: passed.
- Regressions cover stale success/error responses, detail identity mismatch, safe plain text,
  empty iframe sandbox/referrer policy, IPC argument casing, explicit lazy download identity,
  duplicate clicks, failed/dismissed downloads, safe filenames and object URL cleanup.
- Sanitizer regressions cover scripts/events, CSS imports/URLs, srcset/picture, SVG/media,
  navigation, forms, remote images and lazy CID resources; raster data images remain supported.
- Synthetic browser QA at desktop and 390×844 verified the actual reader, sandbox document,
  explicit file saving and compact back navigation with restored list focus.

---

### Validation ([04.4])
- User approved the implementation plan and cached bits-ui v2 on 2026-10-30.
- Rust workspace check, tests (141 passed), formatting and Clippy (`-D warnings`): passed.
- Frontend check (zero errors/warnings), tests (36 passed), Prettier and SPA build: passed.
- Regressions cover optimistic rollback, duplicate suppression, stale refresh/search responses,
  canonical attachment identity, UTF-8 tag limits, extraction cache revision invalidation,
  bounded badge requests, keyboard filtering/navigation, Escape, focus restoration, explicit
  unsubscribe gating and escaped extraction values. Existing durable outbox tests still pass.
- Browser QA used 100,000 synthetic summaries at desktop and 390×844. Verified palette layout,
  keyboard action execution, Tab containment, restored focus, local tags and all six badge
  types. Eleven rows were mounted at the observed desktop viewport. No new native frame-rate
  measurement was made. The temporary fixture route was removed before the final SPA build.
- Svelte runes invariant, tracked/new-file email/secret-pattern scan and `git diff --check`: passed.
- Rust socket mocks required loopback binding permission; no live provider, model, AI endpoint
  or package registry was contacted. Cached bits-ui dependency contents and integrity metadata
  were restored locally. Lockfile references/integrity metadata validate; a fresh frozen offline
  pnpm install remains unverified because pnpm's policy metadata mirror is absent (k-008).
- No IPC/schema/core boundaries, CLI flags, environment variables or documented validation
  commands changed. Existing DTOs persist tags through IPC but do not expose saved tags/folder
  overrides for rehydrating UI feedback after restart; these presentation labels are session-local.

---

## Backlog (Upcoming, in priority order)

*(No remaining MVP v0.1 tasks in queue after [04.4]; optimization/hardening phases follow as requested).*

---


## Release / Cycle Wrap-up (Not the next task)

Release/tag only with explicit human request. When triggered, the ID is `[99.1]`. Do not number feature, hygiene, or CI tasks as `99.x`. Do not calculate next task ID from this section.

---

## Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Post-triage webhook & automation consumers implementing `TriageEventHook`
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
