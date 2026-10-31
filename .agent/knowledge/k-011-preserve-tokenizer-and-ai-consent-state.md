---
id: "k-011"
title: "Preserve Tokenizer and AI Consent State"
date: 2026-10-31
tags: ["fts5", "vault", "consent", "embeddings"]
severity: "medium"
affected_components: ["crates/storage", "apps/desktop/src-tauri/src/settings.rs", "crates/triage-ai/src/embeddings.rs"]
symptoms:
  - "Tokenizer selection differs after reopening the database"
  - "An AI endpoint changes after the confirmation is displayed"
---

## Problem / Symptom

The storage constructor accepts a tokenizer but migrations do not alter an existing FTS
table. Remembering the constructor argument can report the wrong active tokenizer after
reopening. AI confirmation has a similar stale-state hazard if another window changes
settings before native dispatch. Writing endpoint metadata and keys separately can also
associate the wrong key after a process crash.

## Approved Solution

Read the persisted tokenizer from `sqlite_master` after migrations. `set_tokenizer` drops,
recreates and rebuilds the external-content FTS table in one transaction under the shared
connection lock; existing message triggers continue to target the same table name. Update
the in-memory mode only after commit. The SQL schema version remains unchanged.

Persist AI metadata and its endpoint-bound key as one vault value. The native summary
command requires enabled settings, per-message consent and equality with the settings
snapshot shown during confirmation. An endpoint/provider change cannot reuse an old key.
Disable inherited proxies and redirects on provider clients.

The current `LocalCpuEmbeddingProvider` computes deterministic projections seeded by cached
model bytes; it does not execute ONNX inference. The settings UI must disclose this limitation.
Artifact caching is explicit, bounded, and accepts trusted direct HTTPS URLs without query
strings, credentials or redirects. Many model-hosting share URLs redirect and will be rejected;
use direct artifact hosting. No default fetcher is installed on the provider.

## What NEVER to Do

- Do not infer the persisted tokenizer from the default constructor argument.
- Do not rebuild an external-content index outside a transaction or remove message triggers.
- Do not reuse bearer keys at a different endpoint, or treat saving settings as network consent.
- Do not describe cached artifacts as learned semantic inference or enable automatic downloads.

## Verification

Storage tests reopen the database, match substrings and exercise update triggers after rebuild.
Desktop tests cover redacted responses, vault rehydration, endpoint key isolation, stale
confirmation rejection, loopback summary requests and bounded artifact transport.
