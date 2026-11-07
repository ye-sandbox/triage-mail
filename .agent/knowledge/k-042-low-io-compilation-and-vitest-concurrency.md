---
id: "k-042"
title: "Low-I/O Compilation Profiles and Vitest Serial Concurrency"
date: 2026-11-07
tags: ["cargo", "profiles", "vitest", "disk-io", "performance"]
severity: "medium"
affected_components: ["Cargo.toml", "apps/desktop/vite.config.ts"]
---

# Low-I/O Compilation Profiles and Vitest Serial Concurrency

- Default Rust debug level (`debug = 2` / `debug = true`) produces multi-gigabyte DWARF
  sections (`.debug_info`, `.debug_types`, `.debug_loc`), saturating rotational mechanical
  disk (HDD) queues during continuous test and compilation cycles.
- Setting `debug = 1` in `[profile.dev]` and `[profile.test]` preserves `.debug_line`
  and `.debug_info` line tables. File names and line numbers remain fully resolved in
  `RUST_BACKTRACE=1` stack traces and panic diagnostics while omitting heavy variable/type
  symbols, drastically shrinking intermediate build artifacts.
- Vitest's default thread pool executes worker suites in parallel across multiple CPU cores.
  On rotational disks, simultaneous random reads across separate jsdom module bundles trigger
  severe I/O queue head thrashing (>300 IO/s), causing unpredictable test latency spikes.
- Configuring `pool: 'forks'`, `maxConcurrency: 1`, and `fileParallelism: false` in
  `apps/desktop/vite.config.ts` bounds execution to sequential file-level runs with fork
  isolation, eliminating concurrent disk head contention while keeping test suite execution
  stable and deterministic.
