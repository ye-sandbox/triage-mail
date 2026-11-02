# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

[03.6] Local Notifications & Generic Webhook Alerts — READY FOR PLANNING

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
- **Status:** DONE
- **Constraints:** Preserve single-file SQLCipher encryption, exact tie handling, and oracle equivalence.
- **Scheduling:** Promoted to planning after [04.1.7] closure; carries the unmet scoped-search budgets as motivation.

#### Authorized Evaluation (2026-11-02)

User explicitly requested proceeding with the ADR-008/mailbox evaluation. Build an
offline example over the existing 100k-message fixture, with a disposable encrypted
binary index, exact int8 cosine reranking, independent ordered-page oracle checks,
candidate-budget sweeps, deep-page fallback and adversarial correctness probes.
Record raw latency/recall/equivalence and an adoption recommendation. Production
schemas, IPC and ranking remain unchanged; no ANN graphs, downloads or provider calls.

#### Completion (2026-11-02)

- **Commit:** `568bedb` (`perf(storage): evaluate binary candidates against exact search`).
- **Evidence:** `docs/performance/binary-quantization.md`, raw CSV, complete summary,
  process report and offline reducer. 64 groups, 64 reopened and 1,920 warm samples.
- **Result:** Retain exact int8 cosine search (ADR-011). Global seed 4242 BQ-200
  p95 21.226ms retains 56/200 oracle hits; BQ-4096 p95 156.361ms retains 166/200.
  All 270 global warm BQ samples across three seeds fail ordered equality. All
  production and deep exact-fallback samples pass the independent oracle.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (222 Rust
  workspace tests); all three adversarial example probes pass. Desktop check has
  zero errors/warnings; all 133 frontend tests pass. Five policy regressions,
  tracked/new-file policy scan, Svelte invariant, reducer completeness, Prettier for
  the new JS tool and committed whitespace pass. No production code/schema/IPC changes.
- **Knowledge:** k-023 preserves binary/cosine coverage limits; k-024 records the
  vec0 int8 replacement-write fixture gotcha. EN/PT-BR reproduction docs synchronized.
- **Deviations:** Existing scoped/native/frame-cadence misses remain. Investigation
  completion does not declare the proposed sub-15ms exact optimization achieved.
- **Next:** [03.5] promoted to READY FOR PLANNING per the user's prioritized backlog.
  Plan the shared local rule evaluator, simulation, ordering and loop prevention;
  implementation awaits an approved plan. No next-task implementation is started.

---

## Definition of Done (DoD) Checklist — [03.5]

A task MUST NOT be marked done based on subjective appraisal. It MUST satisfy:
- [x] Strict Typing: `cargo check --workspace` and `pnpm --filter desktop check` exit with code 0.
- [x] Automated Tests: `cargo test --workspace` and `pnpm --filter desktop test` exit with code 0.
- [x] Linters: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` exit with code 0.
- [x] Svelte 5 Invariant: `! grep -rn "export let" apps/desktop/src/` exits with code 0.
- [x] Secret & PII Scrubbing: Zero real email addresses, tokens, or secrets in tracked files; `git diff --check` exits with code 0.
- [x] Documentation Perimeter: If CLI flags, `.env.example` variables, or validation commands changed, synchronize `README.md` and `README.pt-br.md` (following `readme-standard` skill).
- [x] Knowledge Preservation: Document obscure platform/runtime gotchas or workarounds as an atomic card in `.agent/knowledge/` (if applicable).
- [x] Atomic Commit: Conventional Commits in English (`feat(scope): ...`).
- [x] Task Log: Active task logged in `.agent/TASK.md` with commit hash; next task promoted.

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

## Prioritized Feature Backlog (2026-11-02)

User-requested feature order below; [02.1] and [01.1] are complete. Each entry
requires its own approved plan before implementation and does not establish a release
commitment. [03.5] completed under its separately approved plan. Dependencies are implementation prerequisites, not parallel work assignments.
All tasks inherit the repository DoD when implemented.

### Task [03.5]: Local Rule Engine & Automatic Filters

- **Status:** DONE
- **Priority:** 1
- **Systems Involved:** `crates/triage-ai`, `crates/storage`, desktop IPC/UI.
- **Scope:** Share one local rule evaluator across filters and alerts. Match account,
  sender/domain, subject, body keywords, attachment metadata and extracted amounts/dates
  with all/any/except conditions. Support tags, stars and archive through existing
  mutation boundaries; define rule ordering, stop-processing behavior and loop prevention.
  Expose post-ingestion/triage integration through the existing observer boundaries.
- **Acceptance:** Create/edit/disable rules, create a rule from a message, and simulate
  against selected existing messages without mutations or deliveries. Explain matches.
  Regression fixtures cover compound conditions, exceptions and conflicting actions.
  Initial sync/history imports never trigger rules unless explicitly requested.

#### Approved Implementation Plan (2026-11-02)

1. `[03.5.1]` **Pure evaluator (`crates/triage-ai`):** Add versioned typed rule,
   condition, message-facts and explanation models in a `rules` module. Preserve
   existing traits. Account IDs match exactly; parsed sender addresses/domains match
   normalized exact values (domain suffix matching is an explicit operator). Subject,
   body and attachment filename/MIME keywords use documented case-insensitive literal
   matching, without scripts or user regex. Attachment predicates use metadata only.
   Compare extracted BRL amounts as integer cents and validated ISO calendar dates;
   missing facts never satisfy positive predicates. Define matching as all required
   conditions AND (any optional condition, when present) AND no exception. Reject
   empty match-all definitions and bound definition size/nesting and simulation batches.
2. `[03.5.2]` **Deterministic action planning:** Evaluate enabled rules in ascending
   explicit position, then stable rule ID, against one immutable message snapshot.
   Accumulate distinct local tags, star and archive actions; repeated actions coalesce.
   A matching stop-processing rule prevents later evaluation. V1 supports additive
   tags, star and archive only; no inverse actions, trash or automatic unsubscribe.
   Simulation uses this exact evaluator/planner and explains matched, excluded,
   missing-fact and stopped rules without writing state or invoking mutation adapters.
3. `[03.5.3]` **Encrypted persistence (`crates/storage`):** Add append-only migration
   v3 for rule definitions/revisions/order, per-folder initial-history baselines,
   durable eligible evaluation jobs and processed-event identities. Keep schema v1/v2
   and existing IPC/traits compatible. Persist eligibility in the ingestion transaction
   with the checkpoint so crashes cannot lose newly eligible work. Persist application
   markers, local changes and existing remote outbox entries atomically. Use canonical
   account-bound identity; a new folder membership is not a new message event. Reject
   invalid/stale revisions; validate account references and definitions natively.
4. `[03.5.4]` **Post-ingestion execution:** Compose a headless-compatible rule consumer
   through the existing post-triage observer boundary without changing its signature.
   Only newly inserted canonical messages after an established folder baseline are
   eligible. First sync, history catch-up/import, newly discovered folders and
   UIDVALIDITY rebuilds establish/refresh baselines without automatic rule actions;
   capture the initial UID frontier so interrupted multi-batch imports stay excluded.
   Messages received while an established account is offline remain eligible on return.
   Resume durable jobs after restart; rule edits/enablement never replay old mail.
   Rule-generated flags/moves, body hydration and duplicate sync events cannot trigger
   evaluation recursively. Reuse existing account-bound mutation/outbox primitives;
   preserve canonical folder/UID targeting and archive snooze clearing. Fixed safe
   errors and bounded job processing must not block sync or silently drop failures.
5. `[03.5.5]` **Additive IPC and desktop UI:** Add rule list/save/delete/reorder and
   simulation commands with typed DTOs, optimistic revision checks and bounded selected
   message IDs. Add an EN/PT-BR Rules settings module to create/edit/disable/order rules,
   choose stop-processing and inspect simulation explanations. Add create-from-message
   as an editable draft requiring save; prefill account and parsed sender without
   persisting message bodies. Simulation and saving never apply rules retroactively.
   Historical application/replay is deferred; existing mail can be simulated only.
   Load `svelte-dev` before UI implementation; retain modal guards and mounted inbox.
6. `[03.5.6]` **Evidence and closure:** Test compound/except/missing predicates,
   amount/date boundaries, deterministic order, coalescing, stop behavior, account
   isolation, migration upgrades, rollback/restart recovery, concurrent edits,
   deduplication, initial multi-batch history and reset suppression, observer loop
   prevention, simulation purity and UI lifecycle guards. Use synthetic reserved-domain
   fixtures and offline adapters. Run every repository DoD command plus frontend build,
   relevant formatting and policy scans. Update active contracts, document architectural
   decisions and applicable knowledge cards; load `readme-standard` if README changes
   are needed. Make an atomic Conventional Commit, record its hash and promote [03.6]
   to planning. Do not push or implement notifications/webhooks in this task.

- **Approval:** User approved the complete plan and proposed defaults via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `4b2e15d` (`feat(rules): add durable local filters and simulation`).
- **Delivered:** Shared pure evaluator/planner, ordered all/any/except conditions,
  tag/star/archive actions and stop behavior; encrypted append-only migration v3,
  revision-safe CRUD/reordering, durable ingestion eligibility, restart recovery,
  move-loop suppression and atomic mutation/outbox completion. Additive typed IPC,
  EN/PT-BR Rules settings, account selectors, create-from-message drafts and side-effect-free
  selected-message simulation preserve the mounted inbox and existing trait/IPC contracts.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass: 234 Rust tests.
  Desktop check reports zero errors/warnings; all 137 frontend tests and SPA build pass.
  Changed frontend files pass Prettier. Five policy scanner regression tests pass;
  tracked/new-file scan covers 272 text files with zero findings. Svelte legacy syntax,
  README perimeter and diff whitespace checks pass. No live-provider calls, credentials,
  model downloads or external deliveries were used.
- **Environment:** Existing k-001/k-006 toolchain/native sysroot. Workspace socket fixtures
  and policy child-process regressions require execution outside the restrictive sandbox;
  all remain local/offline. No system packages or shared toolchain files were modified.
- **Contracts:** ADR-012, active notes, k-025 and paired README usage guides record
  matching semantics, history frontiers and recovery. Rule IDs cannot be reused after
  deletion. Rule changes invalidate pending revisions without retroactive replay.
- **Limits:** Rules require an active runner. Each folder establishes a first post-upgrade
  UID frontier; initial history/rebuilds remain excluded. Missing UIDNEXT suppresses
  automation safely. Account-bound identity tombstones persist until account removal;
  indistinguishable headerless copies can be conservatively suppressed. Amount/date
  predicates test facts independently. Existing search/native performance deviations
  remain outside this feature. Historical application and alert deliveries are deferred.
- **Next:** [03.6] promoted to READY FOR PLANNING. Its external-delivery P0 exception
  still needs an explicit approved plan; no notifications/webhooks implementation began.

### Task [03.6]: Local Notifications & Generic Webhook Alerts

- **Status:** READY FOR PLANNING
- **Priority:** 2
- **Depends On:** [03.5]
- **Systems Involved:** `crates/storage`, `crates/vault`, desktop runtime/IPC/UI;
  headless-compatible delivery adapter through existing observer boundaries.
- **Scope:** Notify on matching new mail locally or through explicitly configured
  webhooks, including user-owned n8n workflows. Select exported fields per destination;
  full bodies are off by default. Store secret URLs/authentication only in vault.
  Persist delivery jobs and bounded execution history in SQLCipher, with retry/backoff,
  destination rate limits, stable event IDs and local duplicate suppression.
- **Acceptance:** Offline adapter tests cover restart/reconnect, retries, disablement,
  payload field selection and secret-free errors/history. Ambiguous remote delivery
  outcomes are documented; do not promise exactly-once delivery to arbitrary receivers.
  No initial-history alert flood; manual replay requires explicit selection. Explain
  that processing requires the app or a separately deployed headless runner to be active.
- **Planning Gate:** Propose and obtain approval for a narrow P0 network-policy exception
  before enabling external deliveries. Specify destination consent, TLS, DNS/redirect
  protections and any explicitly configured private-network webhook behavior. This
  backlog entry does not amend `AGENTS.md` or authorize outbound requests.

### Task [03.7]: Telegram & Discord Alert Connectors

- **Status:** BACKLOG
- **Priority:** 3
- **Depends On:** [03.6] and its approved network-policy exception.
- **Systems Involved:** Delivery adapters, `crates/vault`, desktop IPC/UI.
- **Scope:** Add guided Telegram bot/chat and Discord webhook setup using the shared
  rule engine, payload controls, queue and execution history. Include explicit test
  delivery, destination selection and revocation/disable controls.
- **Acceptance:** Synthetic offline connector tests cover formatting, escaping,
  provider size/rate limits, retryable failures and secret redaction. Tokens and
  secret webhook URLs never appear in logs, exports or returned configuration DTOs.

### Task [04.10]: Saved Searches & Virtual Folders

- **Status:** BACKLOG
- **Priority:** 4
- **Systems Involved:** `crates/storage`, desktop search IPC/sidebar.
- **Scope:** Save named queries and scopes spanning selected accounts without moving
  messages. Provide views such as unread clients, invoices and upcoming bills using
  supported structured predicates. Keep saved definitions in encrypted storage.
- **Acceptance:** Create/rename/delete views; refresh membership after sync or triage;
  preserve pagination, account isolation and canonical message identity. Validate
  full-result membership rather than filtering only the loaded frontend page.

### Task [04.11]: Due-Date Dashboard & Scheduled Reminders

- **Status:** BACKLOG
- **Priority:** 5
- **Depends On:** [03.5], [03.6]; [03.7] for Telegram/Discord destinations.
- **Systems Involved:** Existing deterministic extractors, `crates/storage`, runtime
  scheduling, desktop dashboard and rule UI.
- **Scope:** Display extracted boleto/billing amounts and due dates. Trigger configurable
  reminders before deadlines and when matching messages remain unread for a chosen
  interval. Support local and enabled external destinations with amount thresholds.
- **Acceptance:** Deterministic clock tests cover time zones, date-only deadlines,
  suspend/resume, restarts, duplicate suppression, read-before-deadline cancellation
  and explicit dismissal. Extraction never implies a bill was paid. Missed-reminder
  behavior is visible; no always-on delivery claim when all runners are stopped.

### Task [03.8]: Local Automatic Categories

- **Status:** BACKLOG
- **Priority:** 6
- **Depends On:** [03.5]
- **Systems Involved:** `crates/triage-ai`, `crates/storage`, desktop inbox.
- **Scope:** Classify people, finance, purchases, newsletters and notifications locally,
  starting with deterministic rules. Allow corrections and sender-specific overrides.
  Expose category views without changing canonical IMAP folders.
- **Acceptance:** Synthetic fixtures cover classification and override precedence;
  re-sync preserves corrections. No cloud endpoint or model download is required.
  Learned classification remains separately scoped behind explicit model consent.

### Task [04.12]: New-Sender Review

- **Status:** BACKLOG
- **Priority:** 7
- **Systems Involved:** `crates/storage`, desktop inbox/settings.
- **Scope:** Offer an optional local review view for previously unseen senders, with
  accept/decline controls and per-account preferences; reuse rule actions where useful.
- **Acceptance:** Define normalized sender identity and initial-history seeding during
  planning. Persist review choices across re-sync; never auto-delete messages. Clearly
  distinguish local inbox filtering from blocking delivery at the provider, and never
  present a familiar sender address as proof of authenticated identity.

### Task [04.13]: Conversation Grouping & Thread Triage

- **Status:** BACKLOG
- **Priority:** 8
- **Systems Involved:** MIME/header ingestion, `crates/storage`, desktop inbox/reader.
- **Scope:** Group related messages using Message-ID, In-Reply-To and References;
  define safe fallback behavior for missing/malformed headers and account boundaries.
  Offer conversation-level read/archive/tag actions with explicit affected-message scope.
- **Acceptance:** Fixtures cover duplicate IDs, malformed references, subject collisions
  and cross-folder messages. Preserve per-message IMAP identity, lazy attachments,
  pagination and secure rendering; handle partial mutation failures visibly.

### Task [03.9]: Alert Digests & Quiet Hours

- **Status:** BACKLOG
- **Priority:** 9
- **Depends On:** [03.6], [04.11]; [03.7] for Telegram/Discord destinations.
- **Systems Involved:** Rule scheduler/delivery queue, `crates/storage`, desktop settings.
- **Scope:** Batch routine alerts into scheduled summaries, configure quiet hours per
  destination and allow explicit urgent-rule exceptions. Bound summary payloads and
  apply the same user-selected export fields as individual alerts.
- **Acceptance:** Clock tests cover time zones, overnight windows, suspend/resume,
  restart and digest duplication. Avoid a burst of deferred notifications on resume;
  define missed-window and urgency precedence behavior during planning.

### Task [01.4]: Encrypted Backup & Portable Rule Export

- **Status:** BACKLOG
- **Priority:** 10
- **Depends On:** [03.5], [04.10] for rule and saved-view formats.
- **Systems Involved:** `crates/storage`, `crates/vault`, desktop import/export UI.
- **Scope:** Export/import portable rules and saved views without credentials. Provide
  encrypted local backup/restore for supported settings and locally cached mail data;
  version the format and define a consistent SQLite/WAL snapshot and recovery process.
- **Acceptance:** Round-trip, wrong-passphrase, corruption and version-compatibility
  checks use synthetic fixtures. Credentials, database keys and secret destination URLs
  are excluded from portable exports; restored integrations require reconnection and
  remain disabled until confirmed. Restore never silently overwrites an existing vault
  or database. Clearly state that unfetched attachment binaries are absent from backup.

---

## Other Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
- Multi-folder sync (Archive/All Mail with Gmail dedup, Sent read-only): scoped into the proposed [04.1] mailbox queue; implementation pending approval
- Real Tier 2 ONNX inference via `fastembed`/`ort` (`v0.2`)
- Fuzz/property tests for the in-house IMAP FETCH/literal parser (`v0.2`)
- Optional binary-quantized ANN candidate stage + exact int8 rerank (`v0.2`)
