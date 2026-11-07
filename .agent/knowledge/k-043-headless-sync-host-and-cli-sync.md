---
id: "k-043"
title: "Headless One-Shot Sync Host and CLI Sync Command"
date: 2026-11-07
tags: ["sync-runtime", "cli", "headless", "ownership", "imap"]
severity: "medium"
affected_components: ["crates/sync-runtime", "apps/cli", "docs/cli.md"]
---

# Headless One-Shot Sync Host and CLI Sync Command

- `HeadlessSyncHost` in `crates/sync-runtime` implements `SyncHost` without GUI or desktop
  runtime dependencies. It resolves account connection parameters (`AccountConnectionConfig`)
  from `SqliteMailStore` and credentials from `vault::SecretStore`.
- IMAP network transport is decoupled behind the `SyncTransport` trait, allowing hermetic
  offline testing with mock folders and messages while the production `DefaultSyncTransport`
  uses `DefaultImapConnector` and TLS 1.3 / loopback Proton Bridge policies.
- `apps/cli` provides the `sync` subcommand (`--account <OPAQUE_ID>`), strictly acquiring
  `sync_runtime::ownership::WriterOwnership` prior to opening the local installation via
  `sync_runtime::bootstrap::open`.
- Lock contention returns exit code 7 (`installation already owned by another writer`),
  preserving the exclusive writer contract against concurrent GUI or CLI instances.
- Account sync outcomes and run durations serialize to bounded JSON on stdout without
  leaking passwords, tokens, or PII. Failures exit with deterministic status codes
  (0 = success, 2 = args, 3 = vault, 4 = cache/storage, 5 = not found, 6 = output,
  7 = lock contention, 8 = remote IMAP error).
