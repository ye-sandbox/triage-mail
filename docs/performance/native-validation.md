# Native performance validation

These opt-in tools use synthetic local data only. They do not initialize real accounts,
read the user's vault, download models or contact providers. Run on an otherwise idle machine;
record hardware, OS, display/compositor, compiler versions and build profile with results.
Latency measurements are observations, not timing assertions in the ordinary test suite.

## Reproduction

Install the repository's documented native build prerequisites and frontend dependencies first.
With cached Cargo dependencies, use:

```bash
mkdir -p target
TMPDIR="$PWD/target" cargo run --offline --locked --release -p storage --example search_performance > search.csv
bash tools/performance/build-fixture.sh
cargo build --offline --locked --release -p triage-mail-desktop --features performance-validation --example native_performance
node tools/performance/sample-native.mjs target/release/examples/native_performance > native.json
pnpm --filter desktop build
cargo build --offline --locked --release -p triage-mail-desktop --bin triage-mail-desktop
stat -c '%s' target/release/triage-mail-desktop
```

The native fixture needs an accessible graphical session and GTK/WebKitGTK runtime libraries.
It opens a separate window and exits automatically. Use a disposable OS profile/session if
validating the production app; the example itself never boots application services or keyring.
Do not disable WebKit's sandbox to make a restricted execution environment pass.

`build-fixture.sh` temporarily creates an otherwise absent `__performance` route, typechecks
and builds it with the normal SvelteKit/Vite pipeline, copies the resulting assets to ignored
`target/performance-ui`, and removes its temporary sources even on failure. Rebuild the ordinary
SPA afterwards as shown above: the initial `apps/desktop/build` still contains fixture assets.
Do not package that intermediate build. The example embeds `target/performance-ui` with a
separate app identifier and only a local measurement-report command; it has no mail commands.
The example requires the explicit
`performance-validation` Cargo feature so ordinary checks do not need fixture assets.

## Storage method

The storage example creates a disposable file-backed SQLCipher database with an OS-random runtime
key, verifies its encrypted header, and deletes it on completion. It uses the production
migrations/default tokenizer and public repository search path, without changing query code.
The reproduction command places temporary data under the workspace `target` directory to
avoid a memory-backed `/tmp`. The run uses a disk-backed WSL ext4 virtual disk; OS-cold disk
latency is not measured. Fixture population uses prepared statements in one transaction; ingestion time is not measured.
There are 100,000 messages across four equally sized accounts, about 1 KiB of repeated synthetic
body text per message, and 100,000 deterministic pseudorandom int8 vectors of 384 dimensions.
One percent of subjects contain `invoice`; the others contain `newsletter`. All rare messages
belong to account 0; account 2 has only common messages. There are no attachments, tags or
folder overrides. These distributions are explicit, limited workloads, not real-mail realism.

Each mode (lexical, semantic, hybrid) runs rare/common, account-2-filtered, and second-page
(offset 200) queries with a 200-row page. Semantic queries ignore text. Every case records five
fresh-connection first queries, then five untimed warmups and 30 timed warm queries on one
connection. Reopened is **not OS-cold**: the kernel page cache is retained. Opening/migrations,
query vector generation, embedding inference, IPC serialization and UI debounce are outside the
timer. Sorting, metadata hydration and repository mutex acquisition are inside it.

Assertions check dataset/vector counts, uniqueness, bounds, account isolation, lexical content
and the exact unfiltered first-page semantic neighbor. Result counts remain in the CSV so
underfilled filtered KNN pages cannot be mistaken for complete results. Empty lexical input is
also observed as a separate inbox correctness case. No `<15ms` claim is inferred from the
existing five-message in-memory test. Do not interpret pseudorandom vectors as semantic quality.

## Native UI and memory method

The fixture renders production `InboxView`, `InboxController`, `MessageList` and badge-loading
components with injected in-memory transports and empty synthetic extraction metadata. It first
waits five seconds with no summaries, then allocates 100,000 summaries and waits five seconds.
It scrolls by 40,000 CSS pixels per animation frame for 180 frames through preloaded summaries.
It next resets through `connect()` and repeats for 180 frames using the actual 200-row paging
controller against the synthetic transport. It reports all requestAnimationFrame intervals,
maximum mounted row count, loaded summaries, page calls, viewport and user agent.

These intervals measure callback cadence, not GPU presentation or input-to-photon latency.
Crossing the loaded end in paged mode wraps to the current spacer size; that phase observes
prefetch/reconciliation rather than proving access to all 100,000 rows. Preloaded mode proves
only bounded rendering with that many resident summaries, not database-backed end-to-end sync.
The synthetic detail responses exercise visible badge requests, not real IPC latency.

The Node sampler runs the fixture child, samples its `/proc` descendant tree every 250ms,
and stores phase-tagged summed RSS and PSS in KiB. RSS double-counts shared mappings; PSS is
preferable when `pss_readable` equals `processes`. Processes may exit between reads. Measurements
include WebKit network/web processes, exclude the sampler/compositor, and have no fixed memory
target. The fixture's five-second idle baseline is a native shell without production workers
or vault; it is not a production-account idle-RAM claim. Compare loaded against idle with those
limits. Release size is the unstripped ELF file length, excluding shared libraries and installers;
no release packaging or all-hardware guarantee follows from it.

Without a duration argument the sampler requires a `complete` report and enforces a 90-second
limit. For a separately isolated ordinary process, `node tools/performance/sample-native.mjs
BINARY DURATION_MS` samples for an explicit 1,000–90,000ms duration and then sends SIGTERM.
That mode does not require fixture reports; no ordinary-production memory run is claimed here.

## Recorded run

Measured on 2026-10-31, Linux 6.18.33.2 WSL2 x86_64, AMD Ryzen 7 7700 host,
five exposed vCPUs and about 12 GiB guest RAM. Rust 1.99.0, Node 24.19.0, pnpm 11.25.0;
default Cargo release profile (no custom LTO, stripping or native-CPU flags). Cached Ubuntu
GTK 3.24.52 and WebKitGTK 2.52.6 ran against the existing WSL graphical session. Native output
reported unavailable DRI3 acceleration and a missing GStreamer appsink; no multimedia was used.
No GPU/driver/compositor presentation claim is made. This is a shared machine, not CPU-pinned
laboratory hardware. Search was rerun on the workspace filesystem after builds and native measurement finished;
earlier compiler-contended and memory-backed `/tmp` diagnostic passes are excluded.

SQLCipher 4.5.7 community, SQLite 3.45.3, sqlite-vec 0.1.9. The encrypted synthetic database was
216,387,584 bytes (206.36 MiB), excluding transient WAL/SHM. The release production ELF was
35,631,608 bytes (33.98 MiB), unstripped and dynamically linked. Packaging was not attempted.

### Search

Raw samples: [search-100k.csv](search-100k.csv). p50 uses the median; p95 uses nearest rank
`ceil(0.95 * n)`. Warm distributions have 30 samples; reopened has five. Query limit is 200.
Reopened connection latency excludes opening/migrations and retains the OS cache.

| Case | Warm p50 ms | Warm p95 ms | Reopened p50 ms | Rows returned |
|---|---:|---:|---:|---:|
| Lexical-rare | 5.469 | 5.823 | 6.000 | 200 |
| Lexical-common | 56.303 | 65.153 | 59.623 | 200 |
| Lexical-filtered | 58.740 | 67.328 | 59.265 | 200 |
| Lexical-page2 | 53.376 | 61.312 | 58.122 | 200 |
| Semantic-rare | 112.583 | 122.837 | 117.506 | 200 |
| Semantic-common | 118.393 | 132.209 | 112.704 | 200 |
| Semantic-filtered | 127.864 | 144.856 | 128.311 | 41 |
| Semantic-page2 | 133.998 | 147.631 | 146.154 | 200 |
| Hybrid-rare | 117.330 | 128.580 | 118.804 | 200 |
| Hybrid-common | 171.650 | 182.257 | 168.370 | 200 |
| Hybrid-filtered | 168.010 | 183.023 | 169.261 | 200 |
| Hybrid-page2 | 192.626 | 207.906 | 190.190 | 200 |
| Lexical-empty-inbox | 0.000 | 0.000 | 0.000 | 0 |

The full hybrid workload misses the `<15ms` target. Common lexical matches also exceed it.
The benchmark asserts correctness independently of timing; a fast empty result is not success.
Filtered semantic search returns only **41 of 200 requested rows**, although the filtered
account contains 25,000 embedded messages. Production SQL asks vec0 for the global top-k before
filtering the joined messages by account; this is post-filtering, not the intended KNN
pre-filter. Hybrid output fills from lexical candidates but can still miss relevant account
neighbors. Blank lexical input returns zero messages, so the native shell's ordinary empty
inbox request does not expose persisted mail through this search implementation. The injected
UI transport is separate from this database-backed path.

### Native frame timing and memory

Raw frame intervals and process samples: [native-100k.json](native-100k.json). The Tauri example
exited successfully after all phases. Each scroll phase contains 180 intervals at 1280×820 CSS
pixels. It used the production inbox components with motion enabled and synthetic transports.

| Phase | Resident rows at completion | p50 ms | p95 ms | Max ms | Intervals >20ms | Max mounted rows |
|---|---:|---:|---:|---:|---:|---:|
| preloaded-scroll | 100,000 | 17.0 | 23.0 | 41.0 | 20 | 18 |
| paged-scroll | 1,200 | 16.0 | 24.0 | 32.0 | 17 | 18 |

Preloaded scroll height was 8,800,000px. Paged mode made six synthetic 200-row requests and
finished with 1,200 loaded rows and a 105,600px spacer; it did not traverse all 100,000 records.
Bounded mounting is demonstrated. Consistent 60fps is not established: the preloaded p95 frame
interval exceeded the approximately 16.7ms budget. WebKit here reports frame timestamps in
whole milliseconds, so a 17ms interval alone should not be treated as a precise dropped frame.

| Sample label | Samples | Median summed RSS MiB | Median summed PSS MiB |
|---|---:|---:|---:|
| idle | 21 | 534.81 | 374.21 |
| loaded | 32 | 599.57 | 438.83 |

All samples in these two labels covered three processes with readable PSS. Labels update when
a report is emitted: `idle` covers the five-second empty-shell wait until summaries are
allocated; `loaded` covers the five-second resident wait **and preloaded scrolling**, until its
completion report. These are phase-label medians, not an idle-only loaded baseline. PSS rises
about 64.62 MiB between the two medians. Neither baseline includes real accounts, vault or mail
workers. No memory ceiling was specified, and no memory target is declared passed or failed.

The extracted WebKit runtime initially failed to find helpers at its compiled `/usr/lib` path.
An isolated read-only mount namespace exposed cached helpers there without changing host
packages or disabling WebKit sandboxing. The first asset-load attempt timed out because the
fixture URL ended in `/`; removing it made SvelteKit's relative assets resolve correctly.
See [k-014](../../.agent/knowledge/k-014-native-webkit-performance-fixtures.md).

### Follow-up scope

Prioritize ordinary empty-inbox retrieval and account-aware KNN candidate selection before
optimizing latency. Profile common FTS matches, vector scans, account filtering, candidate
hydration and pagination at this scale. Repeat native measurement on a supported accelerated
desktop before drawing compositor or hardware-wide conclusions. These are proposed follow-ups;
no production search/UI optimization, schema/IPC change, push, release or tag is part of [90.1].


## Search correction follow-up ([90.2])

Recorded on 2026-10-31 after the separately approved implementation plan. The original
[90.1] samples above remain the historical baseline. New raw samples are in
[search-100k-corrected.csv](search-100k-corrected.csv); the existing reproduction command and
fixture distribution are unchanged. Same host/compiler, SQLCipher/SQLite/sqlite-vec versions,
release profile, disk-backed workspace and 216,387,584-byte database. This final run started
after compilation/tests finished. Earlier compiler-contended diagnostic runs are excluded.
These are sequential observations on a shared host, not a randomized controlled comparison.

Blank lexical requests now list persisted mail newest first with message-ID ties and normal
pagination. Account-filtered semantic search enumerates eligible message IDs before exact
cosine ranking, so other accounts cannot consume its top-k slots. Unfiltered search keeps
vec0 top-k with one extra neighbor, expands across boundary ties and falls back to exact
ranking at its 4096-candidate ceiling or for deeper pages. Missing embeddings and orphan
vectors cannot consume returned page slots. Schema versions, IPC DTOs and trait signatures
are unchanged; no migration or new dependency was introduced.

Lexical search ranks rowids before reading summary payload pages. For account filters,
`+rowid IN (...)` prevents repeated FTS MATCH point scans and uses the existing covering
account index for membership. An isolated query-plan diagnostic found an explicit
rank/rowid tie-breaker plus wide joins took roughly 250ms, whereas membership filtering
before summary hydration took roughly 30ms. Hybrid retains its existing RRF formula and
candidate-window sizes, adds a message-ID tie-breaker and hydrates metadata only after
fusion. Stable ranking applies to a fixed request and dataset; changing the hybrid candidate
window or ingesting messages can still change page membership.

Correctness assertions now require **200 rows in every benchmark case**. Every semantic
page is compared, outside the timer, with an independent reference built from integer dot
products/norms over the synthetic vectors. The empty lexical page is checked against exact
newest-first fixture IDs. Existing uniqueness, account-isolation and lexical-content checks
remain. Unit/service regressions additionally cover reopening encrypted storage, whitespace,
equal timestamps, tags/folder overrides, missing embeddings, orphan vectors, tied neighbors,
an oversized 4100-vector tie and offsets beyond vec0's k ceiling. Ordinary timing assertions
were not weakened. The independent reference tests ordering, not semantic relevance.

| Case | Warm p50 ms | Warm p95 ms | Reopened p50 ms | Rows returned |
|---|---:|---:|---:|---:|
| Lexical-rare | 6.325 | 7.244 | 6.026 | 200 |
| Lexical-common | 39.950 | 44.692 | 44.687 | 200 |
| Lexical-filtered | 34.502 | 35.952 | 35.588 | 200 |
| Lexical-page2 | 38.563 | 41.435 | 39.038 | 200 |
| Semantic-rare | 110.343 | 113.919 | 112.841 | 200 |
| Semantic-common | 109.576 | 112.448 | 110.218 | 200 |
| Semantic-filtered | 380.273 | 391.899 | 382.545 | 200 |
| Semantic-page2 | 126.846 | 131.932 | 127.005 | 200 |
| Hybrid-rare | 114.302 | 118.479 | 114.292 | 200 |
| Hybrid-common | 153.382 | 169.400 | 147.850 | 200 |
| Hybrid-filtered | 426.470 | 483.285 | 445.044 | 200 |
| Hybrid-page2 | 172.060 | 182.446 | 166.623 | 200 |
| Lexical-empty-inbox | 0.363 | 0.430 | 1.054 | 200 |

Common lexical warm p50 improved from 56.303ms to 39.950ms (29% lower); filtered lexical
improved from 58.740ms to 34.502ms (41% lower). Hybrid-common changed from 171.650ms to
153.382ms and hybrid-page2 from 192.626ms to 172.060ms. Rare lexical changed from 5.469ms
to 6.325ms; this is not a claim that every workload improved. Blank inbox retrieval returns
200 rows at 0.363ms warm p50 instead of returning no persisted mail.

**The <15ms search target remains unmet.** In particular, the schema-preserving account
semantic path now returns the correct 200 neighbors at 380.273ms warm p50. The original
127.864ms returned only 41 rows and is not an equivalent complete-result workload.
Hybrid-filtered now uses those correct candidates and measures 426.470ms versus 168.010ms
with the old incomplete semantic branch. Exact per-message vector lookups are expensive;
this correctness cost remains a performance regression requiring further work.

[90.3] should evaluate account-partitioned vector storage/indexing and the remaining lexical
ranking cost under a separately approved plan, including any additive schema migration.
Do not silently cap candidates, accept underfilled pages or substitute approximate neighbors
to pass the target. Final measurements still exclude embeddings, IPC and UI debounce;
reopened connections retain the OS cache. Native frame timing and production memory were
not remeasured in [90.2]. No real accounts, providers, model downloads or AI calls were used.
The user explicitly permitted build-package downloads to restore a missing disposable Linux
sysroot; these were extracted locally without installing system packages.


## Account-partitioned index follow-up ([90.3])

Recorded on 2026-10-31 following the separately approved v5 migration plan. Raw samples:
[search-100k-partitioned.csv](search-100k-partitioned.csv). The existing reproduction command
runs the expanded benchmark; no new flags, environment variables or dependencies are needed.
Compiler/host, release profile, vector generator, SQLCipher/SQLite/sqlite-vec versions and the
original 100k equal-account fixture remain the same. Final samples were collected after Rust
and frontend compilation finished. The timing changes are sequential shared-host observations,
not a randomized controlled experiment.

Migration v5 retains authoritative `vec_messages` and adds derived `vec_messages_by_account`
with an opaque text account partition key. Backfill only live messages; maintain replacements
and membership changes transactionally. Filtered vec0 KNN prunes partitions before top-k and
retains the existing exact ID tie/deep-page fallback. Global search still uses the original
table. RRF formula/window sizes, tokenizer semantics, IPC and core traits are unchanged.
No approximate neighbors, result cache or reduced candidate coverage was introduced. Older
writers must not write a v5 database because they do not maintain its derived index.

The fixture constructs a synthetic v4 database by removing v5 additions from an otherwise
empty new database, then populating the original messages/vectors. It never downgrades a user
database. The timed migration transaction (including commit, excluding the subsequent WAL
checkpoint) indexed 100,000 vectors in **1296.317ms** in one observed run. After checkpoints,
the main file grew from 216,391,680 to 262,643,712 bytes: **44.11MiB additional space** (about
21.4%). The v4 fixture is one 4096-byte page larger than the [90.2] fixture due to construction
metadata. Peak migration WAL/disk usage, real-mail ingestion overhead and startup distributions
were not measured. Regression tests force a failure after backfill to verify schema/data rollback.

### Cache and query-component profile

`PRAGMA cache_size=-65536` now sets a **64MiB soft target per connection**, replacing SQLite's
usual 2MiB target. This does not cap process RSS. Encrypted WAL, synchronous NORMAL, foreign
keys, busy timeout and in-memory temporary storage retain their prior settings. There is no
plaintext disk cache, extra unsafe code, new FFI or change to the SQLCipher key boundary.

The benchmark records eight component queries per size (first query, two warmups, five samples)
on a fresh connection; the table below uses the last five medians. Global vector SQL requests
201 candidates directly from vec0; FTS ranks 200 rowids with deterministic ties; hydration reads
200 summaries without full metadata fusion. Components are isolated lower-level work, not
an additive decomposition of the complete repository call. Raw profile rows are CSV comments.

| Cache target | Global vector top-k median ms | FTS ranking median ms | Summary read median ms |
|---|---:|---:|---:|
| 2MiB | 105.391 | 30.491 | 0.015 |
| 16MiB | 107.362 | 33.526 | 0.014 |
| 64MiB | 42.751 | 30.280 | 0.014 |

This controlled cache-size comparison is consistent with avoiding repeated encrypted vector
page reads; page-cache miss counters and hardware/crypto counters were not measured. FTS
ranking remains substantial CPU/query work; cached summary reads alone do not explain it.
The 64MiB target increases the available page-cache budget by 62MiB per connection. RSS was
about 79.5–79.6MiB across profile phases and 79.62MiB at benchmark completion. These process
observations include fixture seeding, migration, allocator retention and oracle construction;
sequential RSS staying flat **does not establish zero incremental cache memory cost** for a
fresh application. No isolated per-connection resident-cache or production memory claim is made.

### Complete search calls

Original cases retain five reopened queries, five warmups and 30 timed warm queries with
200-row pages. The independent exact-neighbor oracle, lexical content checks, account
isolation, uniqueness and complete-result assertions remain outside the timer. The original
13 cases are comparable with [90.2]. Four additional cases follow them: a second account page,
then a separate sparse phase adding 31 vectors/messages and an empty account. That phase has
100,031 messages, with 31 versus 25,000 vectors per sparse/large account (about 806:1).
The sparse first page returns 20, the second returns all 11 remaining rows and the empty
account returns zero; those smaller complete results are **not** 200-row latency improvements.
This tests account-size skew, not a realistic dominant-mailbox distribution.

| Case | Warm p50 ms | Warm p95 ms | Reopened p50 ms | Rows returned |
|---|---:|---:|---:|---:|
| Lexical-rare | 0.902 | 1.016 | 5.736 | 200 |
| Lexical-common | 33.704 | 36.114 | 38.276 | 200 |
| Lexical-filtered | 25.718 | 26.569 | 34.917 | 200 |
| Lexical-page2 | 33.571 | 35.004 | 39.064 | 200 |
| Semantic-rare | 45.435 | 50.075 | 127.099 | 200 |
| Semantic-common | 43.650 | 46.313 | 113.012 | 200 |
| Semantic-filtered | 12.041 | 12.928 | 34.848 | 200 |
| Semantic-page2 | 60.230 | 64.414 | 155.202 | 200 |
| Hybrid-rare | 44.129 | 47.184 | 119.919 | 200 |
| Hybrid-common | 77.060 | 83.450 | 150.338 | 200 |
| Hybrid-filtered | 37.482 | 38.495 | 65.696 | 200 |
| Hybrid-page2 | 92.601 | 97.397 | 167.118 | 200 |
| Lexical-empty-inbox | 0.354 | 0.408 | 0.981 | 200 |
| Semantic-filtered-page2 | 16.099 | 16.983 | 39.280 | 200 |
| Semantic-sparse | 0.145 | 0.335 | 1.048 | 20 |
| Semantic-sparse-page2 | 0.152 | 0.190 | 1.105 | 11 |
| Semantic-empty-account | 0.012 | 0.017 | 0.152 | 0 |

Warm filtered semantic p50/p95 improved from 380.273/391.899ms to **12.041/12.928ms** with
the same exact 200 neighbors. Filtered hybrid p50 improved from 426.470ms to **37.482ms**;
global semantic-common from 109.576ms to 43.650ms; hybrid-common from 153.382ms to 77.060ms.
Common/filter lexical p50 changed from 39.950/34.502ms to 33.704/25.718ms.

**The full <15ms target is still unmet.** Only the observed warm first filtered semantic
page meets it among the substantive vector workloads. Its reopened median is 34.848ms and
the second filtered page warm median is 16.099ms. Global scans, FTS ranking and complete hybrid
queries need further work. Oversized ties and deep pages intentionally use the slower exact
fallback; their correctness is covered separately, without claiming these distributions apply.
These timings still exclude embeddings, IPC and debounce; reopened is not OS-cold. Native
frame timing and production memory were not remeasured. Further latency work requires a
separately approved [90.4] plan rather than claiming the release performance target complete.
