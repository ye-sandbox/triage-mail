---
id: "k-029"
title: "Local Category Correction Identity and Backfill"
date: 2026-11-02
tags: ["categories", "sqlite", "sync", "identity"]
severity: "medium"
affected_components: ["crates/storage", "crates/triage-ai", "apps/desktop"]
---

# Local Category Correction Identity and Backfill

## Invariants

- Automatic results, message corrections and sender overrides must remain separate.
  Sync upserts can replace automatic facts but never corrections. Null resets retain
  revision counters; deleting revision rows would admit stale save/reset ABA writes.
- Category SQL eligibility and final hydration share CATEGORY_EFFECTIVE. Never filter
  loaded pages or a global semantic candidate list by category after ranking.
- A provider ID or syntactically valid Message-ID plus received timestamp is the
  durable account-bound identity. Hash structured tuples, not concatenated strings.
  Normalize enclosing Message-ID brackets: MIME ingestion stores the unbracketed form,
  while older/synthetic records may retain brackets. Freeze first identity through canonical MOVE and hydration. A different identity on
  re-ingestion cannot inherit the old correction. Missing identity is record-only:
  the messages deletion trigger clears that correction before the metadata cascades.
- Sender normalization parses exactly one mailbox. Display names, domains, malformed
  addresses and cross-account senders cannot substitute for the exact normalized address.
  Address equality does not authenticate the author.
- History backfill is bounded and transactionally resumable. Fact-change triggers mark
  automatic versions stale; ingestion refreshes them before commit. Background backfill
  must not use ingestion APIs that enqueue rules/reminders or alter IMAP state. Legacy
  badges are temporary fallback until each history row receives current metadata.

## Verification

`cargo test --offline -p storage categories::tests` covers correction precedence,
reset conflicts, account isolation, backfill rollback/resume, upsert badge preservation,
MOVE/re-ingestion/reopen, record deletion, malformed identity, full-result pagination,
semantic/hybrid eligibility and saved-view compatibility. Provider interoperability
and native frame timing are outside these synthetic offline checks.
