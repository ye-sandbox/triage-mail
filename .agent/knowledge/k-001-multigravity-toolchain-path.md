---
id: "k-001"
title: "Resolve Multigravity Profile Cargo Linker and Linux Node/pnpm PATH"
date: 2026-10-29
tags:
  - "cargo"
  - "rustup"
  - "pnpm"
  - "multigravity"
  - "wsl"
severity: "medium"
affected_components:
  - "Cargo.toml"
  - "pnpm-workspace.yaml"
symptoms:
  - "rustc: command not found"
  - "exec: /home/yegear/AntigravityProfiles/yegear/.cargo/bin/zig: not found"
  - "ERR_PNPM_IGNORED_BUILDS"
---

# [k-001] Resolve Multigravity Profile Cargo Linker and Linux Node/pnpm PATH

## Problem / Symptom
When running inside an isolated Multigravity profile (`HOME=/home/yegear/AntigravityProfiles/yegear`) on WSL2:
1. `cargo` and `rustc` live in `/home/yegear/AntigravityProfiles/luisfmb/.cargo/bin`, which also contains a `cc` wrapper script (`exec "$HOME/.cargo/bin/zig" cc "$@"`) that fails when `HOME` points to another profile.
2. Default `PATH` may resolve `/mnt/c/nvm4w/nodejs/pnpm` (Windows binary) instead of the native Linux `node` and `pnpm`.
3. `pnpm v12+` blocks `esbuild` postinstall scripts by default with `ERR_PNPM_IGNORED_BUILDS`.

```text
error: linking with `cc` failed: exit status: 127
  = note: /home/yegear/AntigravityProfiles/luisfmb/.cargo/bin/cc: 2: exec: /home/yegear/AntigravityProfiles/yegear/.cargo/bin/zig: not found
```

---

## Approved Solution
1. Prepend `/usr/bin` before `$CARGO_HOME/bin` so system `/usr/bin/cc` (`gcc`) is used instead of the profile-relative `cc` wrapper, and include `/home/yegear/.nvm/versions/node/v24.19.0/bin` for native Linux `node`/`pnpm`:

```bash
export RUSTUP_HOME="/home/yegear/AntigravityProfiles/luisfmb/.rustup"
export CARGO_HOME="/home/yegear/AntigravityProfiles/luisfmb/.cargo"
export PATH="/usr/bin:$CARGO_HOME/bin:/home/yegear/.nvm/versions/node/v24.19.0/bin:$PATH"
```

2. In `pnpm-workspace.yaml`, declare `allowBuilds` for `esbuild`:

```yaml
packages:
  - "apps/*"
allowBuilds:
  esbuild: true
```

**Root Cause:** Profile isolation changes `$HOME`, causing `$HOME/.cargo/bin/zig` inside the shared `.cargo/bin/cc` shim to point to a non-existent path unless `/usr/bin/cc` takes precedence on `PATH`.

### Direct toolchain and clippy fallback (2026-11-02)

If the login environment has no Cargo on PATH, use the existing stable toolchain `bin`
directory and set `CARGO_HOME`, `RUSTUP_HOME`, and `RUSTUP_TOOLCHAIN=stable` for that command.
Do not pass `+stable` to the direct Cargo executable; toolchain selectors belong to the
rustup proxy. Cargo subcommands such as fmt still need the rustup environment.

Clippy subprocesses can resolve the shared profile's broken `cc` wrapper even when a
previous Cargo check passed. Set `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/usr/bin/cc`
and `CC=/usr/bin/cc` for validation instead of editing shared wrappers or changing HOME.
This does not supply GTK/WebKit development dependencies; follow k-006 for native builds.

---

## What NEVER to Do
- ❌ **Anti-Pattern:** Invoking `/mnt/c/nvm4w/nodejs/pnpm` on a Linux ext4 workspace.
  - *Why:* Windows Node/pnpm binaries create Windows-specific symlinks/junctions and fail on Linux native postinstall scripts.
- ❌ **Anti-Pattern:** Modifying or deleting files inside `/home/yegear/AntigravityProfiles/luisfmb/.cargo/bin`.
  - *Why:* Mutates shared toolchain state across sibling profiles.

---

## Verification
```bash
export RUSTUP_HOME="/home/yegear/AntigravityProfiles/luisfmb/.rustup"
export CARGO_HOME="/home/yegear/AntigravityProfiles/luisfmb/.cargo"
export PATH="/usr/bin:$CARGO_HOME/bin:/home/yegear/.nvm/versions/node/v24.19.0/bin:$PATH"
cargo test --workspace && pnpm --filter desktop test
```
