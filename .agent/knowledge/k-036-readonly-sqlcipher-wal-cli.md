---
id: "k-036"
title: "Read-only SQLCipher cache inspection requires WAL-aware snapshots"
date: 2026-11-05
tags: ["sqlcipher", "wal", "cli", "vault"]
severity: "medium"
affected_components: ["crates/storage", "crates/vault", "apps/cli"]
---

# Read-only SQLCipher cache inspection

## Problem

`SqliteMailStore::open` configures WAL and runs migrations. Desktop bootstrap can
also provision a missing database key and start remote workers. Neither is suitable
for an offline inspection command that must fail without initializing an installation.
SQLite immutable mode can miss live WAL data and is not a substitute for read-only open.

## Contract

- `CachedMailReader` opens with READONLY, sets SQLCipher key before database queries,
  then query_only, memory temporary storage and bounded busy timeout. Its connection
  stays private; no migration or journal-mode pragma runs.
- Validate the full supported migration sequence in the same transaction as each
  command. This prevents mixed snapshots during ranking/hydration and later reads
  after another process changes schemas. Transaction drop rolls back read snapshots.
- Read committed WAL normally. Existing WAL/SHM readability or directory permission
  for shared-memory sidecars may be required. No logical writes does not guarantee
  unchanged filesystem metadata/sidecars. Never make a live standalone main-file copy.
- Read existing vault entries only. Explicit fallback unlock must not silently prefer
  a keyring entry; recovered installations always require fallback selection. The new
  existing-file reader has no setter and never creates a missing vault.
- Bound text bytes in SQL before Rust allocation. Validate UTF-8 once and truncate
  only incomplete final scalars; reject malformed interior bytes. Do not repeatedly
  rescan an invalid string while removing individual bytes.

## Evidence and limits

Offline storage tests retain an encrypted writer, disable automatic checkpoints and
compare lexical results, then commit another write from a thread and read it through
the retained reader. UPDATE/DELETE/CREATE fail even if query_only is disabled because
the underlying file was opened READONLY. Missing paths and incompatible schemas are
not repaired. CLI fixtures verify fallback/recovery gating, queued work preservation,
unread state, oversized output and escaped control bytes.

Build/package dependency inspection establishes GUI independence, not native keyring
interoperability across platforms. Direct symlink checks do not defend against hostile
concurrent path replacement; the app-data directory must be trusted and private.
The 4 MiB output ceiling does not bound all internal lexical-search hydration memory.
