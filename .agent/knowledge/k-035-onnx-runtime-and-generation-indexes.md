---
id: "k-035"
title: "Pinned ONNX Runtime and Generation-Safe Semantic Indexing"
date: 2026-11-05
tags: ["onnx", "embeddings", "sqlite", "backup", "svelte"]
severity: "high"
affected_components: ["crates/triage-ai", "crates/storage", "apps/desktop"]
---

# Pinned ONNX Runtime and Generation-Safe Semantic Indexing

- ort rc.10 targets ONNX Runtime 1.22. Disable its default/download/fetch features;
  load only the checksum-verified platform library via init_from, before any session.
  Commit with telemetry disabled. Dynamic loader failures can panic; isolate the
  initializer and report a fixed error. Missing files may be installed later; after
  dependent-library/load failure, repair and restart. Do not swap a loaded library.
- The application never fetches the native runtime. Its actual library filename and
  hash are platform-specific; a tar archive hash is not a library hash. Cache paths
  must contain regular files rather than symlinks. Model installation verifies both
  artifacts before publication. Unique create_new staging files make abandoned
  installs inert and retryable. The no-redirect app downloader rejects normal Hub
  resolve redirects; direct mirrors or explicit manual provisioning are necessary.
- ONNX outputs must match [1,tokens,384]. Pool with the tokenizer attention mask,
  normalize and quantize. Never substitute the old feature-hash provider after an
  inference failure. Preserve legacy exported/injected APIs separately.
- Schema v11 source triggers invalidate vectors on subject/plain-body changes.
  Native inference never holds the SQL lock. Commit only if generation, random rebuild
  epoch and input version still match. Restart must not regenerate the epoch. The
  per-work pending index avoids revisiting completed messages for every batch.
- Gate every semantic ranking snapshot, including saved/category/review/conversation
  paths. Managed global hybrid uses the existing sequential exact path in a read
  transaction. A check before inference alone cannot certify readiness at ranking time.
  Raw legacy vector writes are rejected once generation management is active.
- The schema-10 backup allowlist has no v11 additions. Keep its payload/version stable;
  derived vectors/work/generations are excluded. Fresh restore triggers create pending
  work, and native startup establishes a fresh managed epoch before workers run.
- Svelte treats a local variable named state as a store when parsing $state; use an
  unrelated name such as progress for a rune variable. Polling and action completions
  require teardown guards. This task uses only local state polling, never model fetching.

Reproduction, source licenses, exact hashes and limitations: docs/onnx.md; ADR-022.
