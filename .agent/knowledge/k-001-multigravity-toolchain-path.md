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
  - "exec: /home/<user>/AntigravityProfiles/<profile>/.cargo/bin/zig: not found"
  - "ERR_PNPM_IGNORED_BUILDS"
---

# [k-001] Resolve Multigravity Profile Cargo Linker and Linux Node/pnpm PATH

## Problem / Symptom
When running inside an isolated Multigravity profile (`HOME=/home/<user>/AntigravityProfiles/<profile>`) on WSL2:
1. `cargo` and `rustc` live in `/home/<user>/AntigravityProfiles/<primary-profile>/.cargo/bin`, which also contains a `cc` wrapper script (`exec "$HOME/.cargo/bin/zig" cc "$@"`) that fails when `HOME` points to another profile.
2. Default `PATH` may resolve `/mnt/c/nvm4w/nodejs/pnpm` (Windows binary) instead of the native Linux `node` and `pnpm`.
3. `pnpm v12+` blocks `esbuild` postinstall scripts by default with `ERR_PNPM_IGNORED_BUILDS`.

```text
error: linking with `cc` failed: exit status: 127
  = note: /home/<user>/AntigravityProfiles/<primary-profile>/.cargo/bin/cc: 2: exec: /home/<user>/AntigravityProfiles/<profile>/.cargo/bin/zig: not found
```

---

## Approved Solution
1. Prepend `/usr/bin` before `$CARGO_HOME/bin` so system `/usr/bin/cc` (`gcc`) is used instead of the profile-relative `cc` wrapper, and include `/home/<user>/.nvm/versions/node/v24.19.0/bin` for native Linux `node`/`pnpm`:

```bash
export RUSTUP_HOME="/home/<user>/AntigravityProfiles/<primary-profile>/.rustup"
export CARGO_HOME="/home/<user>/AntigravityProfiles/<primary-profile>/.cargo"
export PATH="/usr/bin:$CARGO_HOME/bin:/home/<user>/.nvm/versions/node/v24.19.0/bin:$PATH"
```

2. In `pnpm-workspace.yaml`, declare `allowBuilds` for `esbuild`:

```yaml
packages:
  - "apps/*"
allowBuilds:
  esbuild: true
```

**Root Cause:** Profile isolation changes `$HOME`, causing `$HOME/.cargo/bin/zig` inside the shared `.cargo/bin/cc` shim to point to a non-existent path unless `/usr/bin/cc` takes precedence on `PATH`.

---

## What NEVER to Do
- ❌ **Anti-Pattern:** Invoking `/mnt/c/nvm4w/nodejs/pnpm` on a Linux ext4 workspace.
  - *Why:* Windows Node/pnpm binaries create Windows-specific symlinks/junctions and fail on Linux native postinstall scripts.
- ❌ **Anti-Pattern:** Modifying or deleting files inside `/home/<user>/AntigravityProfiles/<primary-profile>/.cargo/bin`.
  - *Why:* Mutates shared toolchain state across sibling profiles.

---

## Verification
```bash
export RUSTUP_HOME="/home/<user>/AntigravityProfiles/<primary-profile>/.rustup"
export CARGO_HOME="/home/<user>/AntigravityProfiles/<primary-profile>/.cargo"
export PATH="/usr/bin:$CARGO_HOME/bin:/home/<user>/.nvm/versions/node/v24.19.0/bin:$PATH"
cargo test --workspace && pnpm --filter desktop test
```
