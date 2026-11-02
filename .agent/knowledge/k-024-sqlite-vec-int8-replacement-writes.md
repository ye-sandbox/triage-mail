---
id: "k-024"
title: "Use Delete/Insert for sqlite-vec Int8 Replacement Fixtures"
date: "2026-11-02"
tags: ["sqlite-vec", "int8", "fixtures"]
severity: "medium"
affected_components: ["crates/storage/examples/binary_quantization.rs"]
---

The [01.1] adversarial fixture initially used:

```sql
UPDATE vec_messages SET embedding_int8=vec_int8(?2) WHERE message_id=?1;
```

With sqlite-vec 0.1.9's account-partitioned int8 table this failed with:
`Updated vector for the "embedding_int8" column is expected to be of type int8, but a float32 vector was provided.`

Use the production replacement pattern: delete the old vec0 row and insert all explicit
columns with `vec_int8(...)`. The three adversarial probes then pass. The likely cause is
subtype handling in vec0 UPDATE; an upstream root-cause investigation was not performed.
Do not widen accepted vector types or reinterpret the bytes as floats to hide the failure.
Production replacements must remain atomic with the surrounding message/index writes;
the example modifies only its disposable fixture before evaluating candidates.
