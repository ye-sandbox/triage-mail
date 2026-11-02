# Mailbox validation — 2026-11-02

**Feature closure is blocked.** The production blank account Inbox search did not return
its first page within a 180-second process budget on the migrated synthetic database.
The native fixture reached ready state but its first scoped batch did not finish within
its 90-second budget. No native first-page, frame-cadence or 60fps pass is claimed.
Production code, schema, IPC contracts, dependencies and performance targets are unchanged.

## Reproduction and dataset

Use the existing Rust/native prerequisites, installed frontend dependencies and an offline
Cargo cache. No provider, production service, user vault or model is accessed.

```bash
cargo run --offline --release -p storage --example mailbox_performance
# Optional exact case selection; each still seeds a fresh disposable database:
cargo run --offline --release -p storage --example mailbox_performance -- Inbox
cargo run --offline --release -p storage --example mailbox_performance -- Plan
bash tools/performance/build-fixture.sh
cargo build --offline --release -p triage-mail-desktop --example native_performance --features performance-validation
node tools/performance/sample-native.mjs target/release/examples/native_performance
# Restore ordinary frontend assets before building an application:
pnpm --filter desktop build
```

The full storage command intentionally retains the failing Inbox workload. Its optional
case names are Global, Inbox, Starred, Snoozed, Archive, All-label and Empty; Plan prints
EXPLAIN output for the production blank Inbox SQL. An external `timeout 180` bounded the
recorded Inbox probe. Timed-out work is not assigned a fabricated latency percentile.
The native example requires a functioning graphical WebKitGTK runtime; the extracted helper
namespace method in [k-014](../../.agent/knowledge/k-014-native-webkit-performance-fixtures.md)
was used without installing system packages or disabling WebKit sandboxing.

Shared fixture: `tools/performance/mailbox_data.rs`. A fresh file-backed SQLCipher v2 store
contains 100,000 canonical messages, 100,000 int8[384] vectors, four accounts with 25,000
messages each, and 200,000 memberships. Every message has its real Inbox/Archive membership
plus an All Mail label with a separate UID. This measures already deduplicated storage;
scripted ingestion/deduplication regressions remain separate evidence.

For each account: 20,000 Inbox members, 5,000 Archive members, 2,500 active snoozes in Inbox,
2,500 expired snoozes in Inbox, 17,500 eligible Inbox messages, and 31 starred messages.
An advertised Empty folder has no members. Starred spans both real folders and snooze states.
Canonical identity is message-NNNNNN, account-(N%4), UID N+1. All content is synthetic with
reserved domains. Every message has identical indexed content and length, making lexical
BM25 ties independently predictable by insertion rowid. Vectors are deterministic
pseudorandom data and say nothing about semantic quality. Runtime database keys come from
OS randomness, are never logged or persisted, and temporary databases are removed on exit.
The database is 120,233,984 bytes excluding transient WAL/SHM.

Machine: Linux 6.18.33.2 WSL2 x86_64, AMD Ryzen 7 7700 host, five exposed vCPUs,
12,249,460 KiB guest RAM; Rust 1.99.0, Node 24.19.0, pnpm 11.25.0, default optimized
Cargo release profile. SQLCipher 4.5.7 community, SQLite 3.45.3, sqlite-vec 0.1.9.
This shared machine is not CPU pinned. Completed storage distributions were collected
without concurrent compilation/native workloads. An earlier compiler-contended partial
run was discarded. Native execution overlapped a short benchmark rebuild and yielded no
accepted timing distributions.

## Correctness and timing method

Every accepted search calls production `SqliteMailStore::search_scoped`. Independently
computed eligibility precedes integer dot-product/norm cosine ranking. Blank order uses
received timestamp descending; lexical ties use insertion order; hybrid oracles implement
the contracted 60-offset RRF candidate window and deterministic ID ties. Every returned
page must match its entire oracle order, count, uniqueness, account/view eligibility and
canonical folder/UID. Validation runs outside the measured query interval, on every sample.
This is not a second call to production search used as its own oracle.

Cases request 200 summaries at offsets 0 and 4,000 in blank, lexical, semantic and hybrid
modes. Each completed case records three reopened-connection samples, five untimed warmups
and 30 warm samples. Reopened excludes connection/migration time and is not OS-cold.
p50 is the median; p95 is nearest rank ceil(0.95*n). Mutex acquisition, ranking, fusion
and hydration are included. Seeding, query-vector generation, inference, IPC and rendering
are excluded from storage timing.

## Blocking query plan

[Raw plan](mailbox-query-plan.txt), [bounded probe](mailbox-inbox-timeout.txt).
The 180-second process budget expired with exit 124 before even the first reopened Inbox
sample returned. That budget includes seeding/oracle construction, so it is not an exact
180-second per-query measurement. It establishes failed bounded validation, not a result
ordering error. The native first batch likewise contains no accepted samples.

```text
SEARCH m USING INDEX idx_messages_account_provider_msgid (account_id=?)
CORRELATED SCALAR SUBQUERY 1
SEARCH membership USING INDEX idx_memberships_account_folder (account_id=?)
SEARCH o USING INDEX sqlite_autoindex_message_folder_overrides_1 (message_id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

The inner plan constrains account only despite the message-ID correlation. With 25,000
outer account rows and 50,000 memberships, it permits about 1.25 billion membership visits.
This is the leading explanation of the stall; an instrumented opcode/visit count was not
collected. The existing message-ID index is present. No fixture-only ANALYZE, optimizer hint,
index or statistics change was applied. Other folder-dependent cases are deferred behind
this blocker rather than repeatedly running the same pathological plan.

## Completed storage measurements

All completed samples passed their independent correctness assertions. Raw CSVs: [Global](mailbox-global.csv), [Starred](mailbox-starred.csv), [Snoozed](mailbox-snoozed.csv).

| View | Mode | Offset | Eligible | Results | Warm p50 ms | Warm p95 ms |
|---|---|---:|---:|---:|---:|---:|
| Global | Blank | 0 | 100000 | 200 | 0.341 | 0.379 |
| Global | Blank | 4000 | 100000 | 200 | 1.334 | 1.799 |
| Global | Lexical | 0 | 100000 | 200 | 33.779 | 39.391 |
| Global | Lexical | 4000 | 100000 | 200 | 33.070 | 36.771 |
| Global | Semantic | 0 | 100000 | 200 | 45.895 | 48.351 |
| Global | Semantic | 4000 | 100000 | 200 | 336.952 | 342.335 |
| Global | Hybrid | 0 | 100000 | 200 | 49.966 | 53.504 |
| Global | Hybrid | 4000 | 100000 | 200 | 427.396 | 502.140 |
| Starred | Blank | 0 | 31 | 31 | 0.068 | 0.095 |
| Starred | Blank | 4000 | 31 | 0 | 0.018 | 0.032 |
| Starred | Lexical | 0 | 31 | 31 | 6.067 | 6.626 |
| Starred | Lexical | 4000 | 31 | 0 | 5.771 | 6.438 |
| Starred | Semantic | 0 | 31 | 31 | 0.153 | 0.209 |
| Starred | Semantic | 4000 | 31 | 0 | 0.098 | 0.123 |
| Starred | Hybrid | 0 | 31 | 31 | 6.478 | 8.618 |
| Starred | Hybrid | 4000 | 31 | 0 | 6.383 | 9.001 |
| Snoozed | Blank | 0 | 2500 | 200 | 6.042 | 7.557 |
| Snoozed | Blank | 4000 | 2500 | 0 | 7.384 | 8.109 |
| Snoozed | Lexical | 0 | 2500 | 200 | 18.885 | 20.575 |
| Snoozed | Lexical | 4000 | 2500 | 0 | 19.148 | 21.944 |
| Snoozed | Semantic | 0 | 2500 | 200 | 21.551 | 23.540 |
| Snoozed | Semantic | 4000 | 2500 | 0 | 21.189 | 23.324 |
| Snoozed | Hybrid | 0 | 2500 | 200 | 39.658 | 43.309 |
| Snoozed | Hybrid | 4000 | 2500 | 0 | 45.420 | 70.344 |

Against ADR-008, the comparable global first-page lexical, semantic and parallel hybrid
p95 observations meet 40/50/60ms respectively. The global offset-4,000 semantic/hybrid
paths exceed those first-page bounds and use the existing exact fallback; this is a
workload difference, not proof that every page meets the target. Sparse Starred (31 rows)
and Snoozed (2,500 rows) are not comparable to the 25k filtered-semantic reference.
Snoozed semantic first-page p95 is 23.540ms, exceeding the 15ms filtered-semantic
budget even with fewer eligible vectors; it uses the scoped exact-lookup path rather
than the reference partition top-k path. This remains a measured miss requiring follow-up.
Their offset-4,000 pages correctly contain zero results. These fast/empty results cannot
substitute for dense Inbox or explicit-folder acceptance.

## Native and browser evidence

The isolated example uses the production search DTO, Tauri serialization, storage entry
point, InboxController and InboxView with four synthetic accounts. Catalog reads also use
fixture-only IPC. No production service, vault, provider or embedding runtime is started.
Its predefined query vector excludes inference by design; production consent gates are not
being tested by this fixture. Read-only triage and attachment transports refuse writes.

The intended first-page interval begins at controller submission, excluding the toolbar's
200ms debounce, waits for response hydration and two animation-frame callbacks, and reports
IPC-response time separately. Two callbacks are a conservative opportunity to paint, not
proof of GPU presentation. Five warmups and 30 samples are required per distribution.
The later phases preload 100k summaries for bounded-rendering measurement and then explicitly
reload the first database page for paged scrolling. These phases did **not** execute in this run.

[Incomplete native report](mailbox-native-incomplete.json) contains only the ready/idle phase
with 200 global rows, process samples, and a SIGTERM result. It is not a completed native
measurement. The extracted WebKit runtime warned about DRI3 and missing GStreamer appsink;
no multimedia or GPU-presentation result is claimed. The initial sampler timeout terminated
only its wrapper and left native descendants holding stdout. Those specific fixture processes
were terminated. The sampler now uses an isolated Unix process group; a disposable wrapper
with a sleeping child verified timeout completion with exit 0 in timed-stop mode. The native
slow query was not rerun merely to reproduce the same blocker.

Browser observations use the fixture's **in-memory** fallback, not SQLCipher timing evidence.
Screenshots are synthetic browser captures at desktop 1280x820 and narrow 390x844 viewports:

- [Desktop empty folder, dark EN](mailbox-screenshots/desktop-empty-dark-en.png).
- [Keyboard-selected Starred, dark PT-BR](mailbox-screenshots/desktop-starred-dark-pt.png).
- [Narrow Starred with sidebar dismissed, dark PT-BR](mailbox-screenshots/narrow-starred-dark-pt.png).

| Acceptance area | Evidence/status |
|---|---|
| Multiple independent disclosures, real/Other folders, long labels, initials fallback | Browser observed |
| Scoped Starred (31 rows), empty folder, account/view context | Browser observed; production storage ordering independently checked where completed |
| Keyboard disclosure and mailbox activation | Enter/Tab/Enter observed with visible focus |
| Narrow sidebar dismissal and selected context | Browser observed |
| EN/PT-BR mailbox labels and disclosure persistence after reload | Browser observed; current scope resets to All Mail as specified |
| Browser console errors | None observed |
| Late scope/page/refresh results, canonical reader identity, rejected mutations, expiry | Existing controller/component regressions pass |
| Offline cached catalog, account additions/removal, avatar remount/failure | Existing component regressions pass; no new native restart claim |
| Empty/one-account visual states, light theme, reduced-motion visual state, actual photo visual state | Still require browser/native visual verification |
| Dense Inbox/explicit-folder timing, complete native first-page and 100k scrolling/frame report | Blocked/uncompleted; do not reuse historical frame reports |

## Repository checks and follow-up

Offline workspace check, all 209 Rust tests, fmt, workspace all-target Clippy with warnings
denied, and an additional all-features Clippy check including the native example pass.
Desktop check has zero errors/warnings, all 126 frontend tests pass, fixture and restored
ordinary SPA builds pass. Existing loopback tests and policy scanner needed local sandbox
exceptions. Policy scanner regressions pass; tracked/new text policy and diff review are
recorded with the validation commit. No network downloads or system installation occurred.

[04.1.7] and the parent feature remain open; the next milestone is not promoted. The required
follow-up is approval to fix the production scoped-membership query plan within
`crates/storage/src/scoped_search.rs` and its regression tests, preserving the schema and
IPC contracts. Then repeat dense/sparse/empty/deep-page correctness and timing, finish the
native report and remaining visual matrix, and close only against actual passing evidence.
No timing-target relaxation, approximate search, fixture-only tuning or release is proposed.
