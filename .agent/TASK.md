# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

[01.1] Binary Quantization & Global Vector Pre-filtering Evaluation — PLANNING

---

## Next Milestone (v0.2.0)

### 📌 Task [02.1]: Intelligent Account Autodiscovery & Streamlined Onboarding

- **Description:** Add offline provider presets, explicit domain-only ISPDB lookup and confirmed-host TLS probing in `crates/mail-engine`, exposed through additive Tauri IPC. Streamline email-first onboarding, Microsoft PKCE with a user-provided client ID, Gmail App Password assistance and advanced manual settings.
- **Systems Involved:** `crates/mail-engine` (`discovery/`), `apps/desktop/src-tauri` (`onboarding.rs`, `runtime.rs`), `apps/desktop` (`src/lib/onboarding/`).
- **Action Type:** Feature & UX Architecture.
- **Status:** DONE
- **Subtasks:**
  - `[02.1.1]` **Discovery Engine Core (`crates/mail-engine`):** Parse domain from RFC 5322 email addresses, match against instant offline presets table (Gmail, Outlook/Hotmail, Yahoo, iCloud, Fastmail, Zoho, popular BR domains), query Mozilla ISPDB (`autoconfig.thunderbird.net/v1.1/{domain}`) with 3s timeout, and offer a separately confirmed TLS port 993 probe.
  - `[02.1.2]` **IPC Bridge & DTOs (`apps/desktop/src-tauri`):** Implement `cmd_discover_email_provider` returning `DiscoveredProviderDto` with recommended auth method, OAuth endpoint metadata, and contextual help URLs without leaking credentials.
  - `[02.1.3]` **Streamlined Onboarding UI (`apps/desktop`):** Refactor `AccountForm.svelte` to feature a Microsoft PKCE setup button requiring the user’s own client ID, debounce autodiscovery on email input blur, provide direct link assistance for Gmail App Passwords (`myaccount.google.com/apppasswords`), and place host/port/BYO-OAuth/Proton controls in a collapsible advanced details panel.
- **Constraints:**
  - Zero credential leakage in logs or debug strings.
  - Strict timeout ($\le 3$s) on external ISPDB requests; never freeze UI.
  - Svelte 5 Runes strictly enforced (`$state`, `$derived`, `$effect`, zero `export let`).
  - Unit tests MUST use RFC 2606 reserved domains (`@example.com`, `@example.org`).

#### Completion (2026-11-02)

- **Commit:** `3c25660` (`feat(onboarding): add consent-based provider discovery`).
- **Validation:** Workspace check/test/fmt/Clippy passed (222 Rust tests). Desktop check
  passed with zero errors/warnings; all 133 frontend tests and SPA build passed.
  Changed frontend files pass Prettier; policy guard regression tests pass (5 tests)
  and the tracked-file secret/PII/Svelte policy scan reports zero findings. Diff check
  and the legacy Svelte syntax invariant pass.
- **Environment:** Used the existing native sysroot and toolchains described in k-001
  and k-006. Full Rust tests require loopback access; policy guard tests require child
  processes, so those validations ran outside the restrictive sandbox. No provider
  credentials, model downloads or live-provider authentication were used.
- **Contracts:** Existing IPC/account authorization and storage schemas are unchanged.
  New discovery defaults offline; probing is a separate explicit action. Microsoft
  setup uses user-provided credentials rather than a bundled application registration.
- **Documentation:** EN/PT-BR setup guides, active contracts and the approved narrow P0
  exception are synchronized. k-022 preserves discovery trust and parser/UI gotchas.
- **Next:** [01.1] remains PLANNING; implementation requires its own approved scope.

#### Approved Implementation Plan (2026-11-02)

1. `[02.1.1]` Add headless discovery types and an injectable discovery transport in
   `crates/mail-engine`. Validate and normalize the address domain, resolve offline
   presets first, and return an explicit unresolved result instead of guessing success.
   Confirm the supported Brazilian provider list during implementation review.
2. Under the approved network-policy exception below, add bounded ISPDB lookup (domain only,
   never the full address), defensive XML parsing, and bounded IMAP TLS probing.
   Disable redirects and inherited proxies; do not use provider XML for automatic OAuth
   endpoint trust or permit discovery to contact private/loopback destinations.
   Preserve explicitly configured local Proton Bridge behavior.
3. `[02.1.2]` Add the documented `cmd_discover_email_provider` DTO/command without changing
   existing account or authorization contracts. Keep errors fixed and credential-free;
   expose injectable offline services for tests. Discovery never authenticates or saves
   an account automatically.
4. `[02.1.3]` Refactor onboarding around email-first setup, offline discovery on blur,
   contextual Gmail assistance, and advanced manual settings. Ignore stale discovery
   responses and preserve manual edits. Retain transient credential inputs, existing
   OAuth cancellation, Proton Bridge, and EN/PT-BR parity. Load `svelte-dev` before UI edits.
5. Add offline discovery/parser/timeout and IPC tests plus UI regressions for stale
   results, manual overrides, and authorization. Run the full repository DoD commands,
   frontend build and formatting checks; review the diff for secrets and PII. Update
   contracts and applicable documentation, then make an atomic Conventional Commit
   and record its hash before promoting the next task. Do not push.

#### Approved Decisions (2026-11-02)

- **P0 network policy:** AGENTS.md permits only the approved, explicit discovery actions. Approved exception: retain automatic
  offline presets, require an explicit user action for domain-only ISPDB lookup, and
  require user confirmation of a discovered IMAP endpoint before any probe.
  User approved the proposed exception via `proceed`; online lookup and probing remain explicit runtime actions.
- **Microsoft OAuth:** No application client ID is configured in the current onboarding
  flow. Approved scope: streamline Microsoft PKCE using a user-provided client ID held
  transiently and stored only in vault, with a clear setup step before authorization.
  A first-use one-click flow without that setup requires a separate approved application
  registration and credential-provisioning design; do not embed borrowed client IDs.

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

## Follow-up [04.1]: Inbox Settings Cleanup (2026-11-02)

- **Status:** IMPLEMENTED; full workspace validation blocked by concurrent discovery work.
- **User-requested scope:** Remove redundant Accounts and Settings buttons from message
  actions; keep the responsive settings gear and manage accounts inside its modal.
- **Commit:** `ff23564` (`fix(inbox): centralize account management in settings`).
- **Validation:** Desktop check (zero errors/warnings), all 126 frontend tests, production
  build, updated onboarding regression, Svelte syntax invariant, and diff check passed.
  Rust formatting reports discovery changes; Clippy reports discovery XML type errors.
  Cargo check/test were stopped after that blocker; existing [02.1] work and local
  edits remain owned by that task.

---

## Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Post-triage webhook & automation consumers implementing `TriageEventHook`
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
- Multi-folder sync (Archive/All Mail with Gmail dedup, Sent read-only): scoped into the proposed [04.1] mailbox queue; implementation pending approval
- Real Tier 2 ONNX inference via `fastembed`/`ort` (`v0.2`)
- Fuzz/property tests for the in-house IMAP FETCH/literal parser (`v0.2`)
- Optional binary-quantized ANN candidate stage + exact int8 rerank (`v0.2`)
