# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

### Task [04.1]: Unified Settings Modal

- **Description:** Implement a unified, accessible, and responsive Settings dialog with typed modules, persistent mounted inbox shell, gear entry point, and embedded account management.
- **Status:** RUNNING
- **Action Type:** Implementation across 2 parallel waves (`W1` and `W2`).
- **Plan and implementation tasks:** [Unified Settings Modal](plans/04.1-settings-modal.md).
- **Scope:** Persistent inbox beneath dim overlay, gear trigger in sidebar footer and toolbar fallback, typed modules (General, Accounts, Search, AI, Privacy), unsaved-form guards, pending operation isolation, EN/PT-BR localization.

#### Execution Queue (`W1` -> `W2`)

- [x] **[04.1.8]** `[DONE: e3b2ff4]` `[PARALLEL: W1]` Implement Settings dialog primitive and typed modules (General, Search, AI, Privacy)
  - `Model:` `gemini-3.8-flash-medium`
  - `Read:` `apps/desktop/src/lib/settings/SettingsView.svelte, apps/desktop/src/lib/settings/AiSettingsForm.svelte, apps/desktop/src/lib/settings/ModelSettings.svelte`
  - `Paths:` `apps/desktop/src/lib/settings/types.ts, apps/desktop/src/lib/settings/SettingsDialog.svelte, apps/desktop/src/lib/settings/modules/GeneralModule.svelte, apps/desktop/src/lib/settings/modules/SearchModule.svelte, apps/desktop/src/lib/settings/modules/AiModule.svelte, apps/desktop/src/lib/settings/modules/PrivacyModule.svelte, apps/desktop/src/lib/settings/dialog.svelte.test.ts`
  - `Verify:` `([ -e node_modules ] || ln -s /home/yegear/github/triage-mail/node_modules node_modules) && ([ -e apps/desktop/node_modules ] || cp -RP /home/yegear/github/triage-mail/apps/desktop/node_modules apps/desktop/node_modules) && pnpm --filter desktop check && pnpm --filter desktop exec vitest run src/lib/settings/dialog.svelte.test.ts && ! grep -rn "export let" apps/desktop/src/`

- [x] **[04.1.9]** `[DONE: 6c21bbf]` `[PARALLEL: W1]` Implement persistent inbox shell, sidebar gear entry point, and live search preference synchronization
  - `Model:` `gemini-3.8-flash-medium`
  - `Read:` `apps/desktop/src/lib/inbox/Sidebar.svelte, apps/desktop/src/lib/inbox/SearchToolbar.svelte, apps/desktop/src/lib/onboarding/DesktopApp.svelte, apps/desktop/src/lib/onboarding/InboxSession.svelte`
  - `Paths:` `apps/desktop/src/lib/inbox/Icon.svelte, apps/desktop/src/lib/inbox/Sidebar.svelte, apps/desktop/src/lib/inbox/SearchToolbar.svelte, apps/desktop/src/lib/onboarding/DesktopApp.svelte, apps/desktop/src/lib/onboarding/InboxSession.svelte, apps/desktop/src/lib/inbox/InboxView.svelte, apps/desktop/src/lib/inbox/settings-shell.svelte.test.ts`
  - `Verify:` `([ -e node_modules ] || ln -s /home/yegear/github/triage-mail/node_modules node_modules) && ([ -e apps/desktop/node_modules ] || cp -RP /home/yegear/github/triage-mail/apps/desktop/node_modules apps/desktop/node_modules) && pnpm --filter desktop check && pnpm --filter desktop exec vitest run src/lib/inbox/settings-shell.svelte.test.ts && ! grep -rn "export let" apps/desktop/src/`

- [ ] **[04.1.10]** `[APPROVED]` `[PARALLEL: W2]` Embed account management in Settings with reusable account list and add account flow
  - `Model:` `gemini-3.8-flash-medium`
  - `Read:` `apps/desktop/src/lib/onboarding/OnboardingView.svelte, apps/desktop/src/lib/onboarding/AccountForm.svelte, apps/desktop/src/lib/onboarding/controller.svelte.ts`
  - `Paths:` `apps/desktop/src/lib/onboarding/AccountList.svelte, apps/desktop/src/lib/onboarding/OnboardingView.svelte, apps/desktop/src/lib/settings/modules/AccountsModule.svelte, apps/desktop/src/lib/settings/accounts-module.svelte.test.ts`
  - `Verify:` `([ -e node_modules ] || ln -s /home/yegear/github/triage-mail/node_modules node_modules) && ([ -e apps/desktop/node_modules ] || cp -RP /home/yegear/github/triage-mail/apps/desktop/node_modules apps/desktop/node_modules) && pnpm --filter desktop check && pnpm --filter desktop exec vitest run src/lib/settings/accounts-module.svelte.test.ts src/lib/onboarding/onboarding.svelte.test.ts && ! grep -rn "export let" apps/desktop/src/`

- [ ] **[04.1.11]** `[APPROVED]` `[PARALLEL: W2]` Add unsaved-form dirty guards, pending operation protection, responsive layout, EN/PT-BR copy, and active contracts
  - `Model:` `gemini-3.8-flash-medium`
  - `Read:` `apps/desktop/src/lib/i18n/translations.ts, apps/desktop/src/lib/settings/SettingsDialog.svelte, .agent/NOTES.md`
  - `Paths:` `apps/desktop/src/lib/settings/guards.ts, apps/desktop/src/lib/i18n/translations.ts, apps/desktop/src/lib/settings/modal-flow.svelte.test.ts, .agent/NOTES.md`
  - `Verify:` `([ -e node_modules ] || ln -s /home/yegear/github/triage-mail/node_modules node_modules) && ([ -e apps/desktop/node_modules ] || cp -RP /home/yegear/github/triage-mail/apps/desktop/node_modules apps/desktop/node_modules) && pnpm --filter desktop check && pnpm --filter desktop exec vitest run src/lib/settings/modal-flow.svelte.test.ts && git diff --check && node scripts/ci-guard.mjs && ! grep -rn "export let" apps/desktop/src/`

## Completed Tasks

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

## Retained Planning Queue (not active)

### Task [04.1]: Account Mailboxes Navigation

- **Description:** Plan an expandable account list below global All Mail, with local account photos and functional per-account mailbox views.
- **Status:** PLANNING
- **Action Type:** Planning only; implementation requires user approval.
- **Plan and implementation tasks:** [Account Mailboxes Plan](plans/04.1-account-mailboxes.md).
- **UI planning addendum:** [Unified Settings Modal](plans/04.1-settings-modal.md), integrating Accounts into a centered modular dialog opened by one sidebar-footer gear icon.
- **Scope:** Inbox, Starred, Snoozed, Spam, Archive, Trash, and additional discovered server folders. Keep global All Mail.
- **Additional UI scope:** Preserve the mounted inbox behind Settings; organize General, Accounts, Search, AI and models, and Privacy in an internal sidebar. Planning only.
- **Next step:** Review the proposed behavior and contract extensions, then approve the implementation queue. No implementation task is RUNNING.

### Separate Settings Modal Queue (under [04.1])

All tasks remain PLANNING; implementation approval is still required. Details, paths,
dependencies, and acceptance criteria: [Settings Modal Tasks](plans/04.1-settings-modal.md#separate-implementation-tasks).

| Task | Scope | Depends on |
|---|---|---|
| [04.1.8] | Persistent inbox, gear entry point, and dialog shell | Explicit modal implementation approval |
| [04.1.9] | Typed Settings modules and live search preferences | [04.1.8] |
| [04.1.10] | Embedded account management | [04.1.9] |
| [04.1.11] | Unsaved-form guards, pending operations, and credential cleanup | [04.1.10] |
| [04.1.12] | Responsive layout, EN/PT-BR, and accessibility | [04.1.11] |
| [04.1.13] | Regression checks, visual QA, and feature closure | [04.1.12] |

Mailbox tasks [04.1.1]–[04.1.7] keep their existing IDs. The modal queue may run first
after approval; shared UI files require sequential integration with mailbox work.

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
