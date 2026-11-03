---
id: "k-030"
title: "Sender Review History Frontiers and Visibility Boundaries"
date: 2026-11-02
tags: ["sender-review", "sync", "identity", "search"]
severity: "medium"
affected_components: ["crates/storage", "apps/desktop"]
---

# Sender Review History Frontiers and Visibility Boundaries

- History Known and user Accepted are different states. Observe with INSERT OR IGNORE;
  a late imported message must never erase Pending or a user decision. Store choices
  independently of message rows so MOVE/cache removal cannot reset familiarity.
- Capture UIDVALIDITY/UIDNEXT per folder before ingestion. Compare each message UID,
  never the batch checkpoint: a batch can straddle the initial history frontier.
  Retain that frontier across interrupted batches. Reset and new-folder history seed
  Known. Unknown UIDNEXT is conservative; replace its sentinel on the first usable
  frontier without treating already-imported history as newly reviewable.
- Enabling/re-enabling seeds cached history before activation. Background batches are
  local, bounded and independent of automation. Emit a refresh even when initialization
  completes with zero newly seeded rows; otherwise a just-enabled empty cache can
  remain visually stale. Settings polls initialization without overwriting dirty drafts.
- Only ordinary Inbox search composes INBOX_VISIBLE. Do not add it to shared mailbox
  eligibility: saved and category views intentionally keep access to declined mail.
  Review membership belongs before both ranking windows, never after loaded pagination.
- Normalization reuses the shared single-mailbox parser, ASCII lowercase and a 320-byte
  bound. No domain trust, authentication, alias collapse or cross-account equivalence.
  Unresolved identity remains visible and has no sender-wide write control.

Verify with `cargo test --offline -p storage sender_review` and desktop review tests.
Offline fixtures cannot establish provider interoperability, native frames or 60fps.
