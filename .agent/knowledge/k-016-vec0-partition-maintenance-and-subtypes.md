---
id: "k-016"
title: "Maintain vec0 Partitions Transactionally and Preserve int8 Subtypes"
date: 2026-10-31
tags: ["sqlite-vec", "triggers", "migrations", "sqlcipher"]
severity: "high"
affected_components: ["crates/storage/src/migrations.rs", "crates/storage/src/repository.rs"]
symptoms:
  - 'Inserted vector for the "" column is expected to be of type , but a float32 vector was provided.'
  - "UPDATE on partition key columns are not supported yet."
---

## Problem / Symptom

sqlite-vec 0.1.9 cannot update partition keys in place. Account reassignment therefore needs
transactional delete/reinsert of the derived vector. An account-update trigger using
`INSERT ... SELECT vec_int8(v.embedding_int8)` failed with the misleading empty-column/type
error above. The int8 marker did not survive this trigger insertion shape, even though normal
embedding writes and late-message insertion had passed. Testing only ordinary writes misses it.

## Approved Solution

Use an ordinary-message BEFORE-account-update trigger to remove old partition membership and
an AFTER trigger, guarded by authoritative-vector existence, to insert the new membership.
Wrap conversion outside the scalar subquery in the insertion's VALUES expression:

```sql
INSERT INTO vec_messages_by_account(message_id, account_id, embedding_int8)
VALUES(new.id, new.account_id, vec_int8((
  SELECT embedding_int8 FROM vec_messages WHERE message_id = new.id
)));
```

Message insertion uses the same shape for vectors that arrived first. Message deletion removes
derived membership, including account cascades and UIDVALIDITY resets. The normal embedding
API replaces authoritative and derived vectors in one transaction; invalid dimensions roll
back both deletions/inserts. Migration v5 backfills only live vectors with the schema version
in the same transaction. A forced trigger-name conflict regression verifies rollback after
backfill, not merely failure before any index data was written.

## What NEVER to Do

- Do not update vec0 partition keys directly or register triggers on a virtual table.
- Do not treat raw int8 bytes as float32 or omit the outer vec_int8 conversion in trigger VALUES.
- Do not update the authoritative vector and derived partition index in separate transactions.
- Do not run older writers against a v5 database; they do not maintain its new derived index.
- Do not use vector shadow-table IDs as message IDs or silently admit orphan partition entries.
