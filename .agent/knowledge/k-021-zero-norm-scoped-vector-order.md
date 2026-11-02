---
id: "k-021"
title: "Preserve Scoped NULL Distance Ordering for Zero-Norm Vectors"
date: "2026-11-02"
tags: ["sqlite-vec", "cosine", "correctness", "scoped-search"]
severity: "high"
affected_components: ["crates/storage/src/scoped_search.rs", "crates/storage/src/scoped_search_tests.rs"]
symptoms:
  - "A finite-distance candidate frontier omits an eligible zero-norm stored vector"
---

## Existing behavior

Storage currently accepts zero-norm int8 vectors. Scalar `vec_distance_cosine` produces
SQL NULL for a zero-norm stored vector or query vector. Existing scoped ascending order
places those NULL distances first, then resolves ties by canonical message ID.

An account-partitioned KNN candidate frontier can omit a zero-norm stored vector even
when the last returned eligible finite distance is strictly below its frontier distance.
The missing NULL-sorted vector invalidates that apparent completeness proof. A targeted
regression reproduced this: a zero vector for `a-098` must precede finite `a-003`, whereas
the experimental candidate path returned `a-003`. The candidate optimization was removed;
the regression also protects deterministic ordering for an all-zero query vector.

## Boundary

Keep scoped exact scalar ranking until an approved design can efficiently account for
all zero-norm eligible vectors as well as missing/orphan vectors and boundary ties. Do not
assume normalization merely because usual embedding providers emit nonzero vectors, change
the accepted embedding domain, or silently alter existing NULL-distance ordering to pass
a latency target. No dependency, schema or public contract changed in this follow-up.
