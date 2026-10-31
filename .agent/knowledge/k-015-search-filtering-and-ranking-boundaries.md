---
id: "k-015"
title: "Keep Search Eligibility and Tie Boundaries Inside Ranking"
date: 2026-10-31
tags: ["sqlite-vec", "fts5", "pagination", "sqlcipher"]
severity: "high"
affected_components: ["crates/storage/src/search.rs", "crates/storage/examples/search_performance.rs"]
symptoms:
  - "Account-filtered semantic search returns 41/200 rows despite 25000 eligible vectors"
  - "k value in knn query too large"
  - "Filtered lexical search regresses when adding a rank tie-breaker"
---

## Problem / Symptom

Filtering joined messages after vec0's global top-k loses eligible account neighbors.
Increasing k heuristically cannot guarantee a complete page. sqlite-vec 0.1.9 also caps k
at 4096; sorting the returned subset by ID does not resolve ties that cross its boundary.

Adding `rowid` to FTS5's `ORDER BY rank` disables its sorted-match plan. Joining every match
to wide message rows then incurs many SQLCipher payload reads. A plain `rowid IN` constraint
can instead cause repeated FTS MATCH point scans, which is substantially worse.

## Approved Solution

- For account-filtered semantic queries, enumerate eligible messages using the existing
  account index, CROSS JOIN point lookups into vec0, then order by exact cosine distance
  and message ID before pagination. Messages without embeddings do not join. CROSS JOIN
  fixes loop nesting; this preserves correctness but remains expensive at 25k vectors.
- For ordinary unfiltered queries, request top-(offset+limit+1). Expand when the boundary
  distance ties; use exact scalar ranking when k reaches 4096 or for deeper pages. Check
  candidate identities so orphan vectors cannot consume page slots. Do not access vec0
  shadow tables or assume their rowids equal message rowids.
- For filtered lexical search, use `+rowid IN (SELECT rowid FROM messages WHERE account_id=?)`.
  Unary plus prevents FTS5 constraint pushdown while retaining integer membership checks.
  The account subquery uses a covering index, so filtering reads no message payload pages.
  Rank narrow candidates by BM25/rowid and hydrate only the final page, after hybrid fusion.
- Verify query plans and exact neighbors independently of elapsed time. The 100k benchmark
  checks full result counts and semantic pages against integer-dot-product reference ranks.

## What NEVER to Do

- Do not report a faster empty or underfilled page as an optimization.
- Do not resolve a boundary tie only after truncating candidates, or silently cap deep offsets.
- Do not replace `+rowid IN` with a plain FTS5 rowid constraint without profiling its plan.
- Do not claim the schema-preserving account path meets the <15ms target; see recorded samples.
