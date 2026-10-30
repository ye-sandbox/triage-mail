---
id: "k-006"
title: "Tauri Linux Build Prerequisites and IPC Identity Boundaries"
date: 2026-10-30
tags: ["tauri", "linux", "ipc", "imap"]
severity: "medium"
affected_components: ["apps/desktop/src-tauri", "crates/storage", "crates/mail-engine"]
symptoms:
  - "failed to open icon .../icons/icon.png"
  - "rust-lld: error: unable to find library -lgdk-3"
  - "Operation not permitted when binding loopback test servers"
---

# Tauri Linux Build Prerequisites and IPC Identity Boundaries

## Build prerequisites

- Native Tauri requires `pkg-config`, GTK 3 and WebKitGTK 4.1 development files on Linux.
  Cached Rust crates alone are insufficient. The context macro also requires the committed
  `icons/icon.png`, even for checks/tests without a displayed window.
- A user-owned development sysroot can support sandboxed checks. Set `PKG_CONFIG_SYSROOT_DIR`,
  `PKG_CONFIG_PATH`, and `LD_LIBRARY_PATH` only for the build session. Extracted development
  `.so` symlinks may point to runtime libraries installed on the host rather than inside the
  sysroot; resolve those links before retrying the linker. Do not commit machine-specific paths.
- Existing OAuth/unsubscribe tests bind loopback sockets. A socket-denying sandbox fails those
  tests with `Operation not permitted`; use a locally authorized test execution context.
- Put native Node ahead of `/usr/bin` if the system Node is older than the installed pnpm.
  pnpm 11 can auto-install dependencies before scripts. For a verified existing dependency tree,
  the session-only `pnpm_config_verify_deps_before_run=false` avoids that automatic mutation.

## Runtime invariants

- Account IDs select vault entries and database records; they are not IMAP usernames. The
  additive `connect_and_authenticate_as` API supplies the username separately and retains the
  existing connector trait and authentication API.
- `mail_engine::SyncProgressEvent` is a core checkpoint event. The desktop event has the wire
  fields `account_id`, `state`, `new_messages_count`, and `triage_alerts`. Adapt between them;
  do not replace the core type to match the UI.
- Save ingestion batches, flag changes and folder checkpoints atomically before publishing
  desktop success events. Keep attachment bytes out of ingestion and storage.
- A folder MOVE assigns a destination UID. Optimistic folder overrides must not overwrite
  canonical IMAP identity. Use COPYUID when available; otherwise invalidate the source cache
  and synchronize the destination. Never infer the new UID or issue blanket EXPUNGE.
- Detailed serde/provider errors may contain credential values. Deserialize the credential
  envelope with a fixed error and return only fixed `IpcError` variants over IPC.
