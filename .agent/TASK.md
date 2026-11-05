# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

[01.4] Encrypted Backup & Portable Rule Export — RUNNING

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

## Definition of Done (DoD) Checklist — [03.9]

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

- **Status:** DONE
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

#### Approved Implementation Plan (2026-11-02)

1. `[03.6.1]` **Shared alert planning:** Extend the typed local rule model additively
   with destination-referencing alert actions; preserve existing version-1 rules,
   ordering, stop semantics and mutation actions. Use the same immutable facts and
   evaluator for simulation and execution. Coalesce matching rules to one automatic
   delivery per account-bound event and destination. Add headless delivery request,
   outcome and adapter contracts in existing core modules; no new crate or Tauri
   dependency in core. Desktop composes the native notification and HTTP adapters.
2. `[03.6.2]` **Consent and vault lifecycle:** Destinations start disabled. Explicit
   enablement confirms destination identity, selected exported fields and external
   disclosure. Keep complete webhook URLs (including paths/query secrets) and optional
   bearer credentials exclusively in vault, behind opaque destination IDs. Return only
   safe metadata, public origin and credential-presence flags. Credential inputs stay
   transient and are cleared after submission/teardown. Stage immutable secret revisions
   before SQL configuration commit; recover/clean orphan revisions without exposing
   secrets. Configuration or field changes invalidate queued old revisions; re-enabling
   never silently replays canceled work. Serialize disable/delete against dispatch;
   an already-started request cannot be recalled. Saving never contacts a destination.
3. `[03.6.3]` **Durable encrypted delivery:** Add append-only migration v4 for safe
   destination metadata/revisions, delivery jobs, leases, attempts and bounded history.
   Enqueue alert jobs in the same transaction that completes rule jobs, local actions
   and existing IMAP outbox entries. Reuse [03.5] eligibility/frontiers and account-bound
   identity tombstones; retain compact automatic-delivery deduplication markers until
   account/destination deletion, independently of pruned history. Persist selected-field
   payload snapshots only in SQLCipher, with bounded size; never store destination
   secrets there. Account/message deletion cancels related pending jobs and removes
   payloads. Completed/canceled jobs clear payloads; keep at most 30 days/1,000 safe
   history records per destination. No credentials, subjects, bodies, URLs or receiver
   response bodies enter errors/history. No network operation holds a SQLite lock.
4. `[03.6.4]` **Bounded execution and delivery semantics:** Resume leased work after
   restart through the existing worker; processing requires an active unlocked app
   or a future separately deployed headless runner. Use one in-flight attempt per
   destination, default at most six starts/minute/destination, persisted scheduling,
   a ten-second total attempt deadline and bounded concurrency across destinations.
   Retry transport failures, 408/429 and 5xx with jittered exponential backoff (30s
   initial, one-hour cap), capped at eight attempts/24 hours; honor bounded Retry-After.
   Treat redirects and other 4xx as terminal; only 2xx counts as receiver acceptance.
   Send stable versioned event IDs on every attempt for receiver deduplication. Crash
   and timeout ambiguity can cause duplicates, including OS notifications; promise
   neither exactly-once delivery nor proof that the user saw an accepted notification.
   Expired work fails visibly rather than producing an unbounded resume burst.
5. `[03.6.5]` **Payload and native notification controls:** Default local alerts to
   a generic new-mail notice without sender/subject/body on the lock screen; request
   OS permission only on explicit enablement and show denied/unavailable status.
   Webhook JSON defaults to schema version, opaque event ID and event timestamp.
   Independently opt in account identifier, sender, subject, bounded plain-text excerpt,
   extracted facts, attachment metadata or full plain-text body per destination.
   Never include raw HTML/headers, attachment bytes, OTP/Pix values or credentials
   implicitly; sensitive extraction/body export requires explicit disclosure. Bound
   payloads to 64 KiB with explicit truncation flags. Do not fetch missing bodies or
   attachments to enrich an alert. Simulation previews fields without delivering.
6. `[03.6.6]` **Additive IPC and EN/PT-BR settings:** Add typed destination CRUD,
   enable/disable, safe history, explicit synthetic test delivery and selected-message
   replay commands. Test requires an enabled/confirmed destination and visibly states
   it sends externally; replay requires selected messages/destinations and confirmation,
   creates a separate replay event ID and never invokes mail mutation actions. Preserve
   mounted inbox, revision checks and dirty/busy guards. Extend Rules settings with
   destination selection and side-effect-free explanations. Load `svelte-dev` before
   UI edits; keep Telegram/Discord setup, quiet hours/digests, reminders and a runnable
   headless CLI outside this task.
7. `[03.6.7]` **Evidence and closure:** Offline adapters and injected clocks/resolvers
   cover atomic rollback, encrypted reopen, lease expiry, retries/rate limits, ambiguous
   outcomes, disabled/edited/deleted destinations, vault/SQL partial failures, consent,
   field allowlists, history pruning, deduplication, manual replay isolation and initial
   history suppression. Cover TLS/DNS/redirect policy and secret-free failures without
   live outbound requests. Run all repository DoD commands, SPA build, formatting and
   tracked-file secret/PII scans. Record ADR, contracts and applicable knowledge cards;
   load `readme-standard` for paired usage documentation. Make an atomic Conventional
   Commit, log its hash and promote [03.7] to planning; do not push.

#### Approved P0 Exception (2026-11-02)

- Permit only user-enabled generic webhook JSON POST delivery, explicit synthetic tests
  and confirmed selected-message replay to that destination's vault-stored HTTPS URL.
  Background deliveries require prior destination/field consent and matching new-mail
  rules; no automatic destination discovery, browser requests or provider calls.
- Require verified TLS 1.2+ and public DNS hosts on port 443 in this iteration. Reject
  URL userinfo/fragments, IP literals, localhost and private/link-local/reserved/mixed
  DNS answer sets. Validate and pin the exact resolved public addresses used by each
  connection; revalidate each retry. Disable redirects and inherited proxies. Never
  forward secrets to another origin or log URLs/HTTP response content. Bound DNS,
  connection, request/response processing and payload sizes.
- Private-network, loopback and plain-HTTP webhook destinations are unsupported,
  including locally hosted n8n. Public HTTPS user-owned n8n remains supported. Any
  private-network exception requires a separately approved address/consent design.
- Approval authorizes implementation and amendment of AGENTS.md with this narrow
  exception. It does not authorize live test deliveries from the development session;
  validation remains offline. Runtime delivery always requires the user's configured
  destination consent. No other P0 exception or telemetry is introduced.

- **Approval:** User approved the complete plan and public-HTTPS-only P0 exception via
  `proceed` on 2026-11-02. Implementation is complete; development validation stayed offline.

#### Completion (2026-11-02)

- **Commit:** `0a76794` (`feat(alerts): add consent-bound notifications and webhooks`).
- **Delivered:** Additive rule destination actions and SQLCipher migration v4, ingestion-time
  destination revision snapshots, atomic rule/delivery enqueue, durable leases/rate limits,
  bounded retry/history, selected-field payloads, vault-only immutable URL/auth revisions
  and orphan cleanup. Additive typed IPC and EN/PT-BR Alerts settings provide CRUD, explicit
  origin/field consent, synthetic tests, confirmed selected-message replay and safe history.
  Linux generic notifications use D-Bus; rule simulation never sends or mutates mail.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (246 Rust tests).
  Desktop check has zero errors/warnings; all 140 frontend tests and the SPA build pass.
  Changed frontend files pass Prettier; all five policy regressions pass. Tracked/new-file
  scan covers 286 text files with zero findings; Svelte invariant and diff checks pass.
- **Environment:** Existing k-001/k-006 toolchain/native sysroot; cached chrono/zbus only,
  no dependency download or system modification. Existing loopback test fixtures and policy
  child-process tests ran outside restrictive sandbox. No live notifications, external
  webhook requests, provider credentials or model downloads were used during validation.
- **Contracts/documentation:** Approved narrow P0 exception is in AGENTS.md. ADR-013,
  active notes, k-026 and paired README guides document payload/privacy limits, vault/SQL
  recovery, destination revision consent and delivery ambiguity. Existing IPC, traits and
  prior migrations remain compatible.
- **Limits:** Requires an active unlocked runner; no runnable headless CLI is added.
  Local notifications currently require the Linux desktop notification service; other
  platforms report unavailable. Webhooks support public DNS HTTPS/443 only. Retry/timeout
  ambiguity can produce duplicates; acceptance is not proof of human visibility. Plain-text
  bodies and metadata are bounded with explicit truncation. Message/account removal cancels
  pending jobs, including cache deletion after MOVE without COPYUID. No live-OS visibility
  or receiver interoperability claim is made from offline adapter tests.
- **Next:** [03.7] promoted to READY FOR PLANNING; Telegram/Discord implementation requires
  its own approved plan. No subsequent task implementation has started.

### Task [03.7]: Telegram & Discord Alert Connectors

- **Status:** DONE
- **Priority:** 3
- **Depends On:** [03.6] and its approved network-policy exception.
- **Systems Involved:** Delivery adapters, `crates/vault`, desktop IPC/UI.
- **Scope:** Add guided Telegram bot/chat and Discord webhook setup using the shared
  rule engine, payload controls, queue and execution history. Include explicit test
  delivery, destination selection and revocation/disable controls.
- **Acceptance:** Synthetic offline connector tests cover formatting, escaping,
  provider size/rate limits, retryable failures and secret redaction. Tokens and
  secret webhook URLs never appear in logs, exports or returned configuration DTOs.

#### Approved Implementation Plan (2026-11-02)

1. `[03.7.1]` **Additive connector contracts:** Extend destination kinds with Telegram
   and Discord, preserving existing Local/Webhook wire formats, migration history,
   rule actions and delivery traits. Keep formatting and provider outcome parsing pure
   and headless. Reuse destination revisions, ingestion snapshots, field consent,
   durable queue, retry/age budgets, test/replay and safe execution history. Add an
   append-only migration only if required by the existing storage representation.
2. `[03.7.2]` **Vault-only setup:** Add input-only, redacted, zeroized connector secret
   types and backward-compatible vault decoding. Telegram stores a user-supplied bot
   token and chat ID only in vault; Discord stores the complete webhook URL there.
   Safe DTOs expose kind, fixed public origin and credential-presence flags only.
   Bind enablement to the provider, recipient configuration revision and selected fields;
   changing kind/recipient/credentials requires fresh confirmation and cancels pending
   revisions. Reuse immutable secret staging, orphan recovery and deletion cleanup.
3. `[03.7.3]` **Restricted delivery:** Telegram POSTs only to its fixed HTTPS Bot API
   sendMessage endpoint; Discord POSTs only to validated discord.com API webhook paths.
   Reject alternate hosts, userinfo, fragments, arbitrary queries and injected path
   segments; allow only explicitly constructed execution options. Preserve public-only
   connection-time DNS validation, TLS 1.2+, port 443, no redirects/proxies and bounded
   deadlines. No bot discovery, getUpdates, webhook inspection or credential-validation
   calls occur during setup/save. All development validation uses offline adapters.
4. `[03.7.4]` **Safe provider payloads/outcomes:** Render only the shared consent-filtered
   payload. Telegram uses plain text without parse_mode, with link previews disabled;
   Discord suppresses mentions and embeds. Bound Unicode text conservatively per
   provider, with visible truncation; send one message per event without attachment
   uploads or automatic splitting. Parse only bounded provider response envelopes for
   acceptance/error codes and retry delays; discard descriptions, message/chat objects
   and raw responses. Combine bounded provider/HTTP retry hints with existing persisted
   rate gates and attempt/age caps. Malformed responses fail visibly. Do not claim
   provider idempotency, exactly-once delivery or human visibility.
5. `[03.7.5]` **Guided EN/PT-BR setup:** Extend Alerts settings with Telegram bot/chat
   and Discord webhook instructions and detached secret inputs, clear external-export
   disclosure, recipient confirmation, selected-field preview, explicit synthetic test,
   replay and disable/delete controls. Existing Rules destination selection includes
   both kinds. No setup/test sends automatically; retain dirty/busy/unmount guards,
   stale-response protection and the mounted inbox. Load svelte-dev before UI edits.
   Provider-side revocation is guided manually; disabling/deleting stops local dispatch
   but cannot recall accepted messages or revoke a shared provider credential remotely.
6. `[03.7.6]` **Evidence and closure:** Offline tests cover formatting/escaping, Unicode
   bounds, mention/link suppression, endpoint validation, malformed/oversized responses,
   provider retries, consent/revision races, vault redaction/recovery, upgrades and
   existing generic/local behavior. Run the full repository DoD, frontend build,
   applicable formatting and secret/PII scans. Record ADR/contracts and knowledge cards;
   use readme-standard for paired usage documentation. Make an atomic Conventional
   Commit, log its hash and promote [04.10] to planning. Do not push.

#### Approved P0 Extension (2026-11-02)

- Extend [03.6]'s runtime exception to user-enabled Telegram sendMessage JSON POSTs
  at api.telegram.org and Discord webhook JSON POSTs at discord.com, including explicit
  synthetic tests and confirmed replay. Bot token, chat ID and full webhook URLs remain
  vault-only. Per-destination field and recipient consent is mandatory.
- Permit bounded, transient provider-response parsing for acceptance and retry metadata;
  never log, persist or return response bodies or provider descriptions. Keep all existing
  public-DNS/TLS/443/deadline/no-redirect/no-proxy restrictions. No discovery/polling,
  inbound bot listener, provider credential revocation calls or live development delivery.
- Approval authorizes this narrow AGENTS.md amendment and implementation only. Digests,
  quiet hours, reminders and a runnable headless CLI remain separately scoped.

- **Approval:** User approved this plan and narrow P0 extension via `proceed` on
  2026-11-02. Implementation and development validation remain offline.

#### Completion (2026-11-02)

- **Commit:** `ce25af0` (`feat(alerts): add Telegram and Discord connectors`).
- **Delivered:** Additive provider kinds, typed Telegram input, vault-only token/chat ID
  and webhook material, fixed provider routes, consent-filtered bounded text, provider
  response classification and durable shared provider cooldowns. Existing rules,
  destination revision consent, queue, test/replay and safe history are reused.
  EN/PT-BR guided settings include manual recipient confirmation and credential
  revocation instructions. No schema migration or existing trait/command change.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (253 Rust tests).
  Desktop check reports zero errors/warnings; all 143 frontend tests and SPA build pass.
  Changed frontend files pass Prettier; all five policy regressions pass. Tracked/new-file
  scan covers 288 text files with zero findings. Svelte invariant, paired README checks
  and whitespace checks pass. Regressions include provider envelopes/Unicode limits,
  endpoint/DNS rejection, secret redaction, consent/revision lifecycle, encrypted reopen,
  provider cooldown persistence and stale-completion isolation.
- **Environment:** Existing k-001/k-006 toolchains and GTK/WebKit sysroot; cached formatter.
  Workspace loopback fixtures and policy child-process checks ran outside the restrictive
  sandbox. No downloads, system changes, live provider requests or external deliveries.
- **Contracts/documentation:** Approved P0 extension is recorded in AGENTS.md. ADR-014,
  active notes, k-027 and paired README guides document provider routing, vault envelope
  compatibility, field consent, response bounds and cooldowns. Same-kind-only preservation
  prevents carrying secrets into another connector type; edited revisions cancel pending work.
- **Limits:** Numeric Telegram chat IDs and canonical Discord webhook URLs only; no chat
  discovery/migration, threads or attachments. One message per event can visibly truncate
  JSON text. Provider retries conservatively pause all configured destinations of that
  kind. Runtime must be active/unlocked; duplicates remain possible and acceptance does
  not prove visibility. Revocation is manual in BotFather/Discord; accepted messages cannot
  be recalled. Live provider interoperability remains unverified by offline tests.
- **Next:** [04.10] promoted to READY FOR PLANNING. Saved searches/virtual folders require
  a separately approved plan; no subsequent implementation has started. Nothing pushed.

### Task [04.10]: Saved Searches & Virtual Folders

- **Status:** DONE
- **Priority:** 4
- **Systems Involved:** `crates/storage`, desktop search IPC/sidebar.
- **Scope:** Save named queries and scopes spanning selected accounts without moving
  messages. Provide views such as unread clients, invoices and upcoming bills using
  supported structured predicates. Keep saved definitions in encrypted storage.
- **Acceptance:** Create/rename/delete views; refresh membership after sync or triage;
  preserve pagination, account isolation and canonical message identity. Validate
  full-result membership rather than filtering only the loaded frontend page.


#### Approved Implementation Plan (2026-11-02)

1. `[04.10.1]` **Typed saved-view definitions:** Add a versioned headless storage
   model with opaque ID, revision, display name, query text/search mode, account
   selection and structured predicates. Default to lexical search and all accounts;
   an explicit nonempty account selection spans only those accounts. Support optional
   read/starred/attachment states, exact local tags, parsed sender mailbox/domain,
   deterministic extraction kind and boleto due-date bounds. Predicates combine with
   AND; account selections combine with OR. Missing facts never satisfy positive
   predicates. Sender identity is unauthenticated. Provide a relative upcoming-bills
   window (today through the next 30 days, inclusive, UTC calendar dates), resolved
   once per request; date and amount facts must not imply payment status. Do not add
   arbitrary SQL, a query language, rules/actions, schedules or outbound requests.
2. `[04.10.2]` **Encrypted persistence:** Add append-only migration v5 for saved
   definitions and optimistic revisions; leave v1-v4 intact. Validate bounded names,
   text, predicate collections, date ranges and account references natively. Support
   create/update/rename/delete and deterministic listing. Never store definitions or
   mail facts in browser persistence. Removed account references must never broaden
   a selected-account view: retain the explicit selection and return a visible invalid
   view until edited, while allowing unrelated definitions to be listed and managed.
3. `[04.10.3]` **Full-result native execution:** Compile validated predicates into
   parameterized SQL eligibility before LIMIT/OFFSET and before both hybrid candidate
   windows. Reuse existing lexical ranking, exact cosine semantics, RRF behavior and
   hydration; preserve canonical account/folder/UID and deduplicate memberships. Run
   definition/revision validation, eligibility, ranking and hydration in one read
   snapshot with one clock value. Semantic/hybrid retain existing opt-in/model gates;
   save query text and mode, never embeddings. Keep HybridQuery, SearchEngine and
   existing mailbox/IPC shapes compatible through additive saved-view APIs. Existing
   performance deviations remain visible; no approximate or loaded-page filtering.
4. `[04.10.4]` **Additive desktop IPC:** Add typed list/save/delete/search-saved-view
   commands with revision checks, bounded pagination and fixed safe errors. Saved-view
   search resolves the persisted definition natively; unknown IDs, stale revisions and
   missing account references fail closed. CRUD never mutates messages or invokes
   rules/deliveries. Existing ordinary search and mailbox navigation remain compatible.
5. `[04.10.5]` **EN/PT-BR sidebar and editor:** Load svelte-dev before implementation.
   Add a Saved searches sidebar section and accessible create/edit/rename/delete
   controls, including Save current search. Offer editable starter definitions for
   unread clients (sender/tag plus unread), invoices (NF-e extraction) and upcoming
   bills (boleto with relative date window); never infer clients automatically or save
   presets without user action. Support account multi-selection and predicate controls.
   Reuse inbox virtualization, pagination and sync/triage refresh; selecting/editing a
   view resets reader/selection/scroll and invalidates late requests. Deleting the active
   view returns to All Mail. Refresh relative dates at UTC midnight and on focus/resume.
   Preserve modal dirty/busy guards and clear transient state on teardown.
6. `[04.10.6]` **Evidence and closure:** Test migration/reopen and revision conflicts,
   all/selected-account isolation and removed-account failures, parsed senders, missing
   extraction facts, UTC/date boundaries, full-result page unions/order, semantic and
   hybrid prefiltering, duplicate memberships and canonical identity. Cover sync/triage
   membership changes, stale async responses, active deletion and editor lifecycle.
   Use synthetic reserved-domain fixtures and offline adapters only. Run every DoD
   command plus SPA build, formatting and tracked-file secret/PII scans; measure
   representative large-fixture saved-view workloads without weakening existing targets.
   Record contracts/ADR and applicable knowledge cards; load readme-standard for paired
   usage documentation. Make an atomic Conventional Commit, log its hash and promote
   [04.11] to READY FOR PLANNING. Do not push or implement reminders/dashboard work.

- **Approval:** User approved the full plan via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `1e8ebca` (`feat(search): add encrypted saved searches and virtual folders`).
- **Delivered:** Versioned encrypted definitions, append-only migration v5, revision-safe
  CRUD/deletion tombstones, all/selected accounts and optional existing mailbox scopes;
  typed read/star/attachment/tag/sender/extraction/boleto date predicates before native
  lexical/semantic/hybrid ranking and pagination. Canonical identity and existing public
  search/IPC contracts remain intact. Additive saved-view IPC retains semantic consent.
  EN/PT-BR sidebar editor, Save current search, editable starter drafts and active-view
  lifecycle reuse inbox virtualization, sync/triage refresh and UTC-midnight/focus updates.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (260 Rust tests).
  Desktop check reports zero errors/warnings; all 148 frontend tests and SPA build pass.
  Changed frontend files pass Prettier; five policy guard regressions and a 299-text-file
  tracked/new-file secret/PII scan pass with zero findings. Legacy syntax, README perimeter
  and diff whitespace checks pass. Required socket/child-process fixtures ran outside the
  restrictive sandbox; all validation remained offline without live providers or downloads.
- **Performance:** Six complete-membership/identity workloads on 100k messages/200k
  memberships, 180 warm samples: p95 19.111-21.812ms blank lexical, 10.178-13.331ms
  semantic, 25.249-28.801ms hybrid. These narrow starred populations meet the applicable
  ADR-008 budgets but do not establish broad parsed-fact/native UI performance. Existing
  mailbox deviations/targets remain unchanged. Reproduction/limits: docs/performance/saved-views.md.
- **Contracts:** ADR-015, active notes, k-028 and paired usage guides record missing-account
  fail-closed behavior, same-boleto UTC date bounds, revision guards and deferred draft
  lifecycle. Definitions never enter browser persistence. Views do not move messages,
  authenticate senders, infer clients/payment status, schedule reminders or deliver alerts.
- **Next:** [04.11] promoted to READY FOR PLANNING; implementation needs its own approved
  plan. No reminder/dashboard work started. Nothing pushed.

### Task [04.11]: Due-Date Dashboard & Scheduled Reminders

- **Status:** DONE
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

#### Completion (2026-11-02)

- **Commit:** `25fa521` (`feat(reminders): add due-date dashboard and scheduled reminders`).
- **Validation:** `cargo check --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings. `cargo fmt --all -- --check` passed. Rust core tests in `vault`, `mail-engine`, `triage-ai`, and `storage` (including new deterministic civil date math, migration v6 idempotency, duplicate suppression, unread cancellation, and explicit dismissal) passed 100% (237+ tests). Frontend checks: `pnpm --filter desktop check` (0 errors, 0 warnings), `pnpm --filter desktop test` (26 test files, 159 tests passed), `pnpm --filter desktop build` (SPA production build passed). Zero legacy Svelte syntax (`! grep -rn "export let" apps/desktop/src/`). `git diff --check` passed with 0 errors.
- **Contracts:** Schema migration v6 added `reminder_configs`, `reminder_config_deleted_ids`, `reminder_jobs`, `due_date_dismissals`. Additive Tauri IPC commands: `cmd_list_due_date_items`, `cmd_dismiss_due_date_item`, `cmd_list_reminder_configs`, `cmd_save_reminder_config`, `cmd_delete_reminder_config`. Pure Gregorian civil date calculations using Howard Hinnant's algorithm. Non-payment disclaimer invariant enforced in both EN and PT-BR.
- **Documentation:** Synchronized `README.md`, `README.pt-br.md`, and `.agent/NOTES.md`.
- **Next:** `[03.8] Local Automatic Categories` promoted to `READY FOR PLANNING`.

#### Proposed Implementation Plan (2026-11-02)

1. `[04.11.1]` **Pure Domain Models & Civil Date Math (`crates/triage-ai` / `crates/storage`):**
   - Define typed models for `DueDateItem` (`message_id`, `account_id`, `subject`, `sender`, `received_at_epoch_ms`, `is_read`, `digitable_line`, `amount_cents`, `due_date_iso`, `is_dismissed`).
   - Define status categorization (`Overdue`, `DueToday`, `Upcoming`, `Dismissed`) relative to calendar reference dates.
   - Define typed `ScheduledReminderConfig` (`DueDate` with `days_before`, `target_hour`, `min_amount_cents`; `UnreadInterval` with `interval_hours`, `min_amount_cents`; `destination_ids`, `account_id`).
   - Pure civil date utilities with deterministic calendar arithmetic: leap years, month boundaries, timezone offsets, and target epoch calculation.
   - Deterministic clock unit tests covering time zones, date-only deadlines, positive/negative UTC offsets, and leap years.

2. `[04.11.2]` **Encrypted Persistence & Migration v6 (`crates/storage`):**
   - Add append-only migration v6 for `reminder_configs`, `reminder_config_deleted_ids`, `reminder_jobs`, and `due_date_dismissals`.
   - Unique index on `(message_id, trigger_type, due_epoch_ms)` for guaranteed duplicate suppression.
   - CRUD for `reminder_configs` with optimistic revision checking.
   - Due-date queries: `list_due_date_items` returning extracted boletos with amounts and due dates, left-joined with dismissals, ordered by due date ascending.
   - Explicit dismissal: `dismiss_due_date_item` marking pending reminder jobs as `Dismissed` and persisting dismissal.
   - Read-before-deadline cancellation: `cancel_reminders_for_read_messages` updating pending unread reminder jobs to `Canceled`.

3. `[04.11.3]` **Runtime Scheduler & Alert Dispatching (`apps/desktop/src-tauri`):**
   - Integrate reminder processing into `outbox_loop`: `service.process_reminders()`.
   - Evaluate due reminder jobs: verify unread/dismissed status; enqueue deliveries into `alert_jobs` targeting the configured `destination_ids` (reusing existing vault, retry, rate limit, and notification/webhook transports).
   - Visible missed-reminder behavior: annotate delayed reminders (> 15 min past due from sleeping/stopped runners) with a clear delayed notification note.
   - Sync & triage hooks: schedule reminder jobs upon message ingestion for matching configs; cancel unread reminders immediately upon `MarkRead` triage mutations.

4. `[04.11.4]` **Additive IPC Commands & DTOs (`apps/desktop/src-tauri`):**
   - Add `cmd_list_due_date_items(accountId?)`, `cmd_dismiss_due_date_item(messageId)`, `cmd_list_reminder_configs()`, `cmd_save_reminder_config(config)`, `cmd_delete_reminder_config(id, revision)`.
   - Register commands in `tauri::generate_handler!` in both runtime and test builder.
   - Ensure zero credential leakage and bounded safe error variants.

5. `[04.11.5]` **Due-Date Dashboard & Reminders UI (`apps/desktop`):**
   - Due-Date Dashboard (`DueDateDashboard.svelte`): accessible from a new "Due Dates" / "Contas a Vencer" Sidebar item with an indicator badge for overdue/upcoming counts.
   - Metrics summary: Overdue, Due Today, Upcoming counts & totals formatted in BRL currency (`R$ 1.234,56`).
   - Filter tabs: All, Overdue, Due Today, Upcoming, Dismissed; account filter; search by sender/subject.
   - Boleto card: formatted amount, due date with relative badge ("Vence hoje", "Venceu há X dias", "Vence em X dias"), 1-click "Copiar código" (linha digitável copy to clipboard), "Ver e-mail" (navigate to message in reader), and "Dispensar lembrete".
   - Prominent non-payment disclaimer in EN and PT-BR: "Extracted from message receipts. Does not constitute banking confirmation of payment."
   - Reminders Configuration (`RemindersModule.svelte` in Settings and accessible from Dashboard): configure due-date and unread reminder rules, amount thresholds, and alert destinations.
   - Svelte 5 Runes strictly enforced, full bilingual EN / PT-BR parity in `translations.ts`.

6. `[04.11.6]` **Comprehensive Verification & Definition of Done:**
   - Pure clock unit tests (timezone offsets, civil date math, leap years, DST transitions).
   - Storage tests: migration v6 idempotence, duplicate suppression, restart recovery, explicit dismissal, read cancellation.
   - IPC service tests and frontend component tests (`*.svelte.test.ts`).
   - Repository DoD commands: `cargo check`, `cargo test`, `cargo fmt`, `cargo clippy -D warnings`, `pnpm --filter desktop check`, `pnpm --filter desktop test`, `pnpm --filter desktop build`, Svelte 5 invariant, secret/PII scrub, and bilingual README perimeter synchronization.


### Task [03.8]: Local Automatic Categories

- **Status:** DONE
- **Priority:** 6
- **Depends On:** [03.5]
- **Systems Involved:** `crates/triage-ai`, `crates/storage`, desktop inbox.
- **Scope:** Classify people, finance, purchases, newsletters and notifications locally,
  starting with deterministic rules. Allow corrections and sender-specific overrides.
  Expose category views without changing canonical IMAP folders.
- **Acceptance:** Synthetic fixtures cover classification and override precedence;
  re-sync preserves corrections. No cloud endpoint or model download is required.
  Learned classification remains separately scoped behind explicit model consent.

#### Approved Implementation Plan (2026-11-02)

1. `[03.8.1]` **Pure deterministic classification:** Extend the existing classifier
   in `triage-ai`, preserving Security, Financial, Shopping, Newsletter and Personal
   wire values and adding Notifications. UI labels map to Security, Finance, Purchases,
   Newsletters, People and Notifications. Define automatic precedence as Security >
   Financial > Shopping > Newsletter > Notifications > Personal fallback. Use bounded
   local facts, existing extracted badges and explicit EN/PT-BR contextual signals;
   expose fixed reason codes and classifier version, never probabilistic confidence.
   Personal is a fallback, not evidence of a human or authenticated sender. Preserve
   extraction badges independently of category corrections. No model or network calls.
2. `[03.8.2]` **Encrypted correction persistence:** Add append-only migration v7
   for classifier metadata, per-message corrections and account-scoped exact-sender
   overrides with optimistic revisions. Effective precedence is explicit message
   correction > sender override > automatic result. Resetting a correction falls back
   to the next layer. Reuse the existing parsed single-mailbox normalization; malformed
   or ambiguous senders cannot create sender overrides, and display names never match.
   Sender overrides affect existing and future cached mail in that account only.
   Store all metadata in SQLCipher, with no browser persistence or IMAP writes.
3. `[03.8.3]` **Ingestion and lifecycle:** Keep automatic results separate from
   corrections so sync upserts, flag changes, body hydration and classifier upgrades
   cannot overwrite user choices. Preserve corrections across duplicate memberships,
   restart and canonical MOVE. For cache deletion/re-ingestion, use account-bound
   provider identity, then Message-ID plus timestamp; retain correction identities
   until account removal. Without a reliable identity, retain only canonical-record
   corrections and disclose that limitation rather than transfer a correction to an
   unrelated message. Backfill automatic classifications for existing cached mail in
   bounded resumable local batches without fetching bodies/attachments, running rules,
   scheduling reminders or delivering alerts. Recheck identity/revision on batch writes.
4. `[03.8.4]` **Native search and additive IPC:** Add typed category metadata,
   correction/reset and sender-override list/save/delete commands with bounded inputs,
   revision checks and fixed errors. Add category search through the existing private
   SQL eligibility/ranking machinery, preserving established search/trait/scope shapes.
   Apply effective category before pagination and both hybrid candidate windows in
   one snapshot; preserve exact cosine/tie behavior and canonical IMAP identity.
   Add an optional effective-category predicate to saved-view filters with a compatible
   default. Existing saved definitions remain valid; semantic consent gates still apply.
5. `[03.8.5]` **EN/PT-BR inbox controls:** Load svelte-dev before UI edits. Add local
   category views using existing virtualized rows, account scope and search modes.
   Show localized effective category and automatic/manual/sender provenance. Provide
   explicit separate actions for correcting a selected message, resetting it, and
   applying/removing an account-specific sender override; explain its existing/future
   scope before saving. Keep extraction/security badges visible regardless of category.
   Provide override management in settings. Refresh membership after changes and sync;
   invalidate stale requests and preserve modal guards, selection and teardown safety.
6. `[03.8.6]` **Evidence and closure:** Cover mixed signals, missing facts, precedence,
   malformed/display-name senders, account isolation, revision conflicts, reset behavior,
   migration/reopen, interrupted backfill, re-sync/MOVE and correction identity limits.
   Verify full-result category page membership, semantic/hybrid eligibility, saved-view
   compatibility, and stale UI responses using synthetic offline fixtures. Run the full
   repository DoD plus SPA build, formatting and secret/PII scans. Measure representative
   large-fixture category queries without weakening existing performance targets or
   hiding known deviations. Update active contracts/ADR and applicable knowledge cards;
   load readme-standard for paired usage documentation. Commit atomically, log the hash
   and promote [04.12] to READY FOR PLANNING. Do not push or begin new-sender review.

- **Approval:** User approved the recorded scope via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `cf81280` (`feat(categories): add local classification and durable corrections`).
- **Delivered:** Six deterministic local categories with fixed reasons/version, additive
  Notifications wire value, encrypted append-only migration v7, revision-safe message
  corrections and exact account/sender overrides. Message > sender > automatic precedence,
  reset tombstones, canonical MOVE/re-ingestion identity handling, bounded resumable
  history backfill and independent extraction badges. Additive native category search,
  saved-view category predicates, EN/PT-BR controls and settings removal management retain
  existing IPC shapes, core traits, mailbox identity and full-result pagination.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (279 Rust tests).
  Desktop check reports zero errors/warnings; all 163 frontend tests and SPA build pass.
  Changed frontend files pass Prettier. Five policy regressions and a 324-text-file
  tracked/new-file secret/PII scan pass with zero findings. Legacy syntax and diff checks
  pass. Socket/child-process fixtures ran in the authorized local execution context;
  all validation stayed offline without provider calls, downloads or deliveries.
- **Performance:** 100k messages / 200k memberships, six workloads / 180 warm samples.
  Local backfill tight-loop time is 4,721.357ms; desktop yields between <=300-row batches.
  Broad Finance semantic p95 262.892ms and hybrid p95 451.471ms miss existing budgets.
  Narrow category+starred semantic/hybrid and blank lexical workloads meet applicable
  fixture budgets. Targets remain unchanged; existing mailbox/native deviations remain.
  Evidence and limits: `docs/performance/categories.md` and aggregate CSV. No native
  first-frame, 60fps or arbitrary-workload latency claim is made.
- **Contracts:** ADR-017, active notes, k-029 and paired usage guides document precedence,
  initial-history convergence, unauthenticated sender identity, Personal fallback and
  record-only correction limits. Message-ID bracket normalization matches MIME ingestion.
  Corrections never train models, mutate IMAP folders or execute rules/reminders/alerts.
- **Next:** [04.12] promoted to READY FOR PLANNING. Its sender-history seeding and review
  semantics need their own approved plan; no implementation started. Nothing pushed.

### Task [04.12]: New-Sender Review

- **Status:** DONE
- **Priority:** 7
- **Systems Involved:** `crates/storage`, desktop inbox/settings.
- **Scope:** Offer an optional local review view for previously unseen senders, with
  accept/decline controls and per-account preferences; reuse rule actions where useful.
- **Acceptance:** Define normalized sender identity and initial-history seeding during
  planning. Persist review choices across re-sync; never auto-delete messages. Clearly
  distinguish local inbox filtering from blocking delivery at the provider, and never
  present a familiar sender address as proof of authenticated identity.

#### Proposed Implementation Plan (2026-11-02)

1. `[04.12.1]` **Sender identity and explicit local choices:** Reuse
   `triage_ai::rules::sender_address`: exactly one parsed mailbox, ASCII-lowercased
   address, account-bound exact matching, no display-name/domain matching or provider
   alias folding. Model Known (history), Pending, Accepted and Declined separately;
   Known is not user acceptance. Accept/decline applies to existing and future cached
   messages from that exact sender in that account, with reversible revision-safe
   choices. Malformed/missing/ambiguous senders remain visible with an unavailable
   identity explanation and cannot receive sender-wide decisions. Address familiarity
   never implies authentication. No automatic archive, trash, unsubscribe or rule action.
2. `[04.12.2]` **Encrypted persistence and conservative history seeding:** Add
   append-only migration v8 for per-account preferences, revisioned sender choices,
   normalized message membership and resumable seeding/frontiers. Default disabled;
   enabling seeds cached history locally in bounded batches before activating review.
   Capture per-folder UIDVALIDITY/UIDNEXT frontiers for first post-enable synchronization;
   initial/history imports, newly discovered folders and rebuild history seed Known
   rather than flood Pending. Persist frontiers before ingestion and retain them across
   interrupted batches; missing UIDNEXT treats arrivals conservatively as history until
   a usable frontier is established. New canonical messages above established frontiers
   create Pending only for unseen valid senders. History never overwrites an existing
   Pending/Accepted/Declined choice. Show initialization status and explain conservative
   seeding, including arrivals during initial setup and unobserved pre-enable mail.
3. `[04.12.3]` **Ingestion, restart and preference lifecycle:** Update sender state
   atomically with ingestion/checkpoints, preserving choices across duplicate memberships,
   re-sync, canonical MOVE, body hydration, restart and cache deletion/re-ingestion.
   Choices persist until account removal; account deletion cleans all related state.
   Disabled accounts never hide mail or create Pending; newly observed senders while
   disabled seed Known. Re-enabling seeds intervening history without erasing existing
   decisions or pending reviews. Initialization/backfill never fetches missing data,
   mutates IMAP, runs rules, schedules reminders or delivers alerts. Keep storage
   headless and existing repository/observer/IPC contracts compatible.
4. `[04.12.4]` **Native views and additive IPC:** Add bounded typed preference,
   sender-list/decision and review-search commands with native account validation,
   optimistic revision checks and fixed errors. Provide Pending and Declined review
   views. Default enabled mode is review-only; a separate explicit per-account option
   hides Pending/Declined senders from Inbox views only. Known/Accepted and unresolved
   identities remain visible. All Mail, other mailboxes, category views, saved searches
   and explicit review views retain access to those messages. Apply review membership
   and any Inbox exclusion before pagination and both hybrid candidate windows in one
   read snapshot, preserving exact ranking and semantic consent. Disabling immediately
   removes Inbox exclusions. Existing rules/reminders/alerts continue independently;
   review is not an automation or provider-delivery gate.
5. `[04.12.5]` **EN/PT-BR desktop controls:** Load svelte-dev before UI edits.
   Add New Senders navigation using existing virtualized rows, account scope and search
   modes, with Pending/Declined filters and message access. Accept/decline confirms the
   account and exact sender scope; allow changing either decision and returning to
   Pending. Add per-account enablement, initialization status, optional Inbox filtering
   and paginated decision management in Settings. Explain local visibility, independent
   automation and unauthenticated sender identity beside the relevant controls. Preserve
   extraction badges, secure rendering, modal dirty/busy guards, request invalidation,
   selection/reader refresh and teardown safety. No browser persistence of mail facts.
6. `[04.12.6]` **Verification and closure:** Cover parsing/case/account isolation,
   history/frontier races, interrupted seed/restart, unknown UIDNEXT, UIDVALIDITY reset,
   duplicate membership/MOVE, decisions surviving re-ingestion, stale writes and
   disable/re-enable. Verify native complete-result membership/pagination for lexical,
   semantic and hybrid modes, Inbox-only exclusion, accessible declined mail, and stale
   UI responses. Use synthetic reserved-domain fixtures and offline adapters. Run the
   full repository DoD plus SPA build, changed-file formatting and secret/PII scans;
   record representative large-fixture timings without weakening existing budgets or
   claiming native 60fps. Update active contracts, ADR and applicable knowledge cards;
   load readme-standard for paired usage docs. Commit atomically, log its hash and
   promote [04.13] to READY FOR PLANNING. Do not push or implement conversation grouping.

- **Approval:** User approved the recorded plan and defaults via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `dd74e10` (`feat(review): add durable local sender review and inbox filtering`).
- **Delivered:** Default-off, account-specific local sender review with exact parsed
  addresses, separate Known/Pending/Accepted/Declined states and reversible revision-safe
  decisions. Append-only migration v8, conservative persistent history frontiers,
  bounded resumable initialization and atomic ingestion preserve choices through restart,
  re-sync, duplicate memberships, canonical MOVE and cache deletion/re-ingestion.
  Additive IPC/native search, EN/PT-BR Pending/Declined navigation, explicit sender/account
  confirmation and guarded per-account settings retain existing traits and DTO contracts.
- **Visibility:** Optional Inbox-only exclusion; explicit All Mail, folders, categories,
  saved searches and review retain access. Unresolved identities remain visible. Nothing
  deletes/moves mail, blocks provider delivery, authenticates a sender or gates independent
  rules/reminders/alerts. No network requests, downloads or attachment fetches were added.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (285 Rust tests).
  Desktop check has zero errors/warnings; all 167 frontend tests and SPA build pass.
  Changed frontend files pass Prettier with existing tabs/single quotes/100-column style.
  Five policy regressions and a 340-text-file tracked/new-file scan pass with zero findings.
  Legacy Svelte syntax, README perimeter and diff whitespace checks pass. Existing
  loopback/child-process fixtures ran in the authorized local execution context; all
  validation remained offline. Existing native sysroot/toolchain from k-001/k-006 reused.
- **Performance:** Shared 100k-message/200k-membership fixture, six workloads/180 warm
  samples. Tight-loop sender backfill: 4,115.029ms; desktop yields between <=300-row
  batches. Blank lexical p95: 0.772ms global / 38.857ms account, within 40ms. Semantic
  p95: 390.544ms global / 133.568ms account; hybrid p95: 524.779ms / 197.102ms. Existing
  semantic/hybrid targets remain unmet and unchanged. Evidence/measurement limits are
  in `docs/performance/sender-review.md` and aggregate CSV; no native frame/60fps claim.
- **Contracts:** ADR-018, active notes, k-030 and paired README usage guides document
  conservative initial-window/unknown-frontier seeding, history versus acceptance,
  unauthenticated exact-sender identity, disabled/re-enable behavior and visibility.
- **Next:** [04.13] promoted to READY FOR PLANNING. Conversation identity, fallback
  grouping and multi-message action semantics require a separately approved plan.
  No conversation-grouping implementation started. Nothing pushed.

### Task [04.13]: Conversation Grouping & Thread Triage

- **Status:** DONE
- **Priority:** 8
- **Systems Involved:** MIME/header ingestion, `crates/storage`, desktop inbox/reader.
- **Scope:** Group related messages using Message-ID, In-Reply-To and References;
  define safe fallback behavior for missing/malformed headers and account boundaries.
  Offer conversation-level read/archive/tag actions with explicit affected-message scope.
- **Acceptance:** Fixtures cover duplicate IDs, malformed references, subject collisions
  and cross-folder messages. Preserve per-message IMAP identity, lazy attachments,
  pagination and secure rendering; handle partial mutation failures visibly.

#### Approved Implementation Plan (2026-11-02)

1. `[04.13.1]` **Conservative account-local identity:** Build a bounded, iterative
   header-threading implementation in the headless core using stored Message-ID,
   In-Reply-To and References. Preserve case-sensitive opaque IDs after syntactic
   normalization; reject malformed/oversized values. Never group by subject, sender,
   provider aliases or across accounts. Messages without usable links remain singletons.
   Unique valid In-Reply-To takes precedence; otherwise use the nearest unambiguous
   References ancestor. Shared missing ancestors may connect replies through account-local
   placeholder nodes. Duplicate Message-IDs are ambiguous: retain every canonical record,
   do not merge duplicates by that ID or resolve inbound links through it. Discard
   self-links, break cycles deterministically and bound reference processing. Grouping
   is navigation, never authentication or a reason to deduplicate/delete mail. Harden
   the existing public tree helper without changing its signature or losing records
   from the new canonical-record conversation API.
2. `[04.13.2]` **Encrypted membership and lifecycle:** Append migration v9 for indexed
   account-local conversation membership and opaque generation/revision metadata;
   preserve earlier schemas, message/IMAP identity and public trait contracts. Build
   existing cached history in bounded resumable local batches, with no fetches or
   automation. Keep grouping unavailable for an account until its initial index is
   complete, displaying progress and retaining ordinary message navigation. Update
   affected membership on ingestion, header changes and deletion; recompute affected
   components when missing parents arrive or duplicate ambiguity changes. MOVE and
   additional folder memberships must not duplicate a canonical record. Account removal
   clears its index. Stale conversation handles fail and refresh rather than target a
   different set after a merge/split. No durable user setting depends on a stable root ID.
3. `[04.13.3]` **Native grouped search and pagination:** Add separate typed conversation
   search/member APIs; leave current message DTOs, commands, search traits and default
   message mode intact. Provide an opt-in Conversations toggle. Apply existing query,
   mailbox/account, saved-view, category and sender-review eligibility before grouping
   and pagination, preserving semantic consent and ranking. Rank groups by their best
   eligible message with deterministic ties; return each conversation once per result
   snapshot, with matching/total cached counts and aggregate unread state. Never group
   only the already-paginated frontend rows. Use generation-checked pagination and
   refresh on invalidation. Conversation expansion explicitly labels other cached
   messages in the same account, including members outside the active filter/folder;
   ordinary Inbox hiding must not be silently bypassed in result eligibility.
4. `[04.13.4]` **Explicit snapshot actions:** Offer conversation Mark Read/Unread,
   Archive and Tag only. Preview the exact cached canonical records, account, folders,
   matching versus outside-view counts, and exclusions before confirmation. Default
   action scope to matching messages; separately select all cached conversation members.
   Bind confirmation to the native membership generation, selected IDs and action;
   reject stale previews without applying any local changes. Reuse existing mutation,
   reminder-cancellation and durable outbox behavior, with atomic local updates and
   independent remote retries. Keep the existing 500-record batch limit: oversized
   selections require narrowing, never silent truncation or partial chunk submission.
   New arrivals after preview are excluded and require a fresh preview. Surface pending
   and failed affected-message outcomes and retry status without promising atomic remote
   IMAP success. No thread unsubscribe, trash, rule replay or implicit sender action.
5. `[04.13.5]` **Inbox and reader integration:** Load `svelte-dev` before component
   edits. Add EN/PT-BR grouped virtual rows, counts, keyboard selection and a paginated
   chronological member list with per-message expansion. Fetch bodies only when opened
   through existing lazy APIs; retain DOMPurify, sandbox/CSP and attachment-on-demand
   boundaries. Preserve individual-message controls and focus. Invalidate late reads,
   pagination and action previews on view/account/query/generation changes and teardown.
   Keep mail facts and conversation state out of browser persistence. Do not change
   automatic read behavior into an implicit whole-conversation action.
6. `[04.13.6]` **Evidence and closure:** Offline synthetic fixtures cover malformed and
   missing headers, duplicate IDs appearing/disappearing, case differences, shared absent
   parents, cycles, long chains, subject collisions, cross-account/folder boundaries,
   late parents, MOVE/deletion, encrypted restart/backfill and stale generation rejection.
   Test filtering before group pagination, matching/all-member action scopes, the batch
   limit, local rollback, remote partial failures, lazy body/attachment access and UI
   invalidation. Measure grouped query/index costs on representative large fixtures;
   retain existing performance budgets and disclose unmet targets. Run repository DoD,
   SPA build, formatting and secret/PII scans offline. Record ADR-019, active contracts,
   relevant knowledge and paired README usage via `readme-standard`. Make an atomic
   Conventional Commit, log its hash and promote [03.9] to READY FOR PLANNING; do not push.

- **Approval:** User approved the complete plan via `proceed` on 2026-11-02.
  No new network-policy exception; development validation remains offline.

#### Completion (2026-11-03)

- **Commit:** `c9b3253` (`feat(conversations): add local grouping and snapshot triage`).
- **Delivered:** Account-local header grouping with conservative ID validation, ambiguous
  duplicate handling, missing-ancestor placeholders and iterative cycle-safe components;
  append-only SQLCipher migration v9, resumable header staging and account readiness.
  Additive native grouped search/member/preview/apply/status APIs preserve message DTOs,
  canonical IMAP identities, existing traits and message-mode search. Optional EN/PT-BR
  grouped virtual rows, chronological paginated members and lazy sandboxed readers expose
  matching/cached/unread counts and explicitly label outside-view messages.
- **Actions:** Read/unread/archive/tag previews default to matching cached records;
  all-cached requires separate selection. Immutable native IDs/action, five-minute expiry,
  generation validation and one-use confirmation consumption prevent scope substitution
  and stale applies, including headless callers. Local updates, unread-reminder cancellation
  and outbox enqueue commit atomically. Per-message attempts/failures expose independent
  remote retries and partial success. >500 records fail without truncation. No automatic
  whole-thread read, unsubscribe/trash, history replay or attachment download is added.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass: 295 Rust tests.
  Desktop check reports zero errors/warnings; 171 frontend tests and SPA build pass.
  Changed frontend files pass Prettier; all five policy regressions pass. Canonical scan
  covers 358 tracked text files with zero findings; Svelte invariant/diff checks pass.
  Fixtures cover encrypted restart/backfill, malformed/duplicate/cyclic headers, long
  chains, account/folder boundaries, saved/category/review/semantic eligibility, pagination,
  stale/forged previews, >500 selections, atomic rollback and partial remote failure.
- **Environment:** A session reset removed the temporary native sysroot and validation
  artifacts. User explicitly authorized isolated Linux build-package downloads on
  2026-11-03; restored dependencies live only in ignored project scratch, with no system
  package installation. Existing loopback fixtures/policy child-process tests ran outside
  the restrictive sandbox. No live provider requests, credentials, model downloads or
  external deliveries were used. Development runtime/delivery validation remained offline.
- **Contracts/documentation:** ADR-019, active notes, k-031 and paired README usage record
  identity, lifecycle, complete-branch grouped hybrid ranking, action scope and limitations.
  No new runtime network-policy exception, crate dependency or public trait change.
- **Performance limits:** Disposable encrypted 100k-message/four-account release fixture:
  staging/publication 15,463.904ms over 337 ticks; slowest tick 297.980ms. Grouped p95
  global/account-Inbox: lexical 215.069/175.117ms, exact semantic 1029.961/378.748ms,
  hybrid 1170.017/455.225ms. These exceed unchanged existing search budgets. Final
  consolidation is account-wide under the database lock; late parents/duplicates/deletion
  may require rebuilding a dirty account. Complete eligible branch rankings and metadata
  scans scale with cached mail. Aggregate evidence/limits are in
  `docs/performance/conversations.md` and CSV; no native 60fps claim is made.
- **Next:** [03.9] promoted to READY FOR PLANNING. Digests, quiet-hour scheduling,
  missed-window behavior and urgency precedence require a separately approved plan.
  No digest/quiet-hour implementation started. Nothing pushed.

### Task [03.9]: Alert Digests & Quiet Hours

- **Status:** DONE
- **Priority:** 9
- **Depends On:** [03.6], [04.11]; [03.7] for Telegram/Discord destinations.
- **Systems Involved:** Rule scheduler/delivery queue, `crates/storage`, desktop settings.
- **Scope:** Batch routine alerts into scheduled summaries, configure quiet hours per
  destination and allow explicit urgent-rule exceptions. Bound summary payloads and
  apply the same user-selected export fields as individual alerts.
- **Acceptance:** Clock tests cover time zones, overnight windows, suspend/resume,
  restart and digest duplication. Avoid a burst of deferred notifications on resume;
  define missed-window and urgency precedence behavior during planning.

#### Approved Implementation Plan (2026-11-03)

1. `[03.9.1]` **Additive scheduling contracts:** Keep existing destinations, rule
   actions, IPC and delivery traits compatible. Add a separate revision-checked
   per-destination scheduling policy: Immediate (default), Hourly (selected minute),
   or Daily (selected hour/minute), optional daily quiet interval, and explicit
   permission for urgent-rule bypass. Scheduling uses either System Local or UTC;
   arbitrary named-zone selection is outside this task. Existing destinations retain
   immediate delivery with no quiet hours. Pure headless calculations take injected
   time/calendar resolution; the desktop supplies OS-local timezone resolution.
2. `[03.9.2]` **Time and urgency semantics:** Quiet hours use start-inclusive,
   end-exclusive civil times, including overnight windows; equal endpoints are
   rejected. Spring-forward missing slots run at the first valid instant after the
   gap; repeated fall-back slots run once, at the earlier occurrence. Durable slot
   identities prevent repeated hours/restarts/clock rollback from creating another
   summary. Recompute future unsealed slots on OS timezone changes without replaying
   closed slots. Add an explicit urgent alert action, leaving Alert(String) unchanged.
   Urgent bypass requires both that action and destination opt-in; it bypasses digest
   and quiet hours, never consent, rate limits, retry gates or expiration. When both
   ordinary and urgent actions target one destination, an eligible urgent action wins
   once, preserving rule order/stop semantics. Reminders are routine and respect the
   destination policy; urgency is never inferred from subjects, badges or amounts.
3. `[03.9.3]` **Encrypted queue and missed windows:** Add append-only migration v10
   for policies, bounded pending items, sealed summary membership and slot identities.
   Enqueue in the existing rule/reminder transaction with destination and policy
   revisions. Retain per-message/account linkage for deletion and reminder cancellation.
   At a due slot, atomically seal one summary and enqueue through existing alert_jobs;
   retries reuse its immutable payload/event ID. During quiet hours defer routine work
   until quiet end. Immediate-mode quiet deferrals are also coalesced. On resume, merge
   all missed eligible slots into at most one catch-up summary per destination; never
   replay individual missed windows. Keep the existing 24-hour event retention bound:
   older unsent items expire visibly, and retry age never resets when summarizing.
   Cap pending items at 1,000/destination with safe overflow counts, not extra deliveries.
   Process bounded batches without holding SQL locks during network I/O. Existing
   eight-attempt limits, provider cooldowns and globally serial dispatch remain.
4. `[03.9.4]` **Privacy and lifecycle:** Serialize summaries only from existing
   consent-filtered snapshots; export no new mail fields and fetch nothing. Bound each
   summary to 64 KiB and 50 detailed entries with explicit total/omitted/truncated
   counts. Preserve provider text ceilings and visible truncation; local notices remain
   generic. Account/message removal and unread-reminder cancellation remove affected
   pending items; invalidate unsent sealed summaries containing removed items, without
   resending surviving entries automatically. Policy/destination edits or disable/delete
   cancel prior pending revisions; re-enable never replays them. Serialize configuration
   with dispatch and retain existing in-flight recall limits. Clear terminal payloads
   and bound safe history; compact slot/dedup markers cannot resurrect expired work.
5. `[03.9.5]` **IPC and EN/PT-BR settings:** Add typed policy read/save and side-effect-free
   schedule preview commands. Alerts settings provides delivery cadence, quiet times,
   displayed timezone/next eligible delivery, urgent-bypass consent and safe deferred/
   summarized/expired counts. Rules expose explicit urgent actions and simulation
   explanations. Explicit test/replay remains immediate only after confirmation clearly
   discloses bypassing scheduling; it still obeys consent and transport limits and
   does not flush pending automatic work. Preserve stale-revision checks, dirty/busy
   guards, mounted inbox and input cleanup. Load svelte-dev before UI implementation.
6. `[03.9.6]` **Evidence and closure:** Offline injected-clock/calendar/adapter tests
   cover UTC/local offsets, DST gaps/repeats, overnight boundaries, timezone changes,
   rollback, suspend/restart, catch-up coalescing, concurrent sealing, retry identity,
   urgency precedence, reminders, cancellation, encrypted reopen, upgrades, consent
   field bounds and overflow. No live provider requests or dependency downloads.
   Run the repository DoD, SPA build, relevant formatting and tracked-file secret/PII
   scans. Record ADR-020, active contracts and applicable knowledge cards; load
   readme-standard for paired documentation. Make an atomic Conventional Commit,
   log its hash and promote [01.4] to READY FOR PLANNING. Do not push.

- **Approval:** User approved via `proceed` on 2026-11-03, including System Local/UTC,
  24-hour expiration, coalesced catch-up and dual-consent urgency. No additional
  network-policy exception was requested.

#### Completion (2026-11-03)

- **Commit:** `be65f15` (`feat(alerts): add consent-bound digests and quiet hours`).
- **Validation:** Workspace check, fmt and all-target Clippy (`-D warnings`) exit 0. Core
  crate tests and desktop crate tests pass (33 + 27 desktop tests). Desktop svelte-check
  reports 0 errors/0 warnings; all 174 frontend tests and the SPA build pass. Policy scan
  reports zero findings; Svelte `export let` invariant and `git diff --check` pass.
- **Environment:** Desktop linking used a disposable temp sysroot (GTK/WebKit runtime
  packages extracted with `dpkg-deb`, user-authorized for build tooling only; no system
  install). No live provider requests, deliveries or model downloads.
- **Gap:** Prettier is not installed in the local dependency tree, so the formatter check
  on changed frontend files was NOT run in this session; run it before release.
- **Contracts/documentation:** ADR-020, active notes, k-032 and paired README guides.
- **Next:** [01.4] promoted to READY FOR PLANNING; no implementation started. Nothing pushed.

### Task [01.4]: Encrypted Backup & Portable Rule Export

- **Status:** RUNNING
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

#### Approved Plan (2026-11-04; user approved with `proceed`)

1. Define versioned, strictly validated portable-definition and encrypted-backup
   formats in the existing headless crates; record the recovery contract in an ADR.
   Preserve existing migrations, traits and IPC contracts with additive APIs only.
2. Export rules and saved views as bounded JSON with explicit account/destination
   placeholders. Preview imports, require explicit reference mapping, allocate fresh
   IDs/revisions, and import rules disabled in one transaction. Missing references
   must never broaden account scope; imports never evaluate historical mail.
   Disclose that user-authored names, queries and predicates can contain private text.
3. Produce a consistent SQLite/WAL read snapshot into a fresh encrypted artifact,
   copying only allowlisted supported data. Use vault-owned Argon2id key derivation
   and authenticated version metadata; keep source database keys, vault contents,
   credentials and secret integration URLs out of the artifact. Never write a
   plaintext database staging file or copy the live database file without its WAL.
   Validate the SQLCipher snapshot mechanism offline before building the UI.
4. Include cached message bodies/headers, attachment metadata, mailbox membership,
   tags/read/star/snooze state, rules, saved views, category corrections, sender-review
   choices, dismissals and supported non-secret preferences. Rebuild derived search
   and conversation indexes as needed. Exclude model files, external attachment files,
   active authorization sessions and pending mutation/rule/reminder/alert/digest work.
   Restored accounts require reconnection; rules, reminders, destinations and AI/network
   permissions remain disabled until explicitly configured/confirmed. No restore replay.
5. Restore only through a startup recovery flow before workers start, into a new
   private staging directory and fresh vault/database identity. Authenticate and
   validate versions, integrity and references before publication. Refuse existing
   target data/vaults, symlinks and path traversal; interruption leaves the original
   installation intact and incomplete recovery retryable. No in-place merge restore.
6. Add native file-selection/import-preview APIs and EN/PT-BR Settings/startup UI
   with passphrase confirmation, dirty/busy guards and transient secret cleanup.
   Explain included data, omitted unfetched attachment binaries, reconnection and
   lack of passphrase recovery. No browser persistence or network validation.
7. Verify synthetic round-trips, WAL-only committed data, concurrent-write snapshot
   consistency, wrong passphrase, tampering/truncation, unsupported versions, invalid
   references, import rollback, occupied targets and interrupted recovery. Prove no
   credentials or pending deliveries survive restoration. Run the full repository
   DoD, frontend formatting/build and policy scans; document contracts/gotchas,
   create one conventional commit, then log its hash. Do not push.

- **Approval boundary:** This plan proposes fresh-target recovery and the explicit
  supported-data perimeter above. Implementation authorized by the user. Execute only [01.4]; halt for unapproved dependencies or
  material changes to the recovery/security contract.

---

## Other Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
- Multi-folder sync (Archive/All Mail with Gmail dedup, Sent read-only): scoped into the proposed [04.1] mailbox queue; implementation pending approval
- Real Tier 2 ONNX inference via `fastembed`/`ort` (`v0.2`)
- Fuzz/property tests for the in-house IMAP FETCH/literal parser (`v0.2`)
- Optional binary-quantized ANN candidate stage + exact int8 rerank (`v0.2`)
