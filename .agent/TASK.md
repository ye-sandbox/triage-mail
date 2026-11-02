# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

### Task [04.1]: Account Mailboxes Navigation

- **Description:** Implement an expandable account list below global All Mail, with local account photos, functional per-account mailbox views (Inbox, Starred, Snoozed, Spam, Archive, Trash, Other folders), same-account deduplication, and append-only Migration v2.
- **Status:** DONE (with known performance deviations)
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

- [x] **[04.1.4]** `[DONE: 9b9f094]` `[SEQ: W4]` Expose account-scoped mailbox search
  - `Paths:` `crates/storage/src/**, apps/desktop/src-tauri/src/**`
  - `Verify:` `cargo test --workspace`
  - `Plan:` `.agent/plans/04.1-account-mailboxes.md`, section `[04.1.4]`; approved on 2026-11-02.
  - `Implementation:` Optional IPC scope, validated account/view identity, snapshot-consistent eligibility before ranking/pagination, exact scoped vector ranking, and storage/IPC regressions added. Existing folder catalog command reused.
  - `Validation (2026-11-02):` Full native `cargo check --workspace`, `cargo test --workspace` (204 tests), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check` pass using an authorized temporary pkg-config/GTK/WebKitGTK sysroot. Desktop check reports zero errors/warnings and all 102 frontend tests pass. Svelte invariant, policy scan, and diff checks pass. All Cargo dependency resolution remained offline; system packages were not installed.
  - `Compatibility correction:` Native IPC tests exposed an extra null scope field in legacy serialized search requests; omit absent scopes and assert the original wire shape in the DTO regression.
  - `Closure:` Committed as `9b9f094` after user authorization on 2026-11-02. Next task promoted to planning. Temporary sysroot downloads were authorized; no system installation is authorized.

- [x] **[04.1.5]** `[DONE: a1c25ad]` `[SEQ: W5]` Add private local account photos
  - `Paths:` `crates/storage/src/**, apps/desktop/src-tauri/src/**, apps/desktop/src-tauri/Cargo.toml, Cargo.lock, apps/desktop/src/lib/onboarding/**, apps/desktop/src/lib/i18n/**`
  - `Verify:` `cargo test --workspace && pnpm --filter desktop check && pnpm --filter desktop test`
  - `Plan:` `.agent/plans/04.1-account-mailboxes.md`, section `[04.1.5]`; implementation, image decoder dependency, and localization paths approved on 2026-11-02.
  - `Initial dependency preflight:` Offline Cargo metadata failed with `no matching package named image found`; both inspected local Cargo caches lacked the decoder. Implementation paused before application edits until the download exception was approved.
  - `Dependency authorization:` User approved downloading `image` and its required dependencies into a temporary Cargo cache on 2026-11-02. Fetch completed; implementation and validation remained offline afterward. No system installation is authorized.
  - `Implementation:` Separate account-photo get/set/remove IPC, bounded native JPEG/PNG/WebP normalization, metadata-free 128x128 PNG thumbnails in existing encrypted storage, and injectable Svelte 5 photo controls with local initials/color fallback, pending/error handling, and EN/PT-BR labels. Existing account DTOs, schemas, and core traits remain unchanged.
  - `Validation (2026-11-02):` Offline native workspace check, all 209 Rust tests, all-target Clippy with warnings denied, and fmt pass with the existing temporary sysroot. Desktop check reports zero errors/warnings; all 109 frontend tests and the SPA build pass. Prettier checks all changed frontend files using a locally available formatter. Tracked/new-file policy scans report zero findings; Svelte invariant and diff checks pass.
  - `Regression coverage:` Raster format/byte/dimension rejection, JPEG comment/EXIF removal, account identity/isolation, replacement failure preserving the previous photo, encrypted reopen/wrong-key rejection, idempotent removal, native wire shapes, local file selection, late-result disposal, UI remount, and bilingual errors. One frontend test fixture failure was corrected by making its mocked File method configurable.
  - `Closure:` Committed as `a1c25ad` after user authorization on 2026-11-02. Next task promoted to planning. No push is authorized.

- [x] **[04.1.6]** `[DONE: 01f8eb0]` `[SEQ: W6]` Build expandable account navigation and view controls
  - `Paths:` `apps/desktop/src/lib/inbox/**, apps/desktop/src/lib/onboarding/{DesktopApp,InboxSession}.svelte, apps/desktop/src/lib/i18n/**`
  - `Verify:` `pnpm --filter desktop check && pnpm --filter desktop test && ! grep -rn "export let" apps/desktop/src/`
  - `Plan:` `.agent/plans/04.1-account-mailboxes.md`, section `[04.1.6]`; implementation proposal approved by user `proceed` on 2026-11-02; implemented and committed as `01f8eb0`.

  - `Implementation:` Expandable account navigation with private thumbnails, real folder catalogs, native scoped search/pagination/refresh, stale-response guards, star/unstar and local snooze controls, expiry/focus refresh, and EN/PT-BR labels. Settings preserves the inbox; localStorage contains only opaque disclosure IDs.
  - `Validation (2026-11-02):` Offline native workspace check, all 209 Rust tests, all-target Clippy with warnings denied, and fmt pass using the existing temporary sysroot/cache. Desktop check has zero errors/warnings; all 126 frontend tests pass and SPA build passes. Policy scan, Svelte invariant, changed-file formatting, and diff checks pass. Full visual/performance closure remains [04.1.7].
  - `Closure:` Local atomic commit `01f8eb0`; next task [04.1.7] promoted to planning. No push performed.
  - `Environment:` Existing loopback tests and the Node policy scanner needed sandbox exceptions for local sockets/subprocesses. No dependency downloads, provider requests, system installation, or push occurred.

- [x] **[04.1.7]** `[DONE: CLOSURE_HASH]` `[SEQ: W7]` Validate and close the mailbox feature
  - `Paths:` `crates/storage/src/scoped_search.rs, crates/storage/src/scoped_search_tests.rs, feature tests, crates/storage/examples/**, tools/performance/**, apps/desktop/src-tauri/examples/**, docs/performance/**, .agent/TASK.md, .agent/NOTES.md, applicable knowledge cards`
  - `Verify:` `cargo test --workspace && pnpm --filter desktop check && pnpm --filter desktop test && node scripts/ci-guard.mjs`
  - `Plan:` `.agent/plans/04.1-account-mailboxes.md`, section `[04.1.7]`; detailed validation proposal approved by user `proceed` on 2026-11-02. Validation found a production query-plan blocker; returned to planning for scoped follow-up approval. No production fix or dependency change was performed in that initial validation.
  - `Initial evidence:` `docs/performance/mailbox-validation.md`, raw scoped CSVs/query plan, incomplete native JSON, and synthetic screenshots. Global/Starred/Snoozed: 720 warm samples with exact ordering/eligibility/identity assertions. Inbox: external 180-second budget exhausted before first returned sample (exit 124). Native: ready phase only, 90-second budget exhausted; no first-page/frame pass.
  - `Initial diagnosis:` Correlated Inbox membership lookup chooses account-only `idx_memberships_account_folder` scans rather than the message-ID index on the 100k/200k fixture; potential ~1.25 billion visits. See k-020. Native wrapper timeout cleanup fixed within the validation harness and verified using a disposable wrapper/child.
  - `Initial checks (2026-11-02):` Offline workspace check, all 209 Rust tests, fmt, all-target Clippy including an extra all-features pass, desktop zero-error/warning check, all 126 frontend tests, fixture build and restored ordinary SPA build pass. Local socket/subprocess exceptions only; no network downloads, provider access, system installation, push, or tag.
  - `Initial closure:` NOT DONE. Remaining dense/empty/folder timings, native first-page/scroll reports and visual matrix are listed in the validation report. Next milestone is not promoted. Validation committed locally as `4dea59c`; scoped production follow-up approved by user `proceed` on 2026-11-02; executed; see final evidence below. Policy scan includes 236 tracked text files with zero findings; diff and Svelte invariants pass.
  - `Approved production follow-up (2026-11-02):` User `proceed` authorized scoped storage changes and regression tests. Fixed message-correlated membership planning using the existing message-ID index and bounded adaptive account/deadline enumeration using existing indices; schemas, IPC and traits preserved. Five new regressions cover query plans, bounded VM work, cross-account snooze skew and zero-norm NULL ordering. A candidate-frontier shortcut failed that ordering regression and was removed.
  - `Final evidence:` All 56 storage workloads / 1,680 warm samples pass independent rank/count/identity oracles. Blank Inbox p95 35.373ms replaces the original timeout; Snoozed semantic p95 16.702ms improves on 23.540ms but still misses 15ms. Native exits 0 with every first-page/scroll report; <=18 mounted rows at 100k. Remaining lexical/semantic/hybrid, first-page and frame-cadence budget misses are listed in the report. Browser visual follow-up covers zero/one accounts, light/dark, thumbnail, static reduced-motion layout, loading/not-synced/offline states.
  - `Final checks:` Offline workspace check, all 214 Rust tests, all-target Clippy plus all-features Clippy with warnings denied, fmt, zero-error/warning desktop check, all 126 frontend tests, fixture and restored ordinary SPA builds pass. Policy scan: 241 tracked text files, zero findings; diff, fixture formatting and Svelte invariant pass. No downloads, live accounts, system installation, tag or push.
  - `Closure (2026-11-02):` Closed by user decision with the unmet timing budgets recorded as known deviations in `docs/performance/mailbox-validation.md` and NOTES; no target weakened. Production fixes: `dcc320a`, `8794854`; evidence: `a5ddad2`. Next milestone [01.1] promoted to PLANNING.


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
- **Status:** PLANNING
- **Constraints:** Preserve single-file SQLCipher encryption, exact tie handling, and oracle equivalence.
- **Scheduling:** Promoted to planning after [04.1.7] closure; carries the unmet scoped-search budgets as motivation.

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
