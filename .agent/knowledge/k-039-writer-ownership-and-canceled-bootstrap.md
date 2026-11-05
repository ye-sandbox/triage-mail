---
id: "k-039"
title: "Writer Ownership and Cancellation-Safe Bootstrap Gates"
date: 2026-11-05
tags: ["runtime", "locking", "vault", "recovery", "cancellation"]
severity: "high"
affected_components: ["crates/sync-runtime", "apps/desktop/src-tauri"]
---

# Writer Ownership and Cancellation-Safe Bootstrap Gates

- SQLite transaction locking cannot prevent duplicate workers or competing vault
  provisioning. Acquire the shared WriterOwnership guard before either operation.
  An empty persistent sibling lock supports absent-target restore. Never unlink it:
  deleting a held lock permits another process to own a different inode at that name.
  A leftover file after a crash is normal; kernel ownership, not existence, matters.
- Final-component installation symlinks are refused. Canonical parent aliases share
  identity before and after directory publication. Keep lock contents empty, mode
  0600 and one hard link. Parent write access is required. This is advisory local
  coordination, not a hostile-filesystem or older-binary security boundary.
- A canceled spawn_blocking await does not cancel the operation. Keep process
  ownership in both the task and its produced service; retain it separately in the
  cloned vault used by detached NativeRemoteMail credential reads. Worker/storage
  paths retain Arc<DesktopService>. Put ownership after resources in field drop order.
- The recovery mutex must also survive canceled callers. Transfer owned gate handles
  into bootstrap/restore blocking tasks, otherwise a same-process retry can overlap
  the original operation despite both sharing one process-ownership guard. Preserve
  recovery-gate-before-process-lock ordering. Failed unlock retains runtime ownership
  until that RuntimeState and any outstanding work have been dropped.
- Offline CLI readers do not acquire the guard. They still require valid SQLCipher
  snapshots and readable initialized WAL/SHM sidecars; locking does not remove those
  requirements. Never repair sidecars by changing the CLI/container write contract.
- Build-only gotcha: the shared profile may contain a pkg-config shim that reports
  every package as present, including nonexistent libonnxruntime. For dynamic-loader
  builds set session-only LIBONNXRUNTIME_NO_PKG_CONFIG=1 so ort-sys does not advertise
  a bogus linker library. This does not provide GTK/WebKit libraries or validate the
  shim's other claims. Do not edit shared wrappers or commit machine-specific paths.
