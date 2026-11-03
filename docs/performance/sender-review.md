# New-sender review fixture evidence

Measured on 2026-11-02 with the existing disposable encrypted 100,000-message /
200,000-membership fixture (four accounts, 384-dimensional int8 vectors). This is a
storage example, with no provider access, deliveries, downloads or native UI run.

```bash
cargo run --offline --release -p storage --example sender_review_performance
```

Local sender-history backfill processed 100,000 rows in **4,115.029ms** in a tight
loop. Desktop yields between batches of at most 300 rows; that tight-loop duration
is not an end-to-end desktop initialization promise. Historical observations create
one exact sender per account in this fixture. Three accounts are explicitly set to
Pending, one Accepted, after conservative history seeding; no automatic history
review or initial-sync behavior is inferred from these timing inputs.

Each workload requests the first 200 rows, discards two warm-ups and records 30 warm
samples. Lexical uses blank text; semantic/hybrid use `newsletter` and a deterministic
synthetic query vector. The final run began after workspace/UI tests and build
completed. Raw aggregates: [sender-review.csv](sender-review.csv).

| Mode | Workload | Eligible messages | p50 ms | p95 ms | Maximum ms |
| --- | --- | ---: | ---: | ---: | ---: |
| Lexical | All review | 75,000 | 0.627 | 0.772 | 0.791 |
| Lexical | Account review | 25,000 | 36.875 | 38.857 | 43.689 |
| Semantic | All review | 75,000 | 374.787 | 390.544 | 405.376 |
| Semantic | Account review | 25,000 | 130.117 | 133.568 | 133.719 |
| Hybrid | All review | 75,000 | 497.476 | 524.779 | 537.956 |
| Hybrid | Account review | 25,000 | 186.189 | 197.102 | 251.057 |

Both blank lexical p95 workloads meet the existing 40ms lexical budget. Global
semantic exceeds 50ms, scoped semantic exceeds 15ms, and both hybrid workloads exceed
60ms. Those targets remain unchanged. Review adds correlated native eligibility to
the existing exact ranker; results do not establish arbitrary-sender-distribution,
keyword, deep-page, ordinary Inbox or native first-frame performance. The fixture's
single sender per account is especially narrow. Existing [mailbox](mailbox-validation.md)
and [category](categories.md) timing deviations remain open.

Every timed row is checked for account/state membership and canonical folder/UID
identity. The storage regressions additionally check complete page membership without
duplicates across lexical, semantic and hybrid review searches; ordinary Inbox hiding
and explicit saved-view access; cross-account/stale-write rejection; history frontiers,
straddling sync batches, deduplication, transaction rollback and encrypted restart.
These tests reuse existing exact ranking machinery; this timing example is not an
independent full-order oracle. No native 60fps or live provider compatibility claim is
made. All tests use reserved-domain synthetic data.
