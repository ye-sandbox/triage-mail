---
id: "k-018"
title: "Pre-Release Schema Consolidation, Single Partitioned vec0, and Triage Persistence"
date: 2026-10-31
tags: ["sqlite-vec", "schema-consolidation", "sqlcipher", "triage", "adr-007"]
severity: "medium"
affected_components: ["crates/storage/src/migrations.rs", "crates/storage/src/repository.rs", "crates/storage/src/search.rs", "apps/desktop/src-tauri/src/worker.rs", "apps/desktop/src/lib/inbox/MessageList.svelte"]
symptoms:
  - "Unreleased schema churn resulting in redundant dual-table synchronization triggers and backfills."
  - "Redundant regex/heuristic triage extraction on message list scroll and detail views."
  - "Full email body IPC serialization solely to extract UI list badges."
---

## Problem / Background

In pre-release development leading up to v0.1.0, five separate schema migrations evolved incrementally (v1 base, v2 FTS, v3 triggers, v4 account vectors, v5 backfill triggers). ADR-006 introduced a derived `vec_messages_by_account` partition table alongside `vec_messages` solely to preserve backward compatibility with an unreleased v4 schema. This added four synchronization triggers (`messages_vector_insert`, `messages_vector_delete`, `messages_vector_account`, `messages_vector_account_insert`), duplicate vector insertions, and backfill code.

Simultaneously, triage analysis (`DefaultDeterministicTriage::analyze`) was recomputed on every message detail request (`cmd_get_message_body`), and list view badges required secondary IPC calls (`cmd_get_message_body`) fetching full decrypted message bodies just to display category and badge pills.

## Approved Solution (ADR-007)

1. **Pre-Release Consolidation to Canonical v1:**
   Before reaching the immutable `v0.1.0` release tag, migrations v1–v5 were collapsed into a single canonical `CURRENT_SCHEMA_VERSION = 1`.
   - Dropped `vec_messages_by_account` and all reconciliation triggers.
   - Defined `vec_messages` directly with partition key:
     `vec_messages(message_id TEXT PRIMARY KEY, account_id TEXT PARTITION KEY, embedding_int8 int8[384] distance_metric=cosine)`.
   - Verified that sqlite-vec handles both partition-filtered queries (`WHERE embedding_int8 MATCH vec_int8(?1) AND k=?2 AND account_id=?3`) and unfiltered global queries (`WHERE embedding_int8 MATCH vec_int8(?1) AND k=?2`) against the exact dot-product oracle at 100k messages.

2. **Triage Persistence at Ingestion:**
   Added `triage_badges TEXT NOT NULL DEFAULT '[]'` column to `messages`.
   - Worker computes deterministic triage once during ingestion, storing `category_badge` and serialized JSON `triage_badges`.
   - Hydrated additively into `MessageRowSummary` / `MessageRowDto`.
   - `MessageList.svelte` renders `<TriageBadges badges={row.triage_badges} />` directly from row summary, eliminating secondary IPC calls and full email body transfers.
   - `get_message_body` reuses stored `triage_badges` when present instead of re-executing regex/heuristics.

3. **P1 Schema Immutability Clarification:**
   Rule P1 applies to *released* schemas to protect production users from breaking changes. In pre-release phases prior to tag `v0.1.0`, canonical schema consolidation is preferred over accumulating legacy baggage. From tag `v0.1.0` onwards, all migrations are strictly append-only.

## What NEVER to Do

- Do not maintain derived duplicate vector tables and trigger overhead when a single partitioned table satisfies all query access patterns.
- Do not re-run regex or heuristic triage extraction repeatedly for read-only viewing when it can be deterministically captured at ingestion.
- Do not transfer full email bodies across the Tauri IPC boundary purely to determine badge pills for virtualized lists.
- Do not create append-only migrations during pre-tag bootstrap phases when no production databases exist in the wild.
