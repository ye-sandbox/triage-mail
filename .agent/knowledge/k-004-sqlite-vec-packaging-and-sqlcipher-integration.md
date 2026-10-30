---
id: "k-004"
title: "sqlite-vec C Source Packaging and SQLCipher Integration Invariants"
date: 2026-10-30
tags:
  - "sqlite-vec"
  - "sqlcipher"
  - "rusqlite"
  - "ffi"
  - "knn"
severity: "medium"
affected_components:
  - "crates/storage/Cargo.toml"
  - "crates/storage/src/connection.rs"
  - "crates/storage/src/vec_ext.rs"
symptoms:
  - "fatal error: sqlite-vec-diskann.c: No such file or directory"
  - "failed to set SQLCipher key: Execute returned results - did you mean to call query?"
  - "Inserted vector for column is expected to be of type int8, but a float32 vector was provided"
---

# [k-004] sqlite-vec C Source Packaging and SQLCipher Integration Invariants

## Problem / Symptom

1. `sqlite-vec v0.1.10-alpha.4` published on crates.io includes `#include "sqlite-vec-diskann.c"` inside `sqlite-vec.c:3772`, but the tarball omits `sqlite-vec-diskann.c`, causing `cc` compilation to fail with `fatal error: sqlite-vec-diskann.c: No such file or directory`.
2. In `rusqlite`, executing `PRAGMA key = ...` via `conn.execute(...)` fails with `Execute returned results - did you mean to call query?`.
3. Inserting raw blobs into `vec0` tables expecting `int8[384]` causes SQLite to reject the insert with `expected to be of type int8, but a float32 vector was provided` because SQLite raw blobs default to float32 vectors unless wrapped in `vec_int8(...)`.

---

## Approved Solution

1. Pin `sqlite-vec = "0.1.9"` in `crates/storage/Cargo.toml`, which provides a stable, complete amalgamated C extension without unbundled diskann dependencies.
2. In `crates/storage/src/connection.rs`, use `conn.pragma_update(None, "key", &key_hex)` to configure SQLCipher 256-bit binary keys formatted with raw hex syntax `x'<hex>'`.
3. In `crates/storage/src/vec_ext.rs`, isolate unsafe FFI into a single function registering `sqlite3_vec_init` with explicit transmute type annotations:
   ```rust
   let init_fn: unsafe extern "C" fn(
       *mut rusqlite::ffi::sqlite3,
       *mut *mut std::os::raw::c_char,
       *const rusqlite::ffi::sqlite3_api_routines,
   ) -> std::os::raw::c_int =
       std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
   let rc = rusqlite::ffi::sqlite3_auto_extension(Some(init_fn));
   ```
4. In SQL queries for `int8` vector tables, wrap vector parameters in `vec_int8(?2)`. Both raw byte slices (`&[u8]`) and JSON arrays are accepted without float32 conversion errors.

---

## What NEVER to Do

- ❌ **Anti-Pattern:** Attempting to patch missing C sources in external crates or using unverified alpha releases when a stable release (`0.1.9`) is available.
- ❌ **Anti-Pattern:** Spreading `#[allow(unsafe_code)]` across the storage crate; keep FFI registration strictly isolated to `vec_ext.rs` under `#![deny(unsafe_code)]`.
- ❌ **Anti-Pattern:** Passing unencrypted SQLite pragmas before setting the SQLCipher encryption key.
