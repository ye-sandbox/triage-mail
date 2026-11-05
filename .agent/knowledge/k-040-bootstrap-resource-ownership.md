---
id: "k-040"
title: "Local Bootstrap Resource Ownership"
date: 2026-11-05
tags: ["runtime", "bootstrap", "locking", "vault", "storage"]
severity: "high"
affected_components: ["crates/sync-runtime", "apps/desktop/src-tauri"]
---

# Local Bootstrap Resource Ownership

- Keeping WriterOwnership only in a service does not cover independently cloned
  storage/vault handles. Each resource returned by shared bootstrap must own a guard.
  OwnedStorage keeps its repository private, exposes borrowed Deref access and has
  no consuming extraction or mutable replacement API. Drop the resource before its
  guard. The same rule applies to detached blocking credential operations.
- A valid guard for another installation is insufficient. Compare the canonical
  parent and literal final component before any local mutation. Matching must not
  create directories. Canonical parent aliases are allowed; direct installation
  symlinks and traversal remain refused. This is cooperative identity validation,
  not race-proof isolation from hostile filesystem replacement or bind aliases.
- The bootstrap caller still needs a retained guard across failures and retries.
  Returned resource guards protect success lifetimes only. Keep the in-process
  recovery gate inside any uncancelable blocking task as documented in k-039.
- Recovery markers force fallback-only unlock even if an OS keyring key exists.
  Never replace missing/malformed keys for an existing mail.db. Preserve the existing
  AutoSecretStore behavior for ordinary passphrase unlock rather than interpreting
  the supplied passphrase as a new fallback-only policy.
- Shared bootstrap must stop at local storage/vault opening. Generation setup,
  orphan cleanup, reconciliation, remote/provider composition and workers stay in
  their host. Moving those hooks can change ordering or introduce side effects in
  future headless composition. Inject keyring behavior in offline tests; real
  fallback fixtures need no user credential service or provider traffic.
