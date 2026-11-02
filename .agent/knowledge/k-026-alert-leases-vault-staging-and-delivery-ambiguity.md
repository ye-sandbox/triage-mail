---
id: "k-026"
title: "Alert leases, vault staging and ambiguous delivery"
date: 2026-11-02
tags: ["alerts", "vault", "transactions", "retry"]
severity: "high"
affected_components: ["crates/storage", "crates/vault", "apps/desktop/src-tauri"]
symptoms: ["Duplicate receiver events after timeout", "Orphaned secret after interrupted destination save"]
---

# Alert leases, vault staging and ambiguous delivery

A receiver may commit a POST before the client times out or crashes. Retrying with the
same event ID/Idempotency-Key permits receiver deduplication but cannot guarantee it.
A local notification service acknowledgment likewise does not prove human visibility.
Never describe these adapters as exactly once. Leases must increment durable attempts
before I/O; stale completions must compare attempts, revision and pending status.

Vault writes and SQL commits cannot share a transaction. Register an opaque immutable
secret revision in SQL before writing vault bytes; commit the destination reference only
after the write succeeds. Cleanup enumerates unreferenced intent IDs, deletes vault data,
then removes the intent. A failed delete leaves the intent for retry. Never delete a
referenced secret because a later SQL operation failed. Never persist URL paths, queries
or authentication in SQL metadata/history to simplify crash recovery.

Freeze enabled destination revisions at ingestion, not only at delivery enqueue. Otherwise
an observer backlog can export old mail after the user changes fields or enables a destination.
Explicit replay/test confirmations also carry the reviewed revision. Disabling serializes
with dispatch; an already-started bounded request can complete before disable returns.

Compact automatic delivery markers survive execution history pruning and message-cache
deletion. Account/destination removal clears them. Message removal cancels related pending
jobs, including cache removal after a MOVE without COPYUID; this favors deletion/privacy
over guaranteed alert delivery. Existing conservative identity suppression from k-025 remains.
