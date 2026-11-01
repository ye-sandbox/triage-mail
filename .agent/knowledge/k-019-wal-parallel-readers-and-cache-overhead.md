---
id: "k-019"
title: "SQLite WAL Parallel Readers, Per-Connection Page Cache Overhead, and Exact Vector Limits"
date: 2026-10-31
tags:
  - "sqlite"
  - "sqlcipher"
  - "wal"
  - "concurrency"
  - "hybrid-search"
  - "vector-knn"
severity: "medium"
affected_components:
  - "crates/storage"
  - "crates/storage/src/search.rs"
  - "crates/storage/src/repository.rs"
symptoms:
  - "Sequential hybrid search latency is additive (~75-79ms warm p50)"
  - "Misconception that connection pooling violates SQLite ACID invariants"
  - "Global exact vector search unable to meet <15ms on CPU without indexing"
---

# [k-019] SQLite WAL Parallel Readers, Per-Connection Page Cache Overhead, and Exact Vector Limits

## Problem & Context

1. **Theoretical vs Physical Limits of Global Exact Vector Search:**
   Scanning 100,000 `int8[384]` embeddings on CPU requires streaming 38.4 MB of raw vector payloads through memory and SQLCipher AES-256 page decryption. On a single modern core, this linear exact scan physically requires ~42–46ms warm. A blanket `<15ms` target across all query modalities (lexical, global semantic, hybrid) was set before 100k native measurements and is physically infeasible for global exact scans without indexing or lossy approximations.

2. **The ACID Pooling Misconception ([90.4]):**
   A previous architectural note asserted that SQLite connection pooling was unsafe for ACID semantics. In SQLite WAL mode (`PRAGMA journal_mode = WAL;`), this is incorrect: multiple read-only connections are fully ACID-compliant, providing concurrent thread-safe snapshot isolation without write contention or locks blocking readers.

3. **Additive Hybrid Search Latency:**
   Executing lexical (FTS5 BM25 ~32ms) and semantic (`vec_messages` ~46ms) candidate collection sequentially on a single connection resulted in ~79ms p50 latency.

---

## Technical Findings & Concurrency Model

### 1. SQLite WAL Concurrency & Snapshot Isolation
In WAL mode, readers do not block writers, writers do not block readers, and multiple readers do not block each other. Each connection sees a consistent point-in-time snapshot of the database at the time its transaction or statement began.

### 2. Thread Safety in Rust with `rusqlite`
- `rusqlite::Connection` implements `Send`, but deliberately does **not** implement `Sync` (`sqlite3*` handle is not thread-safe for concurrent method calls).
- Shared references `&Connection` **cannot** cross thread boundaries (`&T: Send` requires `T: Sync`).
- Exclusive references `&mut Connection` or distinct owned `Connection` instances **can** be sent across threads (`impl<T: Send> Send for &mut T`).
- `std::thread::scope` enables zero-unsafe, zero-dependency parallel execution by passing `conn_lex: &Connection` on the current thread and `conn_sem: &mut Connection` into a scoped spawned thread.

### 3. Empirical Page Cache Overhead (VmRSS)
Each SQLite connection maintains its own independent B-tree page cache (`PRAGMA cache_size = -65536`, 64 MiB soft target).
Empirical measurement over 100,000 messages in SQLCipher:
- Baseline Process RSS: **72.31 MiB**
- Single connection warmed: **77.15 MiB**
- Secondary connection opened: **77.15 MiB** (zero overhead on connect)
- Secondary connection warmed with full 100k vector scans: **102.70 MiB**
- **Net Memory Cost of Secondary Connection:** **~25.5 MiB** (not 64 MiB, due to page sharing and allocator reuse).
- Overall process RSS remains well below the 110 MiB threshold (~102.7 MiB).

### 4. Latency Reduction in Hybrid Search
- Sequential Hybrid p50: **78.96 ms** (p95: 95.13 ms)
- Parallel Hybrid p50: **52.39 ms** (p95: 73.22 ms)
- **Improvement:** **33.7% faster** (~26.6 ms wall-clock reduction per query) with 100% exact result equivalence to sequential execution.

---

## What NEVER to Do
- ❌ **Anti-Pattern:** Attempting to share a single `&Connection` across multiple threads without synchronization.
- ❌ **Anti-Pattern:** Adopting approximate nearest neighbor (ANN) graph indexes (HNSW, IVF) in `v0.1` to chase artificial sub-15ms global targets, risking index corruption, non-deterministic recalls, and external C/C++ sidecars.
- ❌ **Anti-Pattern:** Assuming SQLite connections share page caches. Each connection allocates its own cache based on `cache_size`.

---

## Approved Pattern & v0.2 Roadmap
1. **Parallel Read Support in `SqliteMailStore`:**
   - Disk-backed stores open an optional secondary read connection.
   - `SearchEngine::search(Hybrid)` attempts `try_lock()` on the secondary connection and dispatches `execute_hybrid_search_parallel`.
   - If secondary connection is busy or unavailable (in-memory databases), gracefully fall back to sequential `execute_hybrid_search`.
2. **v0.2 Binary Quantization Roadmap:**
   - Pre-filter candidates using 1-bit scalar quantization (Hamming distance on 48-byte packed vectors via bitwise `POPCNT`).
   - Rerank top 200 candidates with exact `int8` dot products to achieve sub-15ms global semantic search in v0.2.
