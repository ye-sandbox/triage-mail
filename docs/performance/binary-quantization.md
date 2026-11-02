# Binary candidate evaluation — 2026-11-02

Recommendation: retain exact int8 cosine search. Do not ship fixed-budget binary
candidates as an oracle-equivalent replacement. Exact reranking corrects only the
order of retained candidates; it cannot recover omitted neighbors. ADR-008's proposed
200-candidate stage is a hypothesis, not an exactness guarantee.

## Reproduction

Use the existing offline Cargo cache and native prerequisites from the
[mailbox validation](mailbox-validation.md). No network, provider, vault, model download
or production database is used. Build and finish other validation before timing:

```bash
cargo test --offline -p storage --example binary_quantization
cargo build --offline --release -p storage --example binary_quantization
target/release/examples/binary_quantization > docs/performance/binary-quantization.csv
node tools/performance/summarize-binary.mjs docs/performance/binary-quantization.csv
```

The example takes no arguments. Its temporary directory and OS-random key exist only
for the run; it prints no keys. The binary index is inside the same SQLCipher database,
not an unencrypted sidecar or a production migration. No new dependencies or unsafe
code are required. The static fixture contains no concurrent mutations; this does not
validate crash recovery, transactional index maintenance or a production migration.

## Workload and method

The example reuses `tools/performance/mailbox_data.rs` unchanged: 100,000 messages,
100,000 int8[384] vectors, four accounts and 200,000 memberships. It runs Global,
Inbox, Starred, Snoozed, Archive, All-label and Empty, requesting 200 summaries at
offsets 0 and 4,000 with query seed 4242. Global first pages additionally use seeds 17
and 90001. These are deterministic pseudorandom vectors, not learned embeddings;
recall here is not a claim about semantic quality or a real-world recall distribution.

The experimental vec0 table stores 384-bit (48-byte) sign codes, using
`vec_quantize_binary(vec_int8(...))`: strictly positive components become 1; zero and
negative become 0. Candidate budgets are 200, 800 and 4,096. Account partitioning and
an eligible-message rowid IN subquery apply mailbox eligibility **before** top-k,
including snoozes, label membership and pending folder overrides. Global skips the
eligibility subquery because the index was built from live messages and the fixture
is immutable. A production implementation would need transactional lifecycle rules.
Integer keys are public message rowids; no vec0 shadow tables are read or written.

A MATERIALIZED CTE collects Hamming candidates, followed by scalar int8 cosine
reranking with canonical message-ID tie order and production summary hydration (its private source is included in the example rather
than widening the library API).
Raw dot product is not substituted for cosine: unnormalized int8 vectors have different
norms. Candidate generation, query quantization, eligibility SQL, reranking, transaction
and hydration are timed together. The candidate path uses one read snapshot. Production
controls call `SqliteMailStore::search_scoped`; no production ranking code changed.
Seeding, index construction, oracle calculation, opening connections and inference
are excluded from query timings. Index construction and encrypted file growth are
reported separately in the CSV comments.

Each case/algorithm has one reopened-connection sample, five untimed warmups and 30
warm samples. Reopened is not OS-cold. Each page is checked after timing against an
independent Rust i64 dot/norm cosine oracle with deterministic ID ordering, uniqueness
and independent fixture eligibility. `oracle_hits/oracle_count` is page-set recall;
`equivalent` compares the **entire ordered page and length**. Empty pages have no recall
denominator; passing them does not demonstrate successful dense search. Production
and fallback mismatches abort; BQ mismatches are recorded as results, not hidden.

sqlite-vec 0.1.9 caps KNN k at 4,096. An offset-4,000/200-row page requires at least
4,200 candidates, so all BQ variants explicitly dispatch to production exact search.
Those rows are labeled `bq-fallback`, not attributed to binary acceleration. Large
Hamming ties can also exceed the cap; no Hamming frontier provides a bound on omitted
int8 cosine distances. A finite cosine gap after reranking is not a coverage proof.

## Measurements

The run exited 0: **64 case/algorithm groups, 64 reopened and 1,920 warm samples**.
All 480 warm production controls and 630 warm deep-fallback samples matched the
independent oracle. Of 810 genuine warm BQ samples, 570 failed ordered-page equality.
The reduction script rejects incomplete sample matrices. See the [complete p50/p95,
recall and equality tables](binary-quantization-summary.md) and [raw samples/query
plan](binary-quantization.csv). p50 is the median, p95 is nearest rank ceil(0.95*n).

Primary query (seed 4242), first page, warm p95; parentheses show oracle hits out of
the expected page size. Each cell summarizes 30 samples:

| View | Eligible | Production ms | BQ 200 ms (hits) | BQ 800 ms (hits) | BQ 4096 ms (hits) |
|---|---:|---:|---|---|---|
| Global | 100000 | 50.697 | 21.226 (56/200) | 106.348 (107/200) | 156.361 (166/200) |
| Inbox | 17500 | 106.051 | 39.170 (67/200) | 64.456 (131/200) | 89.326 (191/200) |
| Starred | 31 | 0.239 | 0.890 (31/31) | 0.677 (31/31) | 0.723 (31/31) |
| Snoozed | 2500 | 18.944 | 19.976 (100/200) | 23.706 (178/200) | 35.704 (200/200) |
| Archive | 5000 | 54.418 | 46.526 (95/200) | 48.867 (160/200) | 74.026 (200/200) |
| All-label | 25000 | 150.740 | 41.533 (70/200) | 58.801 (131/200) | 98.543 (183/200) |
| Empty | 0 | 31.618 | 30.402 (n/a) | 31.465 (n/a) | 30.679 (n/a) |

For additional global seeds 17/90001, budget 200 yields p95 **17.230/19.302ms** and
only **51/49 of 200** oracle hits. Budget 4096 yields **171.131/146.254ms** and
**163/178** hits. All 270 global warm BQ samples fail ordered equality. No global BQ
configuration meets both sub-15ms p95 and equivalence. Increasing budgets is not a
measured speed win here; Hamming selection and int8 point lookups are included, but
component-level timings were not collected, so their individual costs are not inferred.

Starred is exhaustive at every budget, and Snoozed is exhaustive at 4096; both are slower
than their production controls. Archive at 4096 happens to match this query's top 200,
but neither its 74.026ms p95 nor a single-query observed match certifies a universal
exact path. Empty cases still spend ~30ms enumerating eligibility and do not establish
dense-search performance. Global deep exact p95 is 354.868ms; BQ-labeled fallbacks
remain 353.336–376.515ms and provide no acceleration. Scoped/native deviations persist.
Even the fresh global production control misses the 50ms p95 bound for seeds 4242 and
90001 (50.697/51.877ms); these are recorded misses, not adjusted targets.

Machine: Linux 6.18.33.2 WSL2 x86_64, AMD Ryzen 7 7700, five exposed vCPUs,
Rust 1.99.0, default optimized release profile, SQLite 3.45.3, SQLCipher 4.5.7 community,
sqlite-vec 0.1.9. Database temporaries use the existing `/tmp` tmpfs. The machine was
not CPU pinned; no compilation or native fixture ran concurrently with the accepted
sweep. These distributions describe this shared reference machine, not portable guarantees.

Building the experimental index took **920.030ms** including checkpoint. The encrypted
file grew from **120,233,984 to 126,529,536 bytes** (+6,295,552, about 5.24%; WAL/SHM
excluded after truncate checkpoint). Logical sign payload alone is 4.8MB; vec0 metadata
adds overhead. The [whole-process report](binary-quantization-process.txt) records
2:55.83 elapsed and 81,744KiB peak RSS, including seeding/oracle work. This is not an
incremental connection-cache memory measurement or a production memory-budget pass.

## Adversarial oracle probes

The example's three Rust tests assert the following, using actual sqlite-vec binary
selection, scalar reranking, hydration and an independent Rust oracle:

- **Sign loss:** a unique closest vector differs in 383 low-magnitude signs, whereas
  200 worse vectors share all query signs. Budget 200 returns a full, wrong page and
  omits the best cosine neighbor. Increasing the budget to cover the population fixes
  this fixture; it does not establish a universal budget.
- **Zero stored norm, missing and orphan embeddings:** scalar cosine yields SQL NULL
  for a zero vector, ordered first by the scoped contract (k-021). Its Hamming code can
  exclude it from the first 200. Missing/orphan embeddings are excluded at index build;
  complete coverage restores the expected order. Index lifecycle after writes is outside
  this immutable experiment.
- **Oversized Hamming ties and zero query:** 5,000 same-sign vectors exceed the 4,096
  cap. An omitted vector can become the exact nearest neighbor by changing magnitudes
  without changing its binary code. A zero query makes every exact distance NULL,
  requiring canonical-ID order; a 4,096-candidate shortcut underfills the deep 200-row
  page to 96. Exhaustive fallback is required.

## Decision and limits

Reject production adoption under the existing oracle-equivalence requirement. Keep
released schema v2, public traits, DTOs, exact tie handling and NULL-first scoped order.
Do not promote an approximate top-k to an exact path based on a full output page,
observed recall, a finite reranked gap, or widening up to 4,096. Automatically verifying
all omitted vectors requires an exact scan; this experiment measures deep fallback,
not an accelerated certified-bound algorithm.

For a future approved investigation, prioritize an exact contiguous eligible-vector
scan or provably admissible residual bounds that cover zero norms, missing/orphan
vectors and arbitrarily large ties. Any persistent index needs an append-only migration,
transactional write/delete/account reconciliation, encrypted storage growth and memory
measurements. An approximate mode would require separately approved product semantics,
explicit recall tradeoffs and representative learned embeddings; it is not authorized
by this evaluation. No ANN graphs are introduced.

No lexical, hybrid, IPC/WebKit or scrolling speedup is claimed. Approximate semantic
candidates can also change RRF ranks; unchanged hybrid equivalence cannot be inferred.
The mailbox/native/frame-cadence deviations recorded in [04.1.7] remain open as measured
deviations, with ADR-008 targets unchanged.

## Repository validation

Workspace check/test/fmt and all-target Clippy with warnings denied passed offline:
222 workspace Rust tests plus all three example probes. Desktop check has zero
errors/warnings and all 133 frontend tests pass. Policy scanner regressions (five),
tracked/new-file secret/PII review, Svelte syntax invariant and diff whitespace pass.
The summary reducer verifies all 1,984 raw query rows and 64 complete groups.

Existing native sysroot/toolchains follow k-001/k-006. Loopback Rust tests and policy
scanner child processes ran outside the restrictive sandbox. An initial temporary Cargo
cache exhausted `/tmp`; moving that cache to ignored workspace build storage and using
workspace build temporaries resolved validation. Those failed build attempts are not
benchmark samples; the accepted timing process started only after validation finished.
