---
id: "k-041"
title: "Ingestion Frontiers and Canonical Results"
date: 2026-11-05
tags: ["sync", "storage", "triage", "transactions"]
severity: "high"
affected_components: ["crates/sync-runtime", "apps/desktop/src-tauri"]
---

# Ingestion Frontiers and Canonical Results

- Shared ingestion establishes the rule baseline and then sender-review frontier
  before conversion. Those transactions intentionally precede the message batch;
  conversion/checkpoint failure does not roll them back. Retrying historical mail
  must not advance the initial frontier and enqueue history as automatic arrivals.
- Message conversion creates proposed JSON tuple IDs, but storage can deduplicate
  a folder membership onto an existing account-local canonical record. Always pass
  the returned records to hooks. Reconstructing IDs from the incoming UID can target
  nonexistent rows or duplicate effects. Flag deltas resolve folder memberships.
- Messages, flags, checkpoint and reset deletions share one transaction. Injecting
  failure at the final checkpoint write verifies that earlier changes roll back.
  Arrival-frontier changes can still persist; do not claim whole-operation atomicity.
- The injected clock preserves two distinct evaluations when Date is absent and
  only one when supplied. Ingesting an empty batch never calls it. Attachment
  conversion retains metadata only and refuses sizes outside signed storage range.
- Hosts retain their blocking executor, mutation gate, ownership lifetime and
  post-commit effects. The shared function takes a borrowed store, acquires no lock
  or credentials and does not start workers/inference/network. A canceled await can
  still leave blocking ingestion and hooks running under the retained service.
- The shared coordinator continues to reject zero remote UIDVALIDITY before host
  commit. Direct ingestion callers must preserve that upstream validation contract;
  this extraction does not introduce a new transport validation policy.
