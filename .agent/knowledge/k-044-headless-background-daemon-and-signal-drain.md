---
id: "k-044"
title: "Headless Background Daemon and Signal-Safe Drain"
date: 2026-11-07
tags: ["daemon", "scheduler", "cli", "signals", "ndjson"]
severity: "medium"
affected_components: ["crates/sync-runtime", "apps/cli", "docs/cli.md"]
---

# Headless Background Daemon and Signal-Safe Drain

- `run_daemon` in `crates/sync-runtime::daemon` provides persistent background synchronization
  without GUI sidecars. It periodically runs `sync_accounts` on a configurable tick cadence
  (`--interval-secs 1..86400`, default 60s).
- Graceful drain is coordinated via `tokio::sync::watch`: when a termination signal (`SIGINT` or
  `SIGTERM`) is received, the watch channel signals cancellation. If the daemon is sleeping between
  intervals, `tokio::select!` cancels the sleep immediately; if a sync pass is active, the in-flight
  batch commit finishes before breaking the loop, preventing partial transaction writes.
- Continuous `WriterOwnership` is held across the entire daemon lifetime, preventing conflicting
  concurrent writes from GUI or CLI processes. Contention on startup exits with deterministic status 7.
- Streaming lifecycle updates are emitted to stdout as newline-delimited JSON (NDJSON) lines
  (`started`, `sync_tick`, `stopping`, `stopped`), exposing iteration numbers, durations, and message
  counts without leaking credentials, tokens, or PII.
- Bounded testing via `--max-iterations <N>` enables deterministic offline test harnesses without
  fragile timing sleeps or hanging subprocesses.
