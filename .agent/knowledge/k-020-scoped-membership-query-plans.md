---
id: "k-020"
title: "Inspect Correlated Membership Plans at Mailbox Scale"
date: "2026-11-02"
tags: ["sqlite", "sqlcipher", "mailboxes", "query-planner"]
severity: "high"
affected_components: ["crates/storage/src/scoped_search.rs"]
symptoms:
  - "Blank scoped Inbox search fails to return a page on a 100k-message synthetic store"
  - "Native fixture reaches idle but cannot finish the first scoped batch"
---

## Observed problem

The unmodified scoped Inbox predicate correlates membership by message ID and account.
On the disposable migrated v2 database with 100k messages and 200k memberships, EXPLAIN
selects `idx_memberships_account_folder (account_id=?)` for the correlated subquery,
rather than `idx_memberships_message_id`. Account-2 has 25k messages and 50k memberships.
The outer query scans the account and sorts through a temporary B-tree. This plan permits
roughly 25k x 50k membership visits before a 200-row result can finish.

The native 90-second fixture timed out without completing its first Inbox batch. See
`docs/performance/mailbox-validation.md` for the separate bounded storage probe and raw plan.
These are failed acceptance observations, not accepted latency samples or a correctness failure.

## Follow-up boundary

A production fix needs separate approval under [04.1.7]. Investigate a message-correlated
lookup or equivalent eligible-ID formulation with existing indices, preserving effective
pending moves, account isolation, one read snapshot, deduplication and native ranking.
Measure all scoped branches after correcting the plan. This card does not prescribe a fix.

Never add ANALYZE, optimizer hints, extra indices or artificial statistics only to the fixture
to obtain favorable results. Benchmark the actual migration and production entry point.


## Approved fix and measured follow-up (2026-11-02)

User `proceed` approved the scoped production fix. Inbox/folder predicates now explicitly
use the existing message-ID membership index. Snoozed enumeration uses the existing
snooze-deadline or account index, chosen by bounded index-only counts in the same snapshot.
The account probe caps at 16,384 rows; the global active-deadline probe caps at that account
count. This prevents tiny-account scans from traversing another account's entire active range.
A skewed-population VM-step regression preserves that guard. Query-plan regressions cover blank/lexical/semantic branches; a
5k-message/10k-membership VM-step budget detects renewed quadratic account scans without
a hardware-specific wall-clock assertion. The native fixture now completes all phases.

Final Snoozed semantic p95 is 16.702ms versus the 15ms budget; Inbox semantic is
106.144ms. The index fix removes the stall but does not satisfy feature closure. See the current follow-up section of the
validation report. KNN candidate shortcuts must also preserve zero-norm NULL ordering;
see k-021 before attempting another semantic optimization.
