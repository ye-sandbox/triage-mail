---
id: "k-012"
title: "Keep Local Folder Intent Separate from Canonical IMAP Identity"
date: 2026-10-31
tags: ["storage", "ipc", "triage", "imap"]
severity: "high"
affected_components: ["crates/storage", "apps/desktop"]
---

## Invariant

`MessageRowSummary.folder` and `uid` describe the same canonical server identity.
A pending archive/trash action belongs in `folder_override`; substituting its display label
into `folder` makes a lazy attachment request target the wrong folder with the source UID.
Only acknowledged MOVE mapping updates canonical folder/UID, clearing the override atomically.

## Persistence and UI reconciliation

Read sorted tags and nullable overrides from existing encrypted tables. Hydrate ranked
candidates in batches of at most 500 IDs, after pagination; joining one-to-many tags into
ranked SQL can duplicate rows and change page boundaries. SQL parameters carry message IDs.

Optimistic UI writes update summary metadata as well as feedback. Accepted authoritative
pages replace feedback, so cleared overrides cannot linger after MOVE. Mutation rollback
must be generation-guarded together with row restoration; otherwise a rejected old write
can resurrect metadata after switching searches. Reopening encrypted storage and testing all
three search modes exercises persistence without live providers.
