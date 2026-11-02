---
id: "k-025"
title: "Rule history frontiers and move feedback prevention"
date: 2026-11-02
tags: ["rules", "imap", "transactions", "restart"]
severity: "high"
affected_components: ["crates/storage", "crates/mail-engine", "apps/desktop/src-tauri"]
symptoms: ["History replay after interrupted sync", "Repeated rules after archive moves"]
---

# Rule history frontiers and move feedback prevention

Capture the first SELECT UIDNEXT snapshot for each account/folder/UIDVALIDITY before
committing its first batch. Persist this frontier across every resumed batch. A fetched
batch's last UID is not the initial history frontier. Missing UIDNEXT must fail closed;
a later SELECT must not move an already established frontier. Existing installations
establish a new baseline on their first post-upgrade sync. Newly discovered folders and
UIDVALIDITY resets suppress their own initial history even if other folders are ready.

Only canonical inserts above the baseline enqueue rule jobs, inside the ingestion/checkpoint
transaction. Snapshot scheduled rule revisions. Saving/enabling a rule cannot retrospectively
create eligibility; editing/reordering invalidates its pending revision. Simulation uses
the same evaluator without jobs, markers, mutations or remote calls.

A processed marker attached only to message_id with ON DELETE CASCADE is insufficient:
MOVE without COPYUID deletes the source cache and later destination sync can create a
new canonical ID. Keep account-bound hashed identity tombstones until account removal.
Prefer provider ID, then Message-ID/timestamp, then original headers/subject/body. The
last fallback conservatively suppresses identical headerless copies; it cannot prove
that two indistinguishable IMAP messages are distinct. Never infer cross-account identity.

Keep processed marker, local mutations and IMAP outbox writes atomic. Flags precede
archive in the action plan to avoid targeting a source UID after MOVE. Preserve the
existing canonical server identity and snooze-clearing primitive. Pending jobs retry on
restart through the existing worker, without replaying already committed jobs.

Regression coverage lives in storage rules tests and desktop ipc_service tests; encrypted
reopen, rollback, history resumes, resets and move-cache deletion are covered offline.
