# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log`. User requests during conversation
> take precedence — report discrepancies before acting.

---

## Active Task

### 📌 Task [00.2]: CI Pipeline (GitHub Actions)

- **Description:** Add a GitHub Actions workflow running the repository Definition of Done,
  frontend build, runes invariant and secret/PII checks with pinned actions and native deps.
- **Systems Involved:** CI configuration and validation documentation.
- **Action Type:** Configuration and documentation changes.
- **Status:** RUNNING
- **Next Task:** [90.1] Native Performance Validation (`BACKLOG`).
- **Plan:** Approved by the user on 2026-10-31; implementation in progress.

### Approved Plan ([00.2])
1. Inspect lockfiles, native build dependencies and existing validation tooling. Design a
   Linux GitHub Actions workflow with commit-pinned actions, least-privilege permissions,
   frozen dependency installs and no access to application credentials.
2. Run Rust workspace check/test/fmt/Clippy and frontend check/test/build in CI. Add
   deterministic Svelte runes and tracked-text secret/PII checks using synthetic fixtures.
   Keep provider, OAuth, AI and model requests out of validation.
3. Synchronize validation documentation in both READMEs using `readme-standard`; document
   native CI prerequisites and any new local validation commands.
4. Validate workflow configuration, meaningful scanner regressions and the full repository
   Definition of Done with cached dependencies. Respect the two-failure circuit breaker;
   disclose that hosted execution remains unverified until an explicitly requested push.
5. Record evidence and applicable knowledge, create an atomic Conventional Commit, log
   its hash and promote [90.1] to READY FOR PLANNING. No push, release or tag.

---

## Completed Task Evidence

### 📌 Task [04.8]: Persist Tag & Folder Override Rehydration

- **Description:** Extend message summary reads to restore persisted local tags and folder
  overrides after restart.
- **Systems Involved:** Storage, additive desktop DTOs and inbox UI.
- **Action Type:** Source code changes.
- **Status:** DONE
- **Next Task:** [00.2] CI Pipeline (GitHub Actions) (`READY FOR PLANNING`).
- **Plan:** Approved by the user on 2026-10-31; implementation complete.

### Approved Plan ([04.8])
1. Document additive message summary fields for persisted tags and folder overrides. Preserve
   canonical folder/UID identity, existing command envelopes, core traits and database schema.
   Remove the desktop search adapter's substitution of the canonical folder with its override.
2. Rehydrate metadata for all summary search paths from existing encrypted tables with bounded
   reads that preserve ranking and pagination. Keep tags deterministic and duplicate-free.
3. Read the required `svelte-dev` skill, then reconcile authoritative summary metadata with
   optimistic inbox feedback on startup, pagination and sync refresh. Preserve mutation rollback,
   stale-response protection and canonical identity for reader/attachment operations.
4. Add synthetic storage reopen, search-mode, IPC identity and frontend refresh/rollback
   regressions, including remote MOVE completion and override removal.
5. Run the full repository Definition of Done plus frontend formatting/build checks using
   cached dependencies. Respect the two-failure circuit breaker; record any verification limits.
6. Record contracts/evidence and applicable knowledge, create an atomic Conventional Commit,
   log its hash and promote [00.2] to READY FOR PLANNING. No push, release or tag.

### Acceptance and Validation ([04.8])

- [x] Additive `tags` and nullable `folder_override` summary fields rehydrate from existing
  encrypted tables in lexical, semantic and hybrid search. Existing payloads deserialize
  with defaults; command envelopes, schema version and headless core traits are unchanged.
- [x] Parameterized metadata reads batch at most 500 candidate IDs after ranking/pagination.
  Tags remain sorted/unique; tag fan-out cannot duplicate summaries or change page boundaries.
- [x] Desktop search preserves canonical folder/UID. Inbox startup, pagination and accepted
  refreshes restore persisted feedback, while MOVE completion clears obsolete overrides.
  Optimistic changes update row metadata; generation-guarded rollback cannot resurrect old state.
- [x] `cargo check --workspace`, `cargo test --workspace` (159 passed), `cargo fmt --all -- --check`
  and `cargo clippy --workspace --all-targets -- -D warnings` passed with offline dependencies
  and the existing temporary native sysroot. Synthetic socket tests required loopback access.
  Initial sandbox run stopped on denied loopback sockets; a new semantic IPC regression then
  exposed a missing test embedding, which was added before the successful full-suite rerun.
- [x] Frontend check (zero errors/warnings), tests (56 passed), matching-style Prettier and SPA
  build passed. Tests cover visible persisted tags/folder feedback, pagination, override clearing,
  rejected-write rollback, stale reads and search replacement. No component layout changes.
- [x] Tracked text secret/PII scan found zero candidates; Svelte runes/unchecked-any scans and
  `git diff --check` passed. Contracts updated in `NOTES.md`; identity/ranking constraints in
  `k-012`. No CLI flags, environment variables or validation commands changed; no README update.
- [x] Atomic implementation commit `2f14690` (`feat(inbox): rehydrate persisted tags and folder
  overrides`) recorded; [00.2] promoted to READY FOR PLANNING.

### Remaining Limits ([04.8])

- Verification used synthetic messages, encrypted local storage and injected transports;
  live-provider MOVE and native WebKit visual QA were not exercised. Existing blank-query search
  behavior and folder navigation remain outside this task. No external endpoint, push, release
  or tag was requested or performed.

---

## Previous Settings Task Evidence

### 📌 Task [04.7]: Settings & AI Opt-in UI

- **Description:** Add explicit model download and AI consent, vault-backed endpoint settings,
  persistent FTS tokenizer selection and per-message remote-image controls.
- **Systems Involved:** Desktop UI/IPC; additive storage tokenizer methods; LLM proxy policy.
- **Action Type:** Source code changes.
- **Status:** DONE
- **Next Task:** [04.8] Persist Tag & Folder Override Rehydration (`READY FOR PLANNING`).
- **Commit:** `3686430` (`feat(desktop): add settings and explicit AI consent`).

User approved the implementation plan on 2026-10-31. Used the required `svelte-dev` skill at
`/home/<user>/github/ye-sandbox/agent-skills/skills/svelte-dev/SKILL.md`.

### Approved Plan ([04.7])
1. Trace native settings, model caching, LLM clients, tokenizer migrations and reader policy.
   Record additive IPC and encrypted settings contracts in `NOTES.md` before implementation;
   preserve existing wire formats, core traits and the headless Rust boundary.
2. Add accessible Svelte 5 settings for explicit model download consent, disabled-by-default
   Ollama/BYOK configuration and tokenizer selection. Store API keys exclusively through vault;
   keep key inputs transient and expose only redacted status. Saving settings makes no request.
3. Connect explicit model download and per-message summarization actions to native services.
   Explain downloaded artifacts and transmitted message content before consent. No automatic
   downloads, endpoint probes, summaries or retries; use local synthetic network fixtures for QA.
4. Apply tokenizer changes through the existing storage machinery with safe index rebuild and
   persisted selection. Add explicit per-message remote-image controls that reset on message
   identity changes; retain DOMPurify, sandbox, CSP and default resource blocking.
5. Add consent, persistence, vault isolation, stale-response and rendering regressions. Run
   the full repository Definition of Done, frontend formatting/build and responsive UI QA;
   respect the two-failure circuit breaker and record any unverified behavior.
6. Record evidence and applicable knowledge, create an atomic Conventional Commit, log its
   hash and promote [04.8]. No push, release or tag.

---

### Acceptance and Validation ([04.7])

- [x] Settings are accessible from the connected inbox. AI defaults disabled; saving makes
  no provider request. Metadata and endpoint-bound API keys persist atomically in vault.
  Keys stay in transient inputs/invoke arguments, clear on submit/teardown and never return.
- [x] Explicit artifact consent gates bounded direct HTTPS downloads (128 MiB model, 8 MiB
  tokenizer), with no redirects, proxies or automatic retries. Files use the existing XDG
  cache. No download fetcher is installed in background indexing or search.
- [x] Each summary requires a separate confirmation naming destination/model and transmitted
  content. Native dispatch checks the displayed settings snapshot and reads only the selected
  stored subject/plain body. Returned summaries render as text. Existing LLM proxy inheritance
  is disabled; endpoint validation and fixed redacted IPC errors remain.
- [x] Tokenizer changes rebuild the external-content FTS index transactionally, preserve
  message triggers, and rehydrate from table SQL on reopen. No schema-version change.
- [x] HTTPS image permission defaults blocked, applies only to the current message identity
  and resets on selection changes. DOMPurify, sandbox, no-referrer and restrictive CSP remain.
- [x] `cargo check --workspace`, `cargo test --workspace` (158 passed), Rust formatting and
  Clippy (`-D warnings`) passed using cached dependencies and the prepared native sysroot.
  Socket mocks required loopback permission. One new mock's ignored read length was corrected
  after Clippy reported it; Clippy and both affected download tests then passed.
- [x] Frontend check (zero errors/warnings), tests (54 passed), Prettier and SPA build passed.
  Regressions cover consent, vault rehydration/key isolation, malformed-secret error redaction,
  native IPC envelopes, stale settings, selected-content summary dispatch, download transport
  bounds/redirects, duplicate actions, untrusted summary text and image-consent reset.
- [x] Browser QA with actual components and removed synthetic fixture verified settings save,
  explicit summary confirmation/output and layouts at 390x844 and 1200x800 without horizontal
  overflow. The temporary server and browser tab were closed.
- [x] Tracked/new text scan (150 files before this completion log) found zero non-reserved
  email/high-signal secret candidates after excluding package-version syntax. Svelte runes
  and unchecked-any scans passed; `git diff --check` passed.
- [x] Additive IPC contracts documented in `NOTES.md`; settings/rebuild gotchas recorded in
  `k-011`. CLI flags, environment variables and validation commands did not change; README
  synchronization is not required. Atomic implementation commit recorded above.

### Remaining Limits ([04.7])

- Existing embeddings are deterministic projections seeded by cached files, not ONNX inference.
  Settings explicitly disclose this pre-existing limitation; true Tier 2 inference remains debt.
- Live BYOK/Ollama providers, actual hosted model downloads and native WebKit visual behavior
  were not exercised. Synthetic network requests used loopback only. Direct artifact URLs must
  avoid redirects and query strings; many model-hosting share/signed URLs will be rejected.
- Disabling AI prevents future dispatch; it does not retract an already dispatched request.
  No live provider, model host or package registry was contacted. No push, release or tag.

---

## Previous Task Evidence

### 📌 Task [04.6]: Vault Unlock & Account Onboarding UI

- **Description:** Make desktop startup usable end-to-end with vault unlock, account listing,
  and explicit account onboarding for App Password, Public PKCE, BYO OAuth, and Proton Bridge.
- **Systems Involved:** Desktop Svelte UI and native IPC; existing headless authentication helpers.
- **Action Type:** Source code changes.
- **Status:** DONE
- **Next Task:** [04.7] Settings & AI Opt-in UI (`READY FOR PLANNING`).

### Approved Plan
User approved on 2026-10-31. The required skill was located and read at
`/home/<user>/github/ye-sandbox/agent-skills/skills/svelte-dev/SKILL.md`.
1. Trace startup, vault initialization, account DTOs and OAuth helpers; resolve the missing
   mandatory `svelte-dev` skill before authoring components.
2. Add accessible vault unlock, account listing and onboarding screens using typed IPC.
   Keep credentials in transient form inputs only, clear them after submission/teardown,
   and expose fixed redacted errors without browser persistence or logging.
3. Reuse account IPC for App Password and loopback-only Proton Bridge. Complete Public PKCE
   and BYO OAuth through native-owned authorization state, loopback callback and vault storage;
   record any necessary additive IPC contract before implementation. Authorization and token
   endpoint requests require explicit user initiation and configured provider endpoints.
4. Add synthetic local regressions for startup states, credential cleanup, wire contracts,
   OAuth cancellation/state validation and network consent. Run the full Definition of Done,
   frontend formatting/build and UI verification, respecting the circuit breaker.
5. Record evidence and relevant knowledge, create an atomic Conventional Commit, then promote
   [04.7]. No push, release or tag.

### Acceptance Criteria
- [x] Startup resolves vault/account state before opening the inbox; locked and failed states
  offer explicit recovery, and account management remains reachable from the inbox.
- [x] App Password, loopback Proton Bridge, Public PKCE and BYO OAuth use typed native commands.
  Credentials remain transient in UI inputs and are stored only through the native vault.
- [x] Native OAuth owns PKCE, callback validation, token exchange and per-window cancellation;
  a readiness acknowledgement prevents early cancellation races. Errors are fixed/redacted.
- [x] Synthetic regression tests, full workspace validation and frontend validation pass.

### Validation ([04.6])
- Rust workspace compilation, tests (151 passed), formatting and Clippy (`-D warnings`): passed.
- Frontend check (zero errors/warnings), tests (47 passed), Prettier and SPA build: passed.
- Regressions cover startup races, vault recovery, duplicate submissions, transient credential
  clearing on submit/mode change/teardown, all auth wire envelopes, native readiness, late
  acknowledgements, cancellation reconciliation, OAuth state rejection, vault staging cleanup,
  browser failures and both OAuth credential variants. Network fixtures are loopback-only.
- Browser QA used a removed temporary fixture with the actual components. Verified unlock,
  first-account confirmation, explicit OAuth pending/cancel states and cleared credential
  inputs. At 390x844 the document had no horizontal overflow; desktop layout was also checked.
- Restored the authorized temporary native sysroot from hash-verified Ubuntu packages without
  installing system packages. Located the existing Rust toolchain/cache in the older profile;
  Cargo stayed offline. No live mail provider, OAuth endpoint, model or AI service was contacted.
- Added additive OAuth commands/readiness channel to `NOTES.md`; existing commands, core traits
  and SQLCipher schema remain unchanged. Reused cached reqwest/url versions in the desktop crate.
- Recorded Svelte cleanup/default-value and native readiness constraints in `k-010`.
- Svelte runes invariant and tracked/new-file email/high-signal secret scan: passed with zero
  candidates. `git diff --check` passed. No CLI flags, environment variables or documented
  validation commands changed; README synchronization is not required.
- Live-provider authorization and native visual QA remain unverified. Automatic OAuth token
  refresh remains unimplemented and is disclosed in the onboarding UI. Abrupt process exit can
  leave an unused encrypted staging entry in vault (see `k-010`). No push, release or tag.
- Implementation committed as `79ebb4c`. Task complete; [04.7] promoted to `READY FOR PLANNING`.

---

## Earlier Task Evidence

### 📌 Task [04.5]: Desktop Security Boundary Audit and Targeted Hardening

- **Description:** Audit email rendering, attachment downloads, native IPC input validation, credential isolation, and explicit network-action gating. Reproduce confirmed defects with synthetic local fixtures and apply narrowly scoped fixes preserving active contracts.
- **Systems Involved:** `apps/desktop`; existing core security boundaries only where a confirmed desktop path requires investigation.
- **Action Type:**
  - [ ] Read-only / Documentation
  - [x] Source code changes
- **Status:** DONE
- **Next Task:** [04.6] Vault Unlock & Account Onboarding UI (`READY FOR PLANNING`).

### Approved Plan
User approved on 2026-10-30.
User subsequently authorized native build dependency downloads into a temporary sysroot.
1. Trace untrusted email content and IPC inputs through rendering, downloads, vault access, and network-action dispatch; use local files and synthetic fixtures only.
2. Record reproducible findings with severity and affected paths. Fix confirmed defects within the existing IPC/schema/trait contracts and add focused regression coverage.
3. Run the repository Definition of Done and frontend formatting/build checks with cached dependencies; respect the two-failure circuit breaker.
4. Record evidence and remaining limitations, then create an atomic Conventional Commit. No push, release, or tag is included.

### Acceptance Criteria
- [x] Audit findings reference concrete paths and reproduction evidence; distinguish confirmed defects from unverified concerns.
- [x] Confirmed in-scope defects receive targeted fixes and meaningful regression coverage.
- [x] Preserve secret isolation, sandboxed rendering, lazy attachments, explicit network consent, and active IPC/schema/trait contracts.
- [x] Complete and record the full repository Definition of Done.

### Audit Evidence ([04.5])

- **Confirmed high-severity defect:** `crates/mail-engine/src/unsubscribe.rs` accepted
  syntactically public HTTPS hostnames without validating their resolved addresses. A sender's
  RFC 8058 endpoint could resolve to a non-public destination. The regression injects a
  loopback answer for `unsubscribe.example.org`: URI validation accepts it, while the fixed
  reqwest connector rejects it before connection.
- Added a private connection-time resolver that rejects empty/non-public/mixed address sets
  and supplies the same validated addresses to the connector. Disabled inherited proxies so
  proxy-side DNS cannot bypass this policy. Redirect blocking, HTTPS verification, explicit
  unsubscribe dispatch, and the timeout remain in place. See `k-009` for conservative address
  exclusions and limitations.
- Inspected the DOMPurify allowlist/CSP and empty sandbox reader, attachment metadata and
  UIDVALIDITY checks, credential redaction/vault rollback, and stored-header unsubscribe
  dispatch. Existing regression coverage passed for the frontend and core boundaries; this
  bounded source audit is not a release certification or live-provider penetration test.
- Workspace compilation and Clippy (`-D warnings`), Rust formatting, frontend typecheck
  (zero errors/warnings), all 36 frontend tests, and the SPA build passed.
- All 127 core Rust tests passed with offline dependencies and loopback socket access.
  Four new regressions cover address classification, mixed/empty DNS, test-mode isolation,
  and the actual HTTP connector. No external DNS, provider, AI, or registry request was made.
- Svelte runes invariant and tracked/new-file email/high-signal secret-pattern scan
  (126 files, zero candidates) passed. `git diff --check` passed.
- **Native validation blocker resolved:** Initial workspace testing could not link GTK and
  WebKitGTK. With explicit user authorization, downloaded Ubuntu development packages and
  extracted a temporary sysroot without installing or upgrading system packages. Real pkgconf
  metadata and repaired development symlinks resolved the native libraries. Full
  `cargo test --workspace` passed: 145 tests, including native IPC wire/service tests.
  Local socket mocks ran with authorized loopback access. Cargo dependencies remained offline;
  only the authorized OS package download contacted external servers.
- No release, tag, or push. No project dependency, schema, IPC, trait, CLI flag, environment
  variable, or canonical validation-command changes were introduced. Temporary build paths
  remain outside the repository.
- Workspace check and Clippy (`-D warnings`) also passed against the prepared native
  sysroot. Implementation and security documentation committed as `75599f1`.
- Task complete. Post-audit gap analysis registered [04.6]–[04.8], [00.2], [90.1] in Backlog.

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
| [04.5] | Desktop Security Boundary Audit and Targeted Hardening | `75599f1` | 2026-10-30 |
| [04.6] | Vault Unlock & Account Onboarding UI | `79ebb4c` | 2026-10-31 |
| [04.7] | Settings & AI Opt-in UI | `3686430` | 2026-10-31 |
| [04.8] | Persist Tag & Folder Override Rehydration | `2f14690` | 2026-10-31 |


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

Registered 2026-10-31 from post-[04.5] gap analysis. Each task requires its own plan approval.

### [04.7] Settings & AI Opt-in UI — `DONE`
- **Gap:** Tier 3 LLM summarizer has no IPC surface; Tier 2 model download has no consent UI.
- **Scope:** Explicit opt-in for embedding model download (`$XDG_DATA_HOME/triage-mail/models`);
  Ollama/BYOK endpoint config (keys → `vault`); FTS5 tokenizer choice (`unicode61`/`trigram`);
  per-message remote-image unblock preference.
- **Constraints:** Zero network calls without explicit opt-in (P0).

### [04.8] Persist Tag & Folder Override Rehydration — `DONE`
- **Gap:** DTOs do not expose saved tags/folder overrides; UI feedback is session-local ([04.4]).
- **Scope:** Extend message summary DTOs and storage reads; rehydrate UI after restart.
- **Constraints:** P1 IPC/schema change — record contract update in `NOTES.md` before implementing.

### [00.2] CI Pipeline (GitHub Actions) — `READY FOR PLANNING`
- **Scope:** Workflow running full DoD: cargo check/test/fmt/clippy (`-D warnings`), pnpm
  check/test/build, Svelte runes invariant, secret/PII scan, `git diff --check`.
- **Constraints:** No secrets in workflow; pinned action versions; GTK/WebKitGTK deps documented
  (see `k-006`); resolve offline frozen pnpm install question (`k-008`).

### [90.1] Native Performance Validation — `BACKLOG`
- **Gap:** 60fps evidence is browser-fixture only; no native WebKitGTK measurement.
- **Scope:** Native WebKitGTK frame timing with 100k messages; hybrid search `<15ms` benchmark
  at 100k rows over SQLCipher; release binary size and idle RAM baseline.

---

---


## Release / Cycle Wrap-up (Not the next task)

Release/tag only with explicit human request. When triggered, the ID is `[99.1]`
(public-transition audit: git-history secret scan, AGPLv3 compliance, README sync, `v0.1.0` tag). Do not number feature, hygiene, or CI tasks as `99.x`. Do not calculate next task ID from this section.

---

## Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Post-triage webhook & automation consumers implementing `TriageEventHook`
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
