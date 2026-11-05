---
id: "k-038"
title: "Preserve host hook ordering when sharing sync orchestration"
date: 2026-11-05
tags: ["sync", "tokio", "storage", "cancellation"]
severity: "high"
affected_components:
  - "crates/sync-runtime"
  - "apps/desktop/src-tauri/src/worker/sync_host.rs"
symptoms:
  - "Success progress appears before committed mail or post-commit hooks"
  - "Blocking ingestion continues after the account pass is canceled"
---

# Preserve host hook ordering when sharing sync orchestration

## Problem / Symptom

The desktop sync pass contains more than a protocol loop. Baselines and sender-review
frontiers must exist before message ingestion. Message/flag/checkpoint persistence
must finish before triage/rule hooks, reminder scheduling, injected indexing and final
success progress. Rule hooks can publish their own ResyncRequired invalidation before
indexing; treating every pre-index event as an ingestion success is incorrect.

Dropping an async pass cancels an outstanding remote future, but cannot cancel an
already running spawn_blocking closure. Earlier committed batches also remain durable.
These are existing desktop semantics, not new rollback or daemon ownership guarantees.

## Approved Solution

The coordinator awaits one host commit-with-hooks operation before the next checkpoint
read. DesktopSyncHost keeps the existing mutation gate and blocking executor around
that operation. Keep baseline/frontier creation and all hook failure classifications
in the adapter. Buffer final batch progress until ingestion and hooks finish; retain
independent rule invalidations after their own durable mutations.

Paused-clock tests verify timeout, pending-fetch cancellation and resumption after an
earlier committed full batch. A file-backed SQLCipher adapter probe checks rule/review
frontiers, rule effects before indexing, independent rule invalidation, final success
ordering, duplicate identity and durable reopen. These tests do not promise cancellation
of blocking work or atomic rollback of separately established frontiers.

## What NEVER to Do

- Do not publish remote fetch progress directly as committed success: ingestion may fail.
- Do not remove rule invalidation events to simplify event-order assertions: they update
  existing desktop views after durable rule effects.
- Do not infer cross-process serialization from the in-process mutation gate, or assume
  dropping a future rolls back storage. An active runner needs a separate ownership plan.
- Do not add a whole-pass timeout or change fallback/folder behavior during extraction;
  those change runtime policy and need separate scope.

## Verification

The coordinator integration suite exercises scripted offline hosts. The ipc_service
suite includes shared_sync_adapter_preserves_encrypted_frontiers_hooks_and_duplicate_identity
and existing reset, batch interruption, rules and commit-before-progress regressions.
