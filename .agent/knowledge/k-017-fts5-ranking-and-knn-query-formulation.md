---
id: "k-017"
title: "Streamline FTS5 Ranking CTEs and Dedicated KNN Prepared Statements"
date: 2026-10-31
tags: ["sqlite-vec", "fts5", "performance", "query-planning", "sqlcipher"]
severity: "medium"
affected_components: ["crates/storage/src/search.rs"]
symptoms:
  - "Unnecessary MATERIALIZED CTE writes intermediate B-tree pages during FTS ranking"
  - "Dead parameter binding ?2 IS NULL forces redundant runtime condition evaluation"
---

## Problem / Symptom

1. Using `WITH ranked AS MATERIALIZED (...)` in SQLite FTS5 queries forces SQLite to allocate
   and populate an ephemeral in-memory B-tree for the ranked candidate subset, adding ~2-3ms
   of materialization overhead per query compared to a non-materialized CTE that flattens into
   the primary outer join.
2. Sharing a single query string between account-filtered and global search queries by appending
   `AND ?2 IS NULL` or `(?2 IS NULL OR account_id=?2)` binds dead parameters and forces SQLite's
   query planner to evaluate runtime nullity checks or prevents optimal index path selection.

## Approved Solution

1. Drop `MATERIALIZED` from the ranking CTE (`WITH ranked AS (...)`), allowing SQLite's query
   flattener to combine the top-k FTS candidate selection directly with the primary key rowid join
   while preserving the exact BM25 ordering and deterministic rowid tie-breakers.
2. Use dedicated query branches with exact parameter bindings for account-filtered vs global
   queries:
   - Global lexical: `WITH ranked AS (SELECT rowid AS rid, rank AS score FROM messages_fts WHERE messages_fts MATCH ?1 ORDER BY rank, rowid LIMIT ?2 OFFSET ?3) SELECT m.id FROM ranked r JOIN messages m ON m.rowid=r.rid ORDER BY r.score, r.rid`
   - Filtered lexical: `WITH ranked AS (SELECT rowid AS rid, rank AS score FROM messages_fts WHERE messages_fts MATCH ?1 AND +rowid IN (SELECT rowid FROM messages WHERE account_id=?2) ORDER BY rank, rowid LIMIT ?3 OFFSET ?4) SELECT m.id FROM ranked r JOIN messages m ON m.rowid=r.rid ORDER BY r.score, r.rid`
   - Global vector: `SELECT message_id, distance FROM vec_messages WHERE embedding_int8 MATCH vec_int8(?1) AND k=?2`
   - Partitioned vector: `SELECT message_id, distance FROM vec_messages_by_account WHERE embedding_int8 MATCH vec_int8(?1) AND k=?2 AND account_id=?3`
3. Pre-allocate RRF fusion maps (`HashMap::with_capacity(limit * 2)`) and use `prepare_cached`
   for summary metadata rehydration.

## What NEVER to Do

- Do not use `ORDER BY rank` without `rowid` for deterministic tie breaking; FTS5 rank ties
  require an explicit secondary criterion to ensure repeatable fixed-request pages.
- Do not bind unused nullable parameters to avoid maintaining separate prepared query strings.
- Do not sacrifice exact candidate verification (orphan checks, exact cosine distances, full
  result page counts) to pass latency thresholds.
