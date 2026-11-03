# Conversation indexing and grouped search

Measured 2026-11-03 with an optimized offline Rust example, a disposable SQLCipher
file, 100,000 synthetic messages, four accounts, 384-dimensional int8 vectors and
200,000 folder memberships. Every account has 1,000 conversations of 25 canonical
records each, linked through shared missing ancestors; members span Inbox/Archive.
Account Inbox eligibility also excludes active snoozes, leaving 17 or 18 matching
members per group. Each group page contains 200 representatives.

Reproduce:

```bash
cargo run --offline --release -p storage --example conversation_performance
```

There are two warmups followed by ten samples per workload; p95 is the nearest-rank
95th percentile (the maximum at this sample count). Times include native eligibility,
complete eligible branch rankings, grouping/counting and final-page hydration. They
exclude fixture generation, IPC, webview work and body/attachment loading. Concurrent
build activity and the machine's filesystem/cache state limit comparability.

| Mode | Global p50 / p95 (ms) | Account Inbox p50 / p95 (ms) |
| --- | ---: | ---: |
| Lexical, blank query | 173.989 / 215.069 | 152.423 / 175.117 |
| Exact semantic | 780.706 / 1029.961 | 326.671 / 378.748 |
| Hybrid, complete-branch RRF | 971.814 / 1170.017 | 387.718 / 455.225 |

Aggregate samples are in [conversations.csv](conversations.csv). Assertions check
representative count, complete cached membership and pre-group Inbox eligibility.
These timings exceed existing search targets, which remain unchanged. Hybrid grouped
search uses complete eligible branches rather than message mode's bounded candidate
windows; its latency/ordering should not be compared as an identical search workload.
Other predicates, distributions, deep pages and cold-cache/native UI performance are
not established by this fixture. No claim of native 60fps is made.

Header staging and component publication took 15,463.904 ms across 337 ticks, excluding
worker delays. The slowest tick was 297.980 ms. Staging is limited to 300 records per
tick; final consolidation reads compact metadata for an entire dirty account and
publishes its components under the database lock. That final operation can stall
other database users. A single arrival/header change/deletion can require account-wide
consolidation again. The current correctness-first implementation does not meet a
sub-15ms mutation/indexing budget for these accounts. Incremental component maintenance,
SQL group ranking and narrower metadata scans are future optimization opportunities,
not delivered behavior. The live runner yields between staging ticks and grouping
remains unavailable until the relevant account index is ready.

No provider access, model downloads, deliveries, credentials or attachment binaries
were used. Earlier mailbox/category/review performance deviations remain in force.
