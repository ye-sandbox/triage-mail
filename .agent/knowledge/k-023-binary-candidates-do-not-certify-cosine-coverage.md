---
id: "k-023"
title: "Binary Candidate Reranking Does Not Certify Exact Cosine Coverage"
date: "2026-11-02"
tags: ["sqlite-vec", "binary-quantization", "cosine", "correctness"]
severity: "high"
affected_components: ["crates/storage/examples/binary_quantization.rs"]
---

Binary sign codes discard magnitudes. A vector with worse Hamming distance can be the
unique best int8 cosine neighbor. Exact reranking a full candidate page does not recover
that neighbor or prove coverage. The example's sign-loss probe constructs this failure.

Same-sign vectors can have different cosine distances; more than 4,096 can tie in Hamming
space. sqlite-vec 0.1.9 caps KNN k at 4,096, preventing arbitrary widening. A cosine gap
inside the retained set is not a lower bound for the omitted set. Zero-norm inputs also
need k-021's NULL-first scoped ordering, independently of binary distance.

Apply mailbox eligibility before top-k. A static index built from live messages excludes
missing/orphan vectors only at build time; production adoption would also need atomic
lifecycle maintenance and append-only schema governance. Keep production exact ranking.
See `docs/performance/binary-quantization.md` and ADR-011 for measurements and decision.
