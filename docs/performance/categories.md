# Local category search validation

Measured 2026-11-02 on this Linux development host, optimized Rust, SQLCipher WAL,
100,000 messages / 200,000 memberships / 384-dimensional synthetic int8 vectors.
No provider calls, downloads or real mail. Reproduce with:

```bash
cargo run --offline --release -p storage --example category_performance
```

The shared disposable fixture has four accounts and identical newsletter text. Local
backfill classified all 100,000 rows in 4,721.357ms (tight offline loop; desktop yields
between bounded batches, so this is not a desktop completion-time claim). A sender
override assigns Finance to account-0; one message correction adds a second account's
message. Broad Finance therefore has 25,001 eligible messages. Adding Starred leaves
32. Each workload discards two warmups and records 30 warm first-page samples.

| Mode | Predicate | Eligible | p50 ms | p95 ms | Max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| Lexical, blank | Category | 25,001 | 1.829 | 2.120 | 2.595 |
| Lexical, blank | Category + Starred | 32 | 17.469 | 18.250 | 18.457 |
| Semantic | Category | 25,001 | 259.338 | 262.892 | 264.127 |
| Semantic | Category + Starred | 32 | 8.468 | 9.237 | 9.246 |
| Hybrid | Category | 25,001 | 423.723 | 451.471 | 462.031 |
| Hybrid | Category + Starred | 32 | 22.281 | 25.273 | 26.952 |

Broad semantic and hybrid workloads **miss** ADR-008's applicable 15ms filtered semantic
and 60ms parallel / 95ms sequential hybrid budgets. The shared scoped ranker uses an
exact eligible scan; category lookup adds correlated metadata joins. Narrow semantic
and hybrid and both blank lexical workloads meet their corresponding budgets on this
fixture. None of these observations changes the budgets or resolves previous mailbox
and native UI deviations. No native first-frame, 60fps, memory or learned-model relevance
claim follows from these measurements. Nonblank lexical searches are not measured here.

The example checks the entire narrow membership set independently and checks broad
first-page eligibility, category and canonical identity on every sample. It does not
certify arbitrary broad ordered pages. Deterministic storage regressions separately
verify complete 300-result lexical/semantic/hybrid page sequences, duplicate membership
handling, empty account-scoped results and saved-view equivalence. Existing scoped
ranking regressions retain exact ties, zero-norm behavior and mailbox semantics.

Raw aggregate measurements: [categories.csv](categories.csv). Values describe one process
run with synthetic data on shared hardware; they are not universal latency guarantees.
