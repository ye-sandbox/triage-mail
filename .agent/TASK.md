# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log`. User requests during conversation
> take precedence — report discrepancies before acting.

---

## Active Task

### 📌 Task [01.3]: Pre-Release Schema Consolidation & Persisted Triage

- **Description:** ADR-006 duplicated the vector index solely to preserve an unreleased v4 schema.
  Triage results are recomputed per detail request; list badges transfer full bodies over IPC.
  Collapse migrations v1–v5 into a single clean v1 schema with one account-partitioned
  `vec_messages` (`account_id` PARTITION KEY); drop the derived table, reconciliation triggers and
  backfill. Persist triage category/badges at ingestion and expose them in `MessageRowDto`.
  Verify global (unfiltered) vec0 KNN over partitions at 100k against the exact reference oracle.
- **Systems Involved:** `crates/storage` (migrations, schema, search), `crates/mail-engine` (ingestion),
  `apps/desktop` (DTOs, triage persistence, list badges).
- **Action Type:** Implementation; requires an approved plan.
- **Status:** READY FOR PLANNING
- **Scope:**
  - Collapse migrations v1–v5 into a single canonical v1 schema with one account-partitioned vector index.
  - Drop derived table, reconciliation triggers, and migration backfill logic.
  - Persist triage category and badges at ingestion in SQLCipher; expose additively in `MessageRowDto`.
  - Validate 100k global and filtered search against the exact reference oracle.
- **Governance:** ADR-007 supersedes ADR-006. P1 schema immutability applies to released schemas; from tag `v0.1.0` migrations are append-only.
- **Constraints:** P0 secrets isolation, strict typing, P1 IPC additive backwards-compatibility. Dev databases recreated from IMAP sync.
- **Origin:** v0.1.0 readiness review (2026-10-31), findings C1 and I2.

---

## Completed Tasks

### 📌 Task [03.4]: Tier 2 Honest Labeling (Experimental Semantic Search)

- **Description:** Mark Tier 2 experimental; hide Semantic/Hybrid modes behind an explicit setting;
  correct UI search copy to honestly label feature-hash semantic projection vs learned ONNX models;
  and expose embedding failure telemetry in the sync worker instead of silently discarding errors.
- **Systems Involved:** `crates/triage-ai`, `apps/desktop` (settings, inbox search modes, sync worker).
- **Action Type:** Implementation.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; implementation and validation complete.
- **Commit:** `96c52a9` (`feat(triage-ai): label experimental semantic search and expose sync embedding telemetry`).
- **Next Task:** [01.3] Pre-Release Schema Consolidation & Persisted Triage (`READY FOR PLANNING`).
- **Acceptance:**
  - [x] Honest doc comments and module definitions in `crates/triage-ai/src/embeddings.rs` and `lib.rs` explicitly labeling Tier 2 as experimental deterministic feature-hash projections into `int8[384]` (not neural ONNX inference; learned ONNX deferred to v0.2).
  - [x] Stored settings and `SettingsDto` in `apps/desktop/src-tauri/src/settings.rs` include `experimental_semantic_search: bool` (default disabled/false), persisted atomically in vault under `ai-settings`.
  - [x] Added `cmd_set_experimental_semantic_search(enabled: bool)` IPC command and registered in Tauri command handlers.
  - [x] Backend guard in `DesktopService::search_messages` rejects `Semantic` and `Hybrid` queries with `IpcError::InvalidRequest` when `experimental_semantic_search` is disabled.
  - [x] `SyncProgressEvent` includes `#[serde(default)] pub embedding_errors_count: usize` and `BatchEvents::on_embedding_failed` tracks batch embedding error counts.
  - [x] Sync worker in `worker.rs` matches `provider.embed_int8(&text)` errors, invokes `events.on_embedding_failed`, and logs non-silent diagnostics instead of silently swallowing errors.
  - [x] Settings UI (`SettingsView.svelte`) provides an "Experimental Semantic Search" toggle with explicit honest disclosures regarding feature-hash projections vs learned neural ONNX models.
  - [x] Search toolbar (`SearchToolbar.svelte`) hides match dropdown by default and, when enabled, displays honest labels: `Words`, `Semantic projection (Experimental)`, `Words + semantic projection (Experimental)`.
  - [x] `InboxController` clamps search mode to `Lexical` when disabled, surfaces indexing warning telemetry when `embedding_errors_count > 0`, and updates error messaging.
  - [x] Both `README.md` and `README.pt-br.md` synchronized following `readme-standard` skill to reflect experimental feature-hash semantic projection.
  - [x] Full test suite passes: 169 Rust workspace tests, 58 Vitest tests, `svelte-check` 0 errors/0 warnings, `cargo clippy` 0 warnings, formatting checks, policy scanner (176 text files, 0 findings), `git diff --check`.

### 📌 Task [02.5]: OAuth Token Refresh

- **Description:** Handle expired OAuth access tokens gracefully. Currently only `refresh_token` is
  stored and access tokens expire (~1h), causing background sync workers to loop indefinitely in
  `Retrying` state.
- **Systems Involved:** `crates/mail-engine` (auth, protocol, refresh transport); `crates/vault` (atomic update); desktop sync worker.
- **Action Type:** Source code changes.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; implementation and validation complete.
- **Commit:** `d65416d` (`feat(mail-engine): handle oauth token refresh and clean orphaned staging`).
- **Next Task:** [03.4] Tier 2 Honest Labeling (Experimental Semantic Search) (`READY FOR PLANNING`).
- **Acceptance:**
  - [x] Pure-Rust `TokenRefresher` and `TokenRefreshTransport` trait in `crates/mail-engine/src/auth.rs`.
  - [x] `HttpTokenRefreshTransport` enforces strict P0 network policy (HTTPS scheme only, <=2048 chars, no query/fragment/credentials, no proxy, no redirects, 30s timeout, <=64KB body limit).
  - [x] SASL XOAUTH2 challenge abort: `protocol.rs` detects `+ ` continuation lines on `AUTHENTICATE` commands, writes `\r\n` CRLF abort sequence, and surfaces `EngineError::AuthenticationFailed`.
  - [x] `DefaultImapConnector` intercepts authentication failure, refreshes token via `TokenRefresher`, updates vault atomically via `SecretStore`, establishes a fresh TLS session, and retries authentication once.
  - [x] `AccountAuthMaterial::OAuth2PublicPkce` and `OAuth2ByoCredentials` store optional `token_endpoint: Option<String>` with backwards-compatible deserialization and secret redaction.
  - [x] Onboarding persists `token_endpoint` during authorization and uses deterministic staging key `VaultKey::for_account("staging", "oauth-pending")`.
  - [x] `DesktopService::new` cleans orphaned staging vault entries at startup (`k-010`).
  - [x] Full workspace verification passes: 100% tests pass (44 mail-engine unit tests, 10 imap connector tests, 18 desktop tests, 15 vault tests), clippy zero warnings, fmt check passed, svelte-check 0 errors, vitest 56 passed, git diff clean, zero PII.

### 📌 Task [02.4]: Lazy Body Fetch via BODYSTRUCTURE & Batched Initial Sync

- **Description:** Fix the Lazy Attachment invariant violation in bulk sync. `fetch_new_messages`
  (`crates/mail-engine/src/protocol.rs`) requested `BODY.PEEK[TEXT]`, fetching full MIME with base64
  attachments, and issued unbounded `UID FETCH 1:*` buffering entire mailboxes in memory.
- **Systems Involved:** `crates/mail-engine` (protocol, sync, MIME assembly); desktop worker batching.
- **Action Type:** Source code changes.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; implementation and validation complete.
- **Commit:** `5124327` (`feat(mail-engine): lazy body fetch via bodystructure and batched sync`).
- **Next Task:** [02.5] OAuth Token Refresh (`READY FOR PLANNING`).
- **Acceptance:**
  - [x] Zero-unsafe RFC 3501 `BODYSTRUCTURE` parser (`crates/mail-engine/src/bodystructure.rs`) extracts `AttachmentMeta` without downloading payload bytes and identifies non-attachment leaf text parts (`text/plain`, `text/html` $\le 2\text{ MiB}$).
  - [x] Two-Phase fetch in `protocol.rs` and `sync.rs`: Phase 1 (`UID FETCH ... BODYSTRUCTURE BODY.PEEK[HEADER]`) and Phase 2 (`UID FETCH ... BODY.PEEK[1]` or `BODY.PEEK[<sec>.MIME] BODY.PEEK[<sec>]`). Wire command strictly never contains `BODY.PEEK[TEXT]`, `BODY[]`, or `RFC822`.
  - [x] Initial sync batched with `DEFAULT_SYNC_UID_BATCH_SIZE: u32 = 300` in `connector.rs` and `worker.rs`.
  - [x] Each batch commits messages and watermark (`FolderSyncState.uidnext`) sequentially to SQLCipher before requesting next batch, surviving process interruption.
  - [x] Replaced `literal.clone()` in `parse_fetch_response_line` with buffer moves.
  - [x] Misleading invariant comments corrected in `protocol.rs` and `lib.rs`.
  - [x] Integration tests pass: `lazy_bodystructure_sync_test.rs` verifies wire command and 50MB attachment bypass; worker batching tests verify sequential commit and interruption resumption.
  - [x] Full workspace test suite passes (58 mail-engine tests, 18 desktop tests, 168+ total tests).


---

## Completed Search Latency Evidence

### 📌 Task [90.4]: Remaining Exact Search Latency

- **Description:** Profile and reduce remaining global vector/FTS ranking and hybrid latency
  while preserving exact eligible neighbors and deterministic fixed-request ranking.
- **Systems Involved:** Storage/search; headless core boundaries remain unchanged.
- **Action Type:** Implementation; requires a separately approved plan.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; implementation and validation complete.
- **Commit:** `8482a78` (`perf(search): streamline ranking CTEs and eliminate dead query parameters`).
- **Next Task:** [99.1] Public Transition Audit & Secret Scrubbing Verification (`READY FOR PLANNING`).
- **Evidence:** [90.4] report and raw samples in `docs/performance/native-validation.md` and `docs/performance/search-100k-latency.csv`.
- **Constraints:** Preserve v5 index consistency, encryption/durability, exact coverage and
  tie/deep-page behavior. No approximations, stale-result caches or unsafe-code expansion.
  Distinguish warm/reopened results and account for memory and migration costs.

### Approved Plan ([90.4])

1. Measure fine-grained breakdown of FTS5 ranking, global vector search, and hybrid pipeline.
2. Remove unused dead parameters/constraints in `knn_candidates`, `exact_semantic_ids`, and `lexical_ids`.
   Drop `MATERIALIZED` from the lexical ranking CTE to avoid intermediate temp-table writing while
   preserving deterministic BM25 score and rowid tie-breakers. Pre-allocate RRF fusion maps and
   switch summary metadata joins to `prepare_cached`.
3. Validate exact neighbor fidelity and ordering against the independent integer-dot-product reference
   oracle, ensuring 100% complete pages (200 rows) with zero underfilled or truncated results.
4. Rerun the 100k SQLCipher benchmark (`search_performance`) measuring warm and reopened distributions
   across all 17 cases.
5. Run full workspace check, tests, fmt, clippy, and frontend check, test, and build.
6. Record knowledge card k-017, update NOTES.md, create atomic Conventional Commit, and log hash.

### Acceptance and Validation ([90.4])

- [x] Streamlined lexical CTE drops `MATERIALIZED`, saving ~2-3ms of temp-table materialization.
  Dedicated query branches for account-filtered vs global queries eliminate dead parameter binding
  (`?2 IS NULL` and `(?2 IS NULL OR account_id=?2)`).
- [x] Dedicated prepared statements in `knn_candidates` and `exact_semantic_ids` eliminate dead
  nullity constraints and parameters while preserving exact partition pruning and cross-join semantics.
- [x] Pre-allocated RRF candidate maps (`HashMap::with_capacity(candidates.limit * 2)`) and cached
  prepared statements in `hydrate_metadata`.
- [x] 100k release benchmark passed all 595 samples across all 17 cases with 100% exact result counts.
  Every semantic page matches the independent integer-dot-product reference ranks with zero ordering
  or candidate divergence.
- [x] Warm filtered semantic p50/p95 improved to **11.822 / 12.176ms** (within <15ms target).
  Common lexical warm p50 improved from 33.704ms to **31.930ms** (-5.3%); lexical page2 from
  33.571ms to **31.185ms** (-7.1%); global semantic common from 43.650ms to **42.697ms** (-2.2%).
- [x] Full workspace check, 168 Rust tests (including native IPC and storage suites), format check,
  and Clippy with `-D warnings` passed with 0 warnings.
- [x] Frontend check (0 errors, 0 warnings), 56 unit/component tests, and static SPA build passed.
- [x] Svelte runes invariant, 5 policy scanner regressions, 172 tracked text files secret scan,
  and `git diff --check` passed with 0 findings.
- [x] Knowledge card `k-017` and `NOTES.md` updated with exact search formulation contracts.
- [x] Atomic implementation commit `8482a78` recorded; [99.1] promoted to READY FOR PLANNING.

### Remaining Limits ([90.4])

- Complete global vector scans over 100,000 int8[384] embeddings physically require ~42ms warm on a
  single thread due to 38.4MB linear memory scanning and page decryption.
- 99,000-match lexical queries require ~30ms in SQLite FTS5 for BM25 calculation and sorting.
- Hybrid search executes lexical and semantic branches sequentially on a single SQLite connection,
  resulting in ~75ms warm p50. Sub-15ms global search would require approximate ANN indexing or
  multi-threaded connection pooling, which were excluded to preserve exact correctness and ACID safety.

---

## Completed Partition Optimization Evidence

### 📌 Task [90.3]: Account-Partitioned Search Optimization

- **Description:** Evaluate account-partitioned vector storage/indexing and remaining lexical
  ranking costs to pursue <15ms search at 100,000 messages with complete, exact results.
- **Systems Involved:** Storage/search; desktop adapter only if an approved plan requires it.
- **Action Type:** Implementation; requires a separately approved plan.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; implementation and validation complete.
- **Commit:** `e40056a` (`perf(search): partition account vectors and tune page cache`).
- **Next Task:** [90.4] Remaining Exact Search Latency (`READY FOR PLANNING`).
- **Evidence:** [90.2] corrected samples/report in `docs/performance/native-validation.md`.
- **Constraints:** Any additive schema migration needs explicit plan approval and contract
  documentation. Preserve account isolation, exact neighbor coverage, stable tie boundaries
  and metadata hydration. No approximate/empty/underfilled-result substitution.

### Approved Plan ([90.3])

1. Profile the corrected 100k workloads with synthetic local data. Separate encrypted page
   reads/cache misses, vector computation/top-k, FTS ranking and final-page hydration. Compare
   bounded connection-local page-cache sizes and record memory costs; do not weaken encryption,
   durability, exact ranking or result coverage to improve timing.
2. Authorize additive migration v5: create `vec_messages_by_account` with a text message-ID
   primary key, opaque account-ID partition key and int8[384] cosine vectors. Preserve existing
   `vec_messages` as the authoritative compatible source; backfill only vectors joined to live
   messages in the migration transaction. Do not alter existing tables, DTOs or trait signatures.
   Record the contract in NOTES.md before implementation, and the decision/trade-off in ADRs.
3. Maintain the derived partition index transactionally with embedding writes and message
   insertion/deletion/invalidation paths. Keep orphan vectors out of the index. Verify migration
   from v4, idempotence, rollback, reopen, replacement, missing embeddings and folder/UIDVALIDITY
   purges. Update synthetic benchmark seeding to populate the same index faithfully.
4. Use partition-constrained vec0 KNN for account-filtered search, retaining exact tie-boundary
   expansion and scalar fallback for oversized ties/deep pages. Preserve fixed-request ID ties,
   RRF formula/candidate windows, global search semantics and final-page metadata hydration.
5. Apply measured, bounded page-cache/query-plan improvements to global vector and lexical
   paths. Keep tokenizer semantics, ranking and tie membership unchanged; preserve both supported
   FTS tokenizers. If the measured target remains unmet, disclose the gap rather than substituting
   approximate, cached-stale, empty or underfilled results.
6. Repeat comparable 100k SQLCipher benchmarks with full counts and independent exact-neighbor
   checks. Include skewed/sparse accounts, second pages and migration/index-growth measurements;
   separate new cases from existing baseline comparisons. Record p50/p95, memory/cache settings,
   database growth and migration cost; distinguish warm from reopened (not OS-cold) samples.
7. Run the full repository Definition of Done and frontend build using cached dependencies and
   authorized local mock/subprocess access. Existing permission covers restoring Linux build
   packages only if needed; no registry/provider/AI/model requests. Respect the circuit breaker.
   Synchronize READMEs via readme-standard if documented commands/configuration change.
8. Save raw results and applicable knowledge, make an atomic Conventional Commit and log its
   hash. Promote remaining work explicitly; no push, release or tag, and no unrelated UI changes.

### Acceptance and Validation ([90.3])

- [x] Approved additive v5 contract was recorded before implementation. Migration preserves
  authoritative vec_messages, backfills live vectors only, and rolls back index/schema changes
  together. Existing DTOs, trait signatures, tokenizer semantics and unsafe-code boundary remain.
- [x] Embedding replacement maintains both indexes atomically; invalid dimensions preserve prior
  vectors. Message insert/account-change/delete triggers reconcile partition membership, including
  late vectors, UIDVALIDITY reset and account cascades. Encrypted v4 upgrade survives reopen.
- [x] Account KNN constrains partitions before top-k; exact reference ordering, missing/orphan
  vectors, 4100-way ties and deep pages are preserved. RRF windows/formula and hydration remain.
- [x] Measured cache component queries at 2/16/64MiB; selected a 64MiB soft connection target.
  Recorded process RSS with allocator-retention limits; no unsupported isolated-memory claim.
- [x] Final release benchmark passed 595 complete-page/reference samples, including original
  comparable cases, second filtered pages, 31-vector sparse and empty accounts. Cache profiling,
  migration time, database growth and all raw samples are committed with methodology.
- [x] Warm semantic-filtered p50/p95 is 12.041/12.928ms versus 380.273/391.899ms; hybrid-filtered
  p50 is 37.482ms versus 426.470ms. Global/lexical/hybrid and reopened targets still miss <15ms.
  Backfill took 1296.317ms in one sample and added 44.11MiB to the encrypted fixture.
- [x] Workspace check, 168 Rust tests, format check and all-target Clippy with -D warnings passed.
  Frontend check had zero errors/warnings; all 56 tests and the ordinary SPA build passed.
- [x] Five policy regressions, tracked-text/runes scans and whitespace checks passed with local
  authorized subprocess/mock access. No external requests or new dependencies were needed.
  k-016 preserves partition/subtype pitfalls; ADR-006 records the approved trade-off. No CLI flags,
  environment settings or validation commands changed, so README perimeter remains unchanged.
- [x] Atomic implementation commit `e40056a` recorded; [90.4] promoted to READY FOR PLANNING.

### Remaining Limits ([90.3])

- Only the observed warm first filtered semantic page meets <15ms among substantive vector cases.
  Its reopened median is 34.848ms; second filtered page warm median is 16.099ms. Global semantic,
  FTS ranking and hybrid calls still miss the target. Exact tie/deep-page fallback remains slower.
- The duplicate index costs encrypted disk and ingestion work; peak migration WAL/disk usage and
  production ingestion/memory were not measured. Older writers must not write the v5 database.
- RSS measurements include allocator retention after synthetic seeding; cache_size is a soft
  per-connection target. No resident-cache bound, OS-cold measurement or UI/IPC performance claim.
- No push, release/tag, hosted CI, real accounts/providers, AI calls or model downloads occurred.

---

## Completed Search Task Evidence

### 📌 Task [90.2]: Search Correctness and Performance

- **Description:** Restore ordinary persisted-inbox retrieval, correct account-aware vector
  candidate selection and address measured search latency at 100,000 messages.
- **Systems Involved:** Storage/search and the existing desktop search adapter.
- **Action Type:** Implementation; requires a separately approved plan.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; implementation and validation complete.
- **Commit:** `c14f6fa` (`fix(search): restore inbox listing and exact account neighbors`).
- **Next Task:** [90.3] Account-Partitioned Search Optimization (`READY FOR PLANNING`).
- **Evidence:** [90.1] report in `docs/performance/native-validation.md`; raw samples included.
- **Constraints:** Preserve existing IPC/schema/core trait contracts unless an explicit plan
  approves an additive change. Separate correctness assertions from timing observations.

### Approved Plan ([90.2])
1. Restore blank lexical inbox requests as newest-first persisted listings with a stable ID
   tie-breaker, bounded pagination and existing tag/folder metadata. Cover empty/whitespace
   input, equal timestamps, account isolation and restart persistence with regressions.
2. Correct semantic candidate selection so account restrictions apply before top-k selection.
   Verify complete eligible pages and nearest-neighbor ordering against a deterministic
   reference, including second pages, sparse accounts and ties. Preserve IPC payloads,
   schema versions and core trait signatures; escalate any required additive schema change.
3. Profile existing 100k SQLCipher workloads and optimize measured query/ranking/hydration
   bottlenecks without reducing candidate coverage or changing the RRF scoring formula.
   Make tie ordering deterministic and avoid hydrating discarded hybrid candidates where safe.
4. Rerun the existing release benchmark with stronger correctness assertions and separate
   timing observations. Preserve baseline samples and document before/after distributions,
   result counts and remaining limits; do not claim the <15ms target unless measured.
5. Run the full repository Definition of Done and frontend build using cached dependencies
   and synthetic local fixtures, respecting the two-failure circuit breaker. Synchronize
   READMEs with readme-standard only if documented commands or configuration change.
6. Record applicable invariants/knowledge, create an atomic Conventional Commit and log its
   hash. Promote any remaining follow-up explicitly; no push, release or tag.

### Acceptance and Validation ([90.2])

- [x] Blank lexical inbox requests return newest-first persisted pages with stable ID ties,
  account filtering and saved metadata. A desktop service regression needs no embedding provider.
- [x] Account semantic pages select exact eligible neighbors before pagination. Unfiltered
  vector ties, orphan vectors, absent embeddings, deep offsets and the vec0 k ceiling are covered.
- [x] Lexical ranking uses narrow rowids and covering-index account membership; hybrid keeps
  RRF/candidate coverage and hydrates final-page metadata once. IPC/schema/traits are unchanged.
- [x] Full offline workspace check/test/fmt/Clippy passed: 163 Rust tests, zero Clippy warnings.
  Frontend check had zero errors/warnings; all 56 tests and ordinary SPA build passed.
- [x] The final 100k release benchmark passed stronger count/reference assertions for all
  455 samples. Every case returns 200 rows. Original baseline samples remain unchanged.
- [x] Warm p50 lexical-common/filter changed from 56.303/58.740ms to 39.950/34.502ms. Blank
  inbox retrieval returns 200 rows at 0.363ms. The <15ms target remains unmet; corrected
  semantic-filtered returns 200 instead of 41 rows at 380.273ms. Record this cost explicitly.
- [x] Five policy scanner regressions passed with authorized subprocess access; tracked scan
  found zero issues. Runes/whitespace checks passed. k-015 preserves search query invariants;
  k-006 records expired sysroot recovery. No documented commands/configuration changed.
- [x] User authorized resuming after the sandbox subprocess circuit breaker. Native tests used
  loopback mocks; missing Linux prerequisites were restored through explicitly authorized
  build-package downloads into a disposable sysroot, without system installation.
- [x] Atomic implementation commit `c14f6fa` recorded; [90.3] promoted to READY FOR PLANNING.

### Remaining Limits ([90.2])

- Full hybrid/vector workloads still miss <15ms. Corrected account-filtered semantic/hybrid
  queries are slower than the previous incorrect candidate path; see before/after samples in
  `docs/performance/native-validation.md`. Account-partitioned indexing requires separate planning.
- Timing excludes embeddings, IPC and debounce; reopened samples retain the OS cache. Native
  rendering/production-memory, hosted CI, package installation, push/release/tag were not performed.
- The fixed-request tie-breakers do not guarantee stable hybrid pages across changing branch
  candidate-window sizes or concurrent writes; this existing ranking limitation is documented.

---

## Completed Performance Task Evidence

### 📌 Task [90.1]: Native Performance Validation

- **Description:** Measure native inbox scrolling, search latency, memory and binary footprint
  with synthetic local data; identify evidence-backed gaps against MVP performance targets.
- **Systems Involved:** Native desktop, storage/search and performance tooling.
- **Action Type:** Validation; implementation scope requires an approved plan.
- **Status:** DONE
- **Plan:** Approved by the user on 2026-10-31; validation complete.
- **Commit:** `98ed221` (`test(perf): measure native inbox and encrypted search at 100k rows`).
- **Next Task:** [90.2] Search Correctness and Performance (`READY FOR PLANNING`).

### Approved Plan ([90.1])
1. Inspect cached native build/runtime tooling and establish an isolated synthetic profile.
   Use no personal mailbox data, live providers, model downloads or external requests.
2. Add a reproducible release-mode storage benchmark with 100,000 synthetic messages and
   deterministic quantized vectors in SQLCipher. Record dataset/vector dimensions, query
   cases, result correctness, warm/cold methodology and latency distributions against <15ms;
   distinguish search execution from embedding generation and IPC overhead.
3. Measure the actual native WebKitGTK inbox with 100,000 synthetic summaries using isolated
   validation tooling. Record frame-time distributions, mounted row counts and paging behavior.
   Clearly separate UI fixture results from database-backed end-to-end behavior; report native
   display/tooling blockers without substituting browser results for native evidence.
4. Build the release desktop binary from cached dependencies; record binary size and process-tree
   idle/loaded memory with sampling method, host/build configuration and measurement limits.
   Report evidence-backed gaps; performance fixes require a separately approved scope.
5. Run the full repository Definition of Done and frontend build, respecting the two-failure
   circuit breaker. Load required domain skills if applicable; synchronize both READMEs if
   validation commands change. Preserve IPC, schema and core trait contracts.
6. Save reproducible results and applicable knowledge, create an atomic Conventional Commit
   and log its hash. Do not mark unmeasured targets verified or automatically start release
   work. No push, release or tag.


### Acceptance and Validation ([90.1])

- [x] Added opt-in release benchmarks for 100,000 SQLCipher messages and int8[384] vectors,
  with correctness assertions, warm/reopened samples and explicit result counts. Uses a
  runtime-generated disposable key and reserved-domain fixtures. No production query changes.
- [x] Native Tauri/WebKitGTK fixture completed at 1280x820, retaining at most 18 mounted rows
  across 100,000 preloaded summaries. Raw frame intervals, paging and process-tree RSS/PSS
  are committed with methodology. p95 preloaded intervals were 23ms; 60fps is not established.
- [x] Measured the ordinary release ELF at 35,631,608 bytes (33.98 MiB), unstripped. Native
  fixture median PSS was 374.21 MiB empty and 438.83 MiB loaded/scrolling. These exclude real
  accounts/vault/workers; neither a production idle baseline nor a memory ceiling is claimed.
- [x] Identified search gaps: hybrid/common lexical workloads exceed <15ms; account-filtered
  semantic search returns 41/200 rows despite 25,000 eligible vectors; empty lexical inbox
  queries return zero rows. Findings are recorded for a separately approved follow-up.
- [x] Workspace compilation, 159 Rust tests, formatting and all-target Clippy with -D warnings
  passed with offline cached dependencies and authorized loopback mocks. Feature-enabled
  native example also passed Clippy and release compilation.
- [x] Frontend check (zero errors/warnings), 56 tests, fixture check/build and restored ordinary
  SPA build passed. New JS/TS/JSON and Svelte fixture passed matching-style Prettier; shell
  and Node syntax checks, runes invariant and whitespace checks passed.
- [x] Authorized policy scanner/test rerun passed after the sandbox circuit breaker; all five
  regressions passed. Staged tracked-text scan covered 167 files with zero findings.
- [x] Both READMEs link the reproducible methodology using readme-standard. Native extracted
  helper paths/static route handling are preserved in k-014. Runtime IPC/schema/core traits
  remain unchanged. Temporary fixture sources/assets are absent from the ordinary SPA.
- [x] Atomic implementation commit `98ed221` recorded; [90.2] promoted to READY FOR PLANNING.
  Final Rust suite rerun passed all 159 tests; policy scan covered 167 text files with zero findings.

### Remaining Limits ([90.1])

- Frame callbacks do not prove GPU presentation or an all-hardware frame rate. WSL WebKit
  warned about unavailable DRI3 acceleration and GStreamer appsink. Paged fixture loaded only
  1,200 summaries; full database/IPC/embedding end-to-end latency remains unmeasured.
- Reopened storage queries retain the OS page cache. Search observations exclude vector
  generation, embedding inference, IPC and debounce; pseudorandom vectors do not test relevance.
- Fixture memory labels include post-load scrolling, as documented; no real-account baseline,
  native installer size, live-provider sync or release packaging was measured.
- No external endpoint, dependency registry, provider, AI endpoint or model download was used.
  No production performance fix, push, release or tag was authorized or performed.

---

## Completed CI Task Evidence

### 📌 Task [00.2]: CI Pipeline (GitHub Actions)

- **Description:** Add a GitHub Actions workflow running the repository Definition of Done,
  frontend build, runes invariant and secret/PII checks with pinned actions and native deps.
- **Systems Involved:** CI configuration and validation documentation.
- **Action Type:** Configuration and documentation changes.
- **Status:** DONE
- **Next Task:** [90.1] Native Performance Validation (`READY FOR PLANNING`).
- **Plan:** Approved by the user on 2026-10-31; implementation complete.

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


### Acceptance and Validation ([00.2])

- [x] Ubuntu 24.04 CI runs on pull requests, main pushes and manual dispatch. Both action
  dependencies use full commit hashes; contents permission is read-only, checkout does not
  persist credentials, jobs have timeouts and superseded runs are cancelled. No application
  secrets, publish steps or untrusted event expressions are passed to shell commands.
- [x] Rust 1.99.0 check/test/fmt/Clippy uses the locked dependency graph and runs offline
  after an explicit fetch. Native GTK/WebKitGTK, keyring and bundled SQLCipher build
  prerequisites are declared. Frontend uses Node 24.19.0, pnpm 11.25.0 and frozen installation.
- [x] Added a dependency-free tracked-text email/high-signal secret guard with redacted
  diagnostics and legacy Svelte syntax checks. Five synthetic regressions cover reserved
  domains, misleading suffixes, token patterns, runes, binary skipping, tracked-file selection
  and CLI failure status. Full scan covered 156 text files with zero findings.
- [x] `cargo check --workspace --locked`, `cargo test --workspace --locked` (159 passed),
  `cargo fmt --all -- --check` and Clippy for all targets with `-D warnings` passed using
  cached dependencies and the existing temporary native sysroot. Tests used authorized local
  loopback mocks. Frontend check (zero errors/warnings), tests (56 passed) and SPA build passed.
- [x] Workflow YAML parsed; trigger/job structure, SHA pins, read-only permission and checkout
  policy inspected; all run blocks passed Bash syntax validation. New JS/YAML passed Prettier.
  Svelte runes scan, staged/working-tree whitespace checks and manual changed-file review passed.
- [x] Both READMEs synchronized using `readme-standard`; native prerequisites, pinned CI
  versions, validation commands and scanner limits documented. Command parity, environment
  table coverage, heading/template checks and nonempty files verified. Replaced unavailable
  frontend Tauri CLI invocations with existing Vite/Cargo entrypoints; no manifest changes.
- [x] The initial scanner test runner failed twice and triggered the circuit breaker. After
  the user authorized diagnosis, direct execution exposed `spawnSync git EPERM`; approved
  local subprocess access passed all five tests. Recorded this sandbox gotcha in `k-013`.
- [x] Implementation commit `8649639` (`ci(workflow): enforce workspace validation and policy
  checks`) recorded; [90.1] promoted to READY FOR PLANNING. Runtime IPC/schema/core traits
  are unchanged; no active contract update was needed.

### Remaining Limits ([00.2])

- Hosted GitHub Actions execution, action retrieval and clean registry-based installs remain
  unverified locally. YAML/shell/static policy checks do not replace a hosted run. Existing
  installed frontend dependencies were used; the offline policy-metadata limitation in `k-008`
  remains applicable. No registry, model, provider or AI endpoint was contacted during this task.
- The scanner checks selected high-signal secret patterns and non-reserved mailbox addresses
  in tracked text, not every possible secret/PII representation or Git history. It complements
  manual review and does not constitute a public-release audit.
- No push, release, tag, native packaging or performance claim was made. CI setup will download
  declared build dependencies when the workflow is explicitly enabled by pushing the change.

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
`/home/yegear/github/ye-sandbox/agent-skills/skills/svelte-dev/SKILL.md`.

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
`/home/yegear/github/ye-sandbox/agent-skills/skills/svelte-dev/SKILL.md`.
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
| [90.1] | Native Performance Validation | `98ed221` | 2026-10-31 |
| [90.2] | Search Correctness and Performance | `c14f6fa` | 2026-10-31 |
| [90.3] | Account-Partitioned Search Optimization | `e40056a` | 2026-10-31 |
| [90.4] | Remaining Exact Search Latency | `8482a78` | 2026-10-31 |


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

### [00.2] CI Pipeline (GitHub Actions) — `DONE`
- **Scope:** Workflow running full DoD: cargo check/test/fmt/clippy (`-D warnings`), pnpm
  check/test/build, Svelte runes invariant, secret/PII scan, `git diff --check`.
- **Constraints:** No secrets in workflow; pinned action versions; GTK/WebKitGTK deps documented
  (see `k-006`); resolve offline frozen pnpm install question (`k-008`).

### [90.1] Native Performance Validation — `DONE`
- **Gap:** 60fps evidence is browser-fixture only; no native WebKitGTK measurement.
- **Scope:** Native WebKitGTK frame timing with 100k messages; hybrid search `<15ms` benchmark
  at 100k rows over SQLCipher; release binary size and idle RAM baseline.

### [90.2] Search Correctness and Performance — `DONE`
- **Evidence:** [90.1] measured hybrid p50 117–193ms, semantic account-filter underfill (41/200),
  and zero rows for blank lexical inbox queries. See the committed methodology/raw samples.
- **Scope:** Completed persisted inbox listing, exact account-aware neighbor selection,
  narrow lexical ranking and final-page metadata hydration; reran the 100k benchmark.
- **Outcome:** All cases return 200 rows; lexical-common/filter warm p50 is 39.950/34.502ms.
  The <15ms target remains unmet, and corrected account-vector ranking is slower. See [90.3].

### [90.3] Account-Partitioned Search Optimization — `DONE`
- **Evidence:** [90.2] semantic-filtered/hybrid-filtered warm p50 is 380.273/426.470ms with
  complete 200-row pages; common lexical remains above <15ms.
- **Scope:** Completed additive v5 account partition index, atomic maintenance and measured
  64MiB page-cache target; retained exact ties, fallback, DTOs and traits.
- **Outcome:** Warm filtered semantic p50/p95 is 12.041/12.928ms; hybrid-filtered p50 is 37.482ms.
  Other/reopened cases still miss <15ms; migration took 1296.317ms and added 44.11MiB in the fixture.

### [90.4] Remaining Exact Search Latency — `DONE`
- **Evidence:** [90.3] global semantic-common/hybrid-common warm p50 is 43.650/77.060ms;
  common lexical is 33.704ms. Filtered second semantic page is 16.099ms warm median.
- **Scope:** Profile global vector and FTS ranking costs, exact hybrid latency and remaining
  reopened overhead with independently checked neighbors and comparable 100k workloads.
- **Constraints:** Separate plan approval required. Preserve encryption, v5 partition consistency,
  exact ranking/candidate coverage and safety boundaries. Native rendering/real-account memory
  remain separate; no automatic push, release or tag.

---

## v0.1.0 Readiness Backlog (in execution order)

Registered 2026-10-31 from the v0.1.0 readiness review. Each task requires its own plan approval.
Active task: [02.5] (see above). Proposed defaults marked *(confirm at plan approval)*.

### [02.4] Lazy Body Fetch via BODYSTRUCTURE & Batched Initial Sync — `DONE`
- **Commit:** `5124327` (`feat(mail-engine): lazy body fetch via bodystructure and batched sync`).
- **Outcome:** RFC 3501 `BODYSTRUCTURE` parser extracts attachments lazily without payload downloads; two-phase fetch queries leaf text bodies only; initial sync batched to 300 messages with durable sequential commit per batch.

### [02.5] OAuth Token Refresh — `DONE`
- **Commit:** `d65416d` (`feat(mail-engine): handle oauth token refresh and clean orphaned staging`).
- **Outcome:** Pure-Rust `TokenRefresher` and `HttpTokenRefreshTransport` with strict P0 policy; SASL challenge abort in `protocol.rs`; transparent refresh & retry in `DefaultImapConnector`; `token_endpoint` persisted in vault; orphaned staging entries cleared at startup (`k-010`).

### [03.4] Tier 2 Honest Labeling (Experimental Semantic Search) — `DONE`
- **Commit:** `96c52a9` (`feat(triage-ai): label experimental semantic search and expose sync embedding telemetry`).
- **Outcome:** Marked Tier 2 experimental; hidden Semantic/Hybrid modes behind an explicit setting; honestly labeled feature-hash semantic projection in UI; exposed embedding failure telemetry in the sync worker (`SyncProgressEvent.embedding_errors_count`).

### [01.3] Pre-Release Schema Consolidation & Persisted Triage — `READY FOR PLANNING`
- **Gap:** ADR-006 duplicated the vector index solely to preserve an unreleased v4 schema.
  Triage results are recomputed per detail request; list badges transfer full bodies over IPC.
- **Scope:** Collapse migrations v1–v5 into a single clean v1 schema with one account-partitioned
  `vec_messages` (`account_id` PARTITION KEY); drop the derived table, reconciliation triggers and
  backfill. Persist triage category/badges at ingestion and expose them in `MessageRowDto`.
  Verify global (unfiltered) vec0 KNN over partitions at 100k against the exact reference oracle.
- **Governance:** ADR-007 supersedes ADR-006. Record policy: P1 schema immutability applies to
  released schemas; from tag `v0.1.0` migrations are append-only. Update NOTES.md contracts first.
- **Constraints:** P1 IPC change is additive. Dev databases must be recreated (re-sync from IMAP).

### [90.5] Parallel Read Pool & Latency Target ADR — `PENDING`
- **Gap:** Hybrid search runs lexical and semantic branches sequentially (~75ms warm p50).
  The [90.4] rationale (pooling unsafe for ACID) is incorrect: WAL read-only connections are safe.
  Global exact <15ms is physically infeasible (~42ms vector scan at 100k).
- **Scope:** Two read-only SQLCipher connections executing branches concurrently; measure memory
  cost of per-connection page cache. ADR redefining targets *(confirm at plan approval)*:
  p95 warm @100k — filtered semantic ≤15ms, lexical ≤40ms, global semantic ≤50ms, hybrid ≤60ms,
  UI first page incl. IPC ≤100ms. No approximate (ANN) search in v0.1; binary-quantized
  candidate + int8 rerank evaluated for v0.2.
- **Depends on:** [01.3] (final schema).

### [99.1] Public Transition Audit & Secret Scrubbing Verification — `PENDING`
- **Description:** Full public repository transition audit: zero PII/secret/token/non-reserved
  mailbox leaks across git history and tracked files; AGPLv3 notices; reproducible packaging.
- **Additional scope (C4, I6, I7):** "Status & Known Limitations" section in both READMEs
  (readme-standard) removing unmet claims (sub-15ms, 60fps native, ONNX, `async-imap`, TLS 1.3-only,
  INBOX-only sync). ADR documenting the in-house IMAP client choice. Measure real-account-free
  production idle memory baseline. Verify a hosted CI run (requires explicit push authorization).
- **Constraints:** RFC 2606 reserved domains only; no push/tag without explicit request.

---


## Release / Cycle Wrap-up (Not the next task)

Release/tag only with explicit human request. When triggered, the ID is `[99.1]`
(public-transition audit: git-history secret scan, AGPLv3 compliance, README sync, `v0.1.0` tag). Do not number feature, hygiene, or CI tasks as `99.x`. Do not calculate next task ID from this section.

---

## Future Backlog / Ideas (Unprioritized)

- Headless CLI / Docker daemon runner consuming `crates/*` without GUI
- Post-triage webhook & automation consumers implementing `TriageEventHook`
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
- Multi-folder sync (Archive/All Mail with Gmail dedup, Sent read-only); v0.1 syncs INBOX only (`v0.2`)
- Real Tier 2 ONNX inference via `fastembed`/`ort` if [03.4] keeps option B (`v0.2`)
- Fuzz/property tests for the in-house IMAP FETCH/literal parser (`v0.2`)
- Optional binary-quantized ANN candidate stage + exact int8 rerank (`v0.2`)
