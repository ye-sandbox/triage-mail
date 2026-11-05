---
id: "k-033"
title: "Logical WAL snapshots and isolated recovery vault selection"
date: 2026-11-04
tags: ["backup", "sqlcipher", "wal", "vault", "recovery"]
severity: "high"
affected_components: ["crates/storage", "crates/vault", "apps/desktop/src-tauri"]
---

# Logical snapshot and recovery invariants

- Pin every table to one SQLite read transaction. Copying `mail.db` without WAL is
  not a backup; checkpointing then releasing the lock before copying also races.
  The concurrency regression writes through a second connection after the read
  snapshot begins and proves the restored view excludes that later commit.
- Fresh databases contain singleton `conversation_epoch` and `alert_schedule_cursor`
  rows. Exclude these from the empty-target check, but never copy them or bypass
  occupancy checks on actual user data/configuration tables.
- Copy only explicit known table rows, dependency-ordered. Bind values; never execute
  imported SQL, schema text or table names. No database file is decrypted to disk.
  Account connection configs, vault references and all pending work stay out.
- AutoSecretStore consults the OS keyring before fallback. Using it for recovery can
  select an unrelated existing application key or overwrite a keyring entry. The
  `recovered-local-vault` marker instead forces explicit fallback-only unlock.
- Stage recovery under a fresh random sibling directory, private mode, with a fresh
  database key in a fresh encrypted local vault. Validate the entire final frame,
  schema, foreign keys and database integrity before publication. Crash-abandoned
  stages remain inert. Never infer recovery completion from an incomplete stage.
- Cached-only account list entries use an empty IMAP host. WorkerSupervisor must
  skip them. Explicit reconnect resolves exact email under account_gate, preserving
  IDs and mail; it cannot replace an existing configured connection.
- Portable definitions replace only structural IDs. User-authored names, queries,
  sender predicates or body conditions can themselves contain private material.
  Always show the readable-data disclosure. Require every structural mapping;
  missing references never become all-account scopes. All-account intent requires
  preview confirmation. Imports remain disabled and do not enqueue history.
- Native portal signals may arrive immediately. Subscribe to the token-derived
  Request.Response path before OpenFile/SaveFile. Portal availability is not proved
  by offline tests; no helper-process or network fallback exists.

## Verification

Use repository workspace tests and frontend tests. Focused regressions live in
`crates/vault/src/backup.rs`, `crates/storage/src/{backup,portable}/tests.rs`, desktop
backup tests, `tests/ipc_service.rs` and `src/lib/backup/backup.svelte.test.ts`.
See ADR-021 and `docs/backup.md` for format limits and recovery operation.
