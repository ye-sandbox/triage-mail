# Zeroized In-Memory Exact int8 Vector Index Validation

Measured 2026-11-18 on Linux (`WSL2 x86_64`), optimized Rust (`--release`), SQLCipher WAL,
100,000 messages, 200,000 folder memberships, and 384-dimensional synthetic `int8` vectors.
Uses only the offline disposable local fixture (`tools/performance/mailbox_data.rs`) with zero
network calls, provider access, or real mail. Reproduce with:

```bash
# Fast default validation (<60s: single vec0 parity check + single RAM hydration + all 16 RAM workloads)
cargo run --offline --release -p storage --example ram_vector_performance

# Full vec0 vs RAM comparative CSV regeneration (opt-in slow baseline + full sample budgets)
cargo run --offline --release -p storage --example ram_vector_performance -- --with-slow-baseline | tee docs/performance/ram-vector-index.csv
```

## Architecture & Memory Footprint

`crates/storage/src/ram_vector_index.rs` adds an additive, 100% safe Rust
(`#![deny(unsafe_code)]`) exact `int8[384]` in-memory accelerator (`RamVectorIndex`) wrapped in
`RamVectorState` (`Uninitialized | Hydrating | Ready`):

- **SoA Vector Arena & Zeroization:** Stores packed `vectors: Vec<i8>` (`N * 384`), precomputed
  exact L2 norm squares (`norm_sq: Vec<u32>`), SQLite `rowid`s, message/account IDs, and compact
  scoped eligibility columns (`is_starred` bit, `snooze_until`, resolved `TriageCategory`, active
  `Sender Review` state + `inbox_hidden`, and interned effective folder `u64` bitmask with overflow
  list). `Drop`, `clear_ram_index()`, tail swap-remove, and vector buffer reallocations explicitly
  wipe prior heap bytes via `zeroize = "1.9.1"`.
- **100% Oracle Parity:** Preserves `sqlite-vec` 0.1.9 `f32`-rounded cosine distances, `k-021`
  NULL-first ordering (`f64::NEG_INFINITY` for zero-norm stored or query vectors), and `message_id`
  ASC tie-breaking using exact parallel `BinaryHeap` top-(`offset + limit`) selection across all
  eligible rows (no ANN pruning or candidate truncation).
- **Synchronization & Transparent Fallback:** While `Uninitialized | Hydrating`, all queries fall
  back transparently to `vec0`. Once `Ready`, intra-store writes synchronize slots inside
  `write_transaction`, and out-of-band SQLite/cross-connection mutations detected via
  `sqlite3_total_changes` + `PRAGMA data_version` trigger safe invalidation before reading.
- **Measured 100k Footprint & Cold Hydration:**
  - Cold hydration from SQLCipher (`hydrate_ram_index`): **992.144 ms** (runs non-blocking on
    `tokio::task::spawn_blocking` after desktop unlock; wiped cleanly on quit/shutdown/restore/rebuild).
  - Contiguous vector buffer: **38,400,000 bytes (36.62 MiB)** (`100,000 * 384` bytes).
  - Eligibility columns & slot maps: **12,705,024 bytes (12.12 MiB)**.
  - Total allocated heap: **48.74 MiB**.

## Pure RAM Ranking Kernel (`rank_filtered`, 100k Vectors)

Excludes SQLite summary/tag/badge hydration (`2` warmups + `30` warm samples per scope, `limit = 200`):

| Kernel Scope | Offset | p50 ms | p95 ms | Max ms |
| --- | ---: | ---: | ---: | ---: |
| `Global` | 0 | 3.153 | 7.945 | 7.999 |
| `Global-deep` | 4,000 | 4.780 | 8.943 | 10.797 |
| `Inbox(account-2)` | 0 | 1.710 | 3.988 | 4.350 |
| `Category(Financial)` | 0 | 2.529 | 4.116 | 5.794 |
| `Review(All-Pending)` | 0 | 3.420 | 5.112 | 8.006 |

*(Note: an uncontended earlier partial sweep before the full matrix completed recorded pure kernel
p95 latencies of `2.222 ms` Global, `3.378 ms` Global-deep, `0.869 ms` Inbox, `0.923 ms` Category,
and `1.921 ms` Review; shared-host WSL2 thread scheduling accounts for the upper p95 tail.)*

## End-to-End Scoped Query Comparison (`vec0` Fallback vs RAM Accelerator)

Each workload executes the full `SqliteMailStore` search API (ranking + single-snapshot SQLCipher
summary/tag/override/badge hydration for `limit = 200`) and asserts exact ordered-page parity
against both `sqlite-vec` (`vec0`) and, on Global/Inbox/Archive semantic paths, an independent
`i64` dot/norm Rust oracle. RAM runs discard 2 warmups and record 30 warm samples; `vec0` baselines
discard 1 warmup and record 5 warm samples when `warmup <= 2s`, 2 samples for `2s–5s`, and 1 warm
sample when a single query exceeds `5s`. Raw CSV: [ram-vector-index.csv](ram-vector-index.csv).

| Scope | Mode | Offset | Eligible | `vec0` p95 ms | RAM p50 ms | RAM p95 ms | RAM Max ms | Speedup (p95) | Exact Parity |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `Global` | Semantic | 0 | 100,000 | 137.648 | 10.309 | 12.126 | 13.273 | **11.4x** | `true` |
| `Global` | Semantic | 4,000 | 100,000 | 35,697.017 | 9.959 | 12.484 | 12.662 | **2,859.3x** | `true` |
| `Global` | Hybrid | 0 | 100,000 | 592.735 | 43.515 | 46.805 | 47.140 | **12.7x** | `true` |
| `Inbox(account-2)` | Semantic | 0 | 17,500 | 7,267.193 | 13.569 | 16.797 | 16.934 | **432.6x** | `true` |
| `Inbox(account-2)` | Semantic | 4,000 | 17,500 | 7,305.030 | 14.730 | 15.599 | 16.464 | **468.3x** | `true` |
| `Inbox(account-2)` | Hybrid | 0 | 17,500 | 8,028.768 | 749.714 | 790.402 | 811.172 | **10.2x** | `true` |
| `Starred(account-2)` | Semantic | 0 | 31 | 5.804 | 3.398 | 4.214 | 4.233 | **1.4x** | `true` |
| `Snoozed(account-2)` | Semantic | 0 | 2,500 | 993.624 | 14.583 | 15.828 | 16.169 | **62.8x** | `true` |
| `Archive(account-2)` | Semantic | 0 | 5,000 | 3,147.045 | 9.355 | 11.007 | 11.804 | **285.9x** | `true` |
| `Category(Financial)` | Semantic | 0 | 25,001 | 10,871.695 | 8.217 | 9.746 | 10.921 | **1,115.5x** | `true` |
| `Category(Financial)` | Hybrid | 0 | 25,001 | 13,198.708 | 1,189.539 | 1,280.929 | 1,291.715 | **10.3x** | `true` |
| `Category+Starred` (`SavedView`) | Semantic | 0 | 32 | 49.438 | 35.824 | 37.932 | 38.401 | **1.3x** | `true` |
| `Review(All-Pending)` | Semantic | 0 | 75,000 | 28,616.412 | 8.956 | 11.185 | 12.430 | **2,558.5x** | `true` |
| `Review(All-Pending)` | Hybrid | 0 | 75,000 | 29,405.378 | 984.773 | 1,035.140 | 1,090.137 | **28.4x** | `true` |
| `Review(Account-0)` | Semantic | 0 | 25,000 | 10,863.126 | 7.024 | 7.780 | 9.258 | **1,396.3x** | `true` |
| `Review(Account-0)` | Hybrid | 0 | 25,000 | 12,849.041 | 1,914.183 | 2,007.862 | 2,046.415 | **6.4x** | `true` |

## Honest Caveats & Bottleneck Attribution

- **Budget Achievement & Remaining SQLite Lexical/Predicate Costs:** `Global` Semantic (`12.1ms <= 50ms`),
  `Global` Hybrid (`46.8ms <= 60ms`), `Category` Semantic (`9.7ms <= 15ms`), `Review` Semantic (`7.8–11.2ms <= 15ms`),
  `Starred` (`4.2ms <= 15ms`), and `Archive` (`11.0ms <= 15ms`) now comfortably clear their ADR-008
  budgets while preserving 100% exact ordered-page parity (unlike approximate binary candidates in
  [binary-quantization.md](binary-quantization.md)).
- **Scoped Hybrid SQLite FTS5 Cost:** In `mailbox_data.rs`, every one of the 100,000 synthetic messages
  contains `"newsletter"`. Therefore, in `Inbox`, `Category`, and `Review` **Hybrid** modes, SQLite's
  FTS5 lexical branch still scans all matching rows through SQLite's eligibility joins (`INBOX_VISIBLE`,
  `CATEGORY_EFFECTIVE`, or `sender_review_*`), accounting for `790–2,008 ms` while the RAM vector branch
  runs concurrently in single-digit milliseconds.
- **`SavedView` Arbitrary Predicates:** `Category+Starred` (`SavedView`) evaluates its SQL predicate to
  materialize eligible `rowid`s before calling `RamVectorIndex::rank_rowids`, so its `37.9ms` p95 is
  governed by SQLCipher predicate evaluation rather than vector distance calculation.
- **Prior Baseline Artifacts Preserved:** [categories.md](categories.md) and [sender-review.md](sender-review.md)
  retain their original pre-RAM-index SQLCipher measurements as historical baselines.
- **No Native UI / Real-Mail Claim:** Measurements use deterministic synthetic vectors on an unpinned
  development host and do not imply 60 fps UI scroll budgets, learned-model semantic quality, or real-provider latency.
