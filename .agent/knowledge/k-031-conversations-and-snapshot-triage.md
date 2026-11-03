---
id: "k-031"
title: "Conversation Rebuilds, Ranked Groups and Atomic Snapshot Actions"
date: 2026-11-03
tags: ["storage", "threading", "ipc", "outbox"]
severity: "medium"
affected_components: ["crates/storage", "crates/mail-engine", "apps/desktop"]
---

# Conversation Rebuilds and Snapshot Actions

Conversation membership uses canonical records, not unique Message-ID assumptions.
Normalize header IDs case-sensitively; malformed/oversized values fail to singleton
navigation. Missing ancestors can link replies, but duplicate IDs cannot resolve
links. Never infer threads from normalized subjects or sender addresses. Union/find
is iterative; the legacy public tree helper additionally caps nesting at 64 to keep
serialization/destruction safe. Group IDs can change after late parents or splits.

Migration v9 stages <=300 header inputs per tick. Dirty-account consolidation reads
all compact staged inputs and replaces that account's membership in one transaction;
this final step is linear account-wide work and can block other database access.
Deletion and changed headers require a rebuild, even when all remaining inputs already
exist. Do not terminate backfill merely because the last staging call returns zero;
check progress.ready. Global reads require all accounts ready; scoped reads check only
relevant accounts. No history replay, remote fetch or automation belongs in this path.

Conversation ranking filters before group pagination. Complete native branch rankings
produce best-message group order, with complete-branch RRF for hybrid conversation
search. Hydrate only final representative/member pages. Do not collapse frontend pages
or increase public HybridQuery/IPC limits to reach unseen matching records.

Generations combine a random database epoch, UTC day and latest expired snooze boundary.
Message/folder/filter changes and vector writes invalidate the epoch. Previews expire
after five minutes, store immutable selected IDs/action, and are consumed inside the
local mutation transaction. Generation-only checks outside that transaction allow
sync races. Keep unread-reminder cancellation and durable remote queue inserts atomic.
Selections >500 must fail, never chunk or truncate them silently. Failed remote changes
remain queued, with fixed failure/attempt metadata only; partial remote success is normal.

Reproduce offline with `cargo test --offline -p storage conversations` and
`cargo run --offline --release -p storage --example conversation_performance`.
