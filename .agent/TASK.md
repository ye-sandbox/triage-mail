# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

### Task [04.1]: Account Mailboxes Navigation

- **Description:** Implement an expandable account list below global All Mail, with local account photos, functional per-account mailbox views (Inbox, Starred, Snoozed, Spam, Archive, Trash, Other folders), same-account deduplication, and append-only Migration v2.
- **Status:** RUNNING
- **Action Type:** Sequential / Wave implementation across tasks `[04.1.1]`–`[04.1.7]`.
- **Plan and implementation tasks:** [Account Mailboxes Plan](plans/04.1-account-mailboxes.md).
- **Scope:** Append-only Migration v2 (`folders`, `message_folder_memberships`, `account_avatars`, `messages` column additions), folder catalog synchronization, two-way Starred and local Snoozed behavior, account-scoped search, local private avatars, and Svelte 5 expandable account navigation.

#### Execution Queue

- [x] **[04.1.1]** `[DONE]` `[SEQ: W1]` Finalize mailbox contracts, append-only migration design, and deduplication specifications
  - `Paths:` `docs/adr/decisions.md, .agent/NOTES.md, .agent/plans/04.1-account-mailboxes.md`
  - `Verify:` `git diff --check && node scripts/ci-guard.mjs`

- [x] **[04.1.2]** `[DONE: c4676d1]` `[SEQ: W2]` Discover and synchronize account folders
  - `Paths:` `crates/mail-engine/src/**, crates/storage/src/**, apps/desktop/src-tauri/src/**`
  - `Verify:` `cargo test --workspace`

- [x] **[04.1.3]** `[DONE: 8779527]` `[SEQ: W3]` Add persisted Starred and local Snoozed behavior
  - `Paths:` `crates/storage/src/**, crates/mail-engine/src/**, apps/desktop/src-tauri/src/**`
  - `Verify:` `cargo test --workspace`

- [ ] **[04.1.4]** `[RUNNING]` `[SEQ: W4]` Expose account-scoped mailbox search
  - `Paths:` `crates/storage/src/**, apps/desktop/src-tauri/src/**`
  - `Verify:` `cargo test --workspace`
  - `Plan:` `.agent/plans/04.1-account-mailboxes.md`, section `[04.1.4]`; approved on 2026-11-02.
  - `Implementation:` Optional IPC scope, validated account/view identity, snapshot-consistent eligibility before ranking/pagination, exact scoped vector ranking, and storage/IPC regressions added. Existing folder catalog command reused.
  - `Validation (2026-11-02):` Full native `cargo check --workspace`, `cargo test --workspace` (204 tests), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check` pass using an authorized temporary pkg-config/GTK/WebKitGTK sysroot. Desktop check reports zero errors/warnings and all 102 frontend tests pass. Svelte invariant, policy scan, and diff checks pass. All Cargo dependency resolution remained offline; system packages were not installed.
  - `Compatibility correction:` Native IPC tests exposed an extra null scope field in legacy serialized search requests; omit absent scopes and assert the original wire shape in the DTO regression.
  - `Closure:` Implementation and validation are complete. User authorized the atomic commit and promotion of [04.1.5] to planning on 2026-11-02. Record the resulting commit before promoting the next task. Temporary sysroot downloads were authorized; no system installation is authorized.

- [ ] **[04.1.5]** `[PLANNED]` `[SEQ: W5]` Add private local account photos
  - `Paths:` `crates/storage/src/**, apps/desktop/src-tauri/src/**, apps/desktop/src/lib/onboarding/**`
  - `Verify:` `cargo test --workspace && pnpm --filter desktop check && pnpm --filter desktop test`

- [ ] **[04.1.6]** `[PLANNED]` `[SEQ: W6]` Build expandable account navigation and view controls
  - `Paths:` `apps/desktop/src/lib/inbox/**, apps/desktop/src/lib/onboarding/{DesktopApp,InboxSession}.svelte, apps/desktop/src/lib/i18n/**`
  - `Verify:` `pnpm --filter desktop check && pnpm --filter desktop test && ! grep -rn "export let" apps/desktop/src/`

- [ ] **[04.1.7]** `[PLANNED]` `[SEQ: W7]` Validate and close the mailbox feature
  - `Paths:` `feature tests, .agent/TASK.md, .agent/NOTES.md`
  - `Verify:` `cargo test --workspace && pnpm --filter desktop check && pnpm --filter desktop test && node scripts/ci-guard.mjs`

## Completed Tasks

### Task [04.1]: Unified Settings Modal

- **Description:** Implement a unified, accessible, and responsive Settings dialog with typed modules, persistent mounted inbox shell, gear entry point, and embedded account management.
- **Status:** DONE
- **Action Type:** Implementation across 2 parallel waves (`W1` and `W2`).
- **Commits:** `e3b2ff4`, `6c21bbf`, `e9464eb`, `e59d45a`
- **Plan and implementation tasks:** [Unified Settings Modal](plans/04.1-settings-modal.md).
- **Scope:** Persistent inbox beneath dim overlay, gear trigger in sidebar footer and toolbar fallback, typed modules (General, Accounts, Search, AI, Privacy), unsaved-form guards, pending operation isolation, EN/PT-BR localization.

### Task [04.9]: Application Title Bar

- **Description:** Implement an application-styled title bar replacing OS decorations, with themed window controls and a persistent desktop shell.
- **Status:** DONE
- **Action Type:** Implementation across 2 parallel waves (`W1` and `W2`).
- **Commits:** `626aaba`, `4cb5658`, `76973ef`, `ec087bc`
- **Plan and implementation tasks:** [Application Title Bar](plans/04.9-custom-title-bar.md).
- **Scope:** Themed chrome, native drag/minimize/maximize/restore/close, minimal Tauri capabilities, remaining-height layout, EN/PT-BR, accessibility, and native compatibility verification.

#### Execution Queue (`W1` -> `W2`)

- [x] **[04.9.1]** `[DONE: 626aaba]` `[PARALLEL: W1]` Implement typed window adapter, Tauri transport, and themed/localized/accessible `TitleBar.svelte` component
- [x] **[04.9.2]** `[DONE: 4cb5658]` `[PARALLEL: W1]` Configure remaining-height desktop shell CSS, view viewport layouts, undecorated window, and minimal Tauri capabilities
- [x] **[04.9.3]** `[DONE: 76973ef]` `[PARALLEL: W2]` Integrate persistent `TitleBar` in `+layout.svelte` global desktop shell and add shell/dialog regression tests
- [x] **[04.9.4]** `[DONE: ec087bc]` `[PARALLEL: W2]` Record Application Title Bar active contracts in `.agent/NOTES.md`

---

## Next Milestone (v0.2.0)

### 📌 Task [01.1]: Binary Quantization & Global Vector Pre-filtering Evaluation

- **Description:** Evaluate 1-bit scalar/binary quantization (BQ) candidate generator + exact int8 reranking for sub-15ms global vector queries on 100k messages in SQLCipher, per ADR-008.
- **Systems Involved:** `crates/storage` (vector indexing, Hamming distance pre-filter, SQLCipher).
- **Action Type:** Investigation & Architecture.
- **Status:** READY FOR PLANNING
- **Constraints:** Preserve single-file SQLCipher encryption, exact tie handling, and oracle equivalence.
- **Scheduling:** Retained unchanged; title-bar planning takes priority for the current user request. Not active.

---

## Definition of Done (DoD) Checklist

A task MUST NOT be marked done based on subjective appraisal. It MUST satisfy:
- [ ] Strict Typing: `cargo check --workspace` and `pnpm --filter desktop check` exit with code 0.
- [ ] Automated Tests: `cargo test --workspace` and `pnpm --filter desktop test` exit with code 0.
- [ ] Linters: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` exit with code 0.
- [ ] Svelte 5 Invariant: `! grep -rn "export let" apps/desktop/src/` exits with code 0.
- [ ] Secret & PII Scrubbing: Zero real email addresses, tokens, or secrets in tracked files; `git diff --check` exits with code 0.
- [ ] Documentation Perimeter: If CLI flags, `.env.example` variables, or validation commands changed, synchronize `README.md` and `README.pt-br.md` (following `readme-standard` skill).
- [ ] Knowledge Preservation: Document obscure platform/runtime gotchas or workarounds as an atomic card in `.agent/knowledge/` (if applicable).
- [ ] Atomic Commit: Conventional Commits in English (`feat(scope): ...`).
- [ ] Task Log: Active task logged in `.agent/TASK.md` with commit hash; next task promoted.

---

## Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Post-triage webhook & automation consumers implementing `TriageEventHook`
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
- Multi-folder sync (Archive/All Mail with Gmail dedup, Sent read-only): scoped into the proposed [04.1] mailbox queue; implementation pending approval
- Real Tier 2 ONNX inference via `fastembed`/`ort` (`v0.2`)
- Fuzz/property tests for the in-house IMAP FETCH/literal parser (`v0.2`)
- Optional binary-quantized ANN candidate stage + exact int8 rerank (`v0.2`)
