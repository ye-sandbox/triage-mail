---
id: "k-047"
title: "Windows Installation Writer Ownership and Path Contract"
date: 2026-11-08
tags: ["runtime", "locking", "windows", "ownership", "filesystem"]
severity: "high"
affected_components: ["crates/sync-runtime", "apps/desktop/src-tauri"]
---

# Windows Installation Writer Ownership and Path Contract

- Windows desktop clients require cooperative process writer ownership (`WriterOwnership`)
  prior to storage initialization and credential access. Without Windows-specific
  locking, `WriterOwnership::acquire` and `matches_installation` failed closed with
  `OwnershipError::Unavailable`, causing `bootstrap::open` to error with
  `StorageUnavailable` during `cmd_list_accounts`.
- The Windows contract mirrors Unix under `sync_runtime::ownership::windows` using 100%
  safe Rust (`#![forbid(unsafe_code)]`). Locking uses `std::fs::File::try_lock()` which
  calls Win32 non-blocking file locking (`LockFileEx`), cleanly translating `WouldBlock`
  to `OwnershipError::Busy`.
- The lock file is persistent and located at `.triage-mail-writer-<installation-name>.lock`
  in the canonical parent directory. Never unlink or delete it on release; kernel
  file locking automatically releases ownership on handle close or process termination.
- Path canonicalization and metadata validation on Windows:
  1. `validate_path`: rejects relative paths, directory traversals (`..`), and root-only paths.
  2. `installation_path`: canonicalizes the parent directory, ensuring that path aliases
     converge on identical sibling lock targets while keeping literal destination names.
  3. `valid`: validates that the lock file is an ordinary file (`meta.is_file()`),
     not a symlink (`!meta.file_type().is_symlink()`), has no reparse point attribute
     (`meta.file_attributes() & 0x400 == 0`), is empty (`meta.len() == 0`), and has exactly
     one link (`meta.number_of_links() == Some(1)`).
  4. Identity validation: compares open handle metadata against named path metadata
     using `volume_serial_number()` and `file_index()` to prevent symlink and junction
     swapping races.
