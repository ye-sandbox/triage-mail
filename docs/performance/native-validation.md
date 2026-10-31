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
