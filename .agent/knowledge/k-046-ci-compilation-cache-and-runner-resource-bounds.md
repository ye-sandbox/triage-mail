---
id: "k-046"
title: "GitHub Actions Runner Resource Limits, Cargo Cache and Linker SIGBUS"
date: 2026-11-07
tags:
  - "ci"
  - "github-actions"
  - "cargo"
  - "performance"
  - "linker"
severity: "high"
affected_components:
  - ".github/workflows/ci.yml"
  - "Cargo.toml"
symptoms:
  - "collect2: fatal error: ld terminated with signal 7 [Bus error], core dumped"
  - "error: could not compile storage (example binary_quantization) due to 1 previous error"
  - "CI compilation taking 16+ minutes on every commit"
---

# [k-046] GitHub Actions Runner Resource Limits, Cargo Cache and Linker SIGBUS

## Problem / Symptom

On GitHub Actions `ubuntu-24.04` runners (2 vCPUs, 7 GB RAM, ~14 GB available root filesystem), Rust workspace CI runs can take 16-17 minutes or crash during linking with signal 7 (`SIGBUS`):

```text
collect2: fatal error: ld terminated with signal 7 [Bus error], core dumped
compilation terminated.
error: could not compile `storage` (example "binary_quantization") due to 1 previous error
Process completed with exit code 101.
```

---

## Approved Solution

### 1. Disable Incremental Compilation on Ephemeral Runners
In `.github/workflows/ci.yml`:
```yaml
env:
  CARGO_TERM_COLOR: always
  CARGO_BUILD_JOBS: '2'
  CARGO_INCREMENTAL: '0'
```

### 2. Enable Pinned Intelligent Cargo Caching
Use `Swatinem/rust-cache` pinned by commit SHA immediately after installing the toolchain:
```yaml
- name: Rust toolchain
  run: |
    rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
    rustup default 1.99.0
- name: Rust cache
  uses: Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3
- name: Fetch locked dependencies
  run: cargo fetch --locked
```

### 3. Keep Debug Symbols Bounded in Workspace Profiles
In `Cargo.toml`:
```toml
[profile.dev]
debug = 1

[profile.test]
debug = 1
```

**Root Cause:**
1. Linux `Signal 7 (SIGBUS)` during GNU `collect2`/`ld` occurs when `ld` writes via `mmap` to an output binary on a filesystem that has run out of space.
2. In ephemeral CI runners, `CARGO_INCREMENTAL: 1` (default in dev/test) writes gigabytes of unneeded object deltas into `target/debug/incremental/`, exhausting runner disk space when linking multiple large test and example binaries statically linked with OpenSSL vendored, SQLCipher, and `aws-lc-sys`.
3. Without dependency caching, every run must recompile all vendored C libraries and external crates from scratch, wasting 12-14 minutes of runner compute time.

---

## What NEVER to Do

- ❌ **Anti-Pattern:** Assuming `SIGBUS` in `ld` is an LLVM/compiler bug and speculating on compiler downgrades.
  - *Why:* It is an out-of-disk-space error caused by intermediate DWARF/incremental bloat.
- ❌ **Anti-Pattern:** Leaving `CARGO_INCREMENTAL` enabled on ephemeral CI runners.
  - *Why:* Intermediate incremental files are never reused between fresh VM instances and only bloat disk and memory.
- ❌ **Anti-Pattern:** Running unpinned GitHub Actions (e.g. `@main` or unverified floating tags).
  - *Why:* Violates organizational P0 security supply chain invariants.

---

## Verification

```bash
# Verify offline workspace validation with zero warnings and clean exit
cargo check --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

---

## References
- `.github/workflows/ci.yml`
- Commit `2361ade` (`perf(ci): enable cargo caching and disable incremental compilation`)
- `k-042-low-io-compilation-and-vitest-concurrency.md`
