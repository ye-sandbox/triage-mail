---
id: "k-037"
title: "Offline CLI containers need initialized readable WAL sidecars"
date: 2026-11-05
tags: ["docker", "cli", "sqlcipher", "wal"]
severity: "medium"
affected_components: ["apps/cli", "packaging/cli", "scripts/build-cli-image.sh"]
---

# Offline CLI container and WAL sidecars

## Contract

The image is assembled from `scratch` using a freshly built trusted local CLI and
its resolved allowlisted ELF dependencies. No base-image pull is required. The build
script's private generated context prevents repository-local mail/vaults from entering
Docker's context; `.dockerignore` additionally excludes everything except the recipe
and generated rootfs. Host runtime patches require rebuilding; local image IDs depend
on the build host and toolchain. There is no cross-machine reproducibility claim.

Use existing fallback unlock via stdin with a non-root user and read-only data/root
mounts, no network, disabled Docker logging and bounded private temporary storage.
Do not expose host keyring IPC or use writable data as a workaround.

## WAL pitfall

Read-only SQLCipher still uses SQLite WAL coordination. A populated WAL and initialized
SHM produced by an existing writer can be read through a read-only container mount.
A missing SHM file cannot be created there: the reader fails closed with exit 4, even
when a frozen synthetic main database and WAL contain valid committed data. Cleanly
closed WAL-mode installations can have the same limitation. Never enable immutable
mode, fabricate SHM or mount production data writable to bypass it. See k-036 for the
native reader contract and docs/cli.md for supported usage.

Native reader parity checks can update SHM read marks. A failed assertion comparing
all SHM bytes does not establish a logical database write. Regression checks compare
main DB/WAL/vault bytes and logical unread/queue/schema state; the container's mount
enforces read-only access to SHM. Synthetic fixtures retain their writer until reads
finish so WAL-only committed content is genuinely exercised.

## Evidence and limits

The opt-in Docker test exercises all three commands against encrypted recovered-vault
fixtures, exact native JSON parity, missing/incorrect stdin unlock, missing vault/cache,
unsupported schema and missing SHM. The image was inspected for its native library
closure and absence of shell/desktop/model/source/data artifacts. Validation used Linux
x86_64 with Docker Desktop only. It does not establish native keyring interoperability,
other architectures, large-cache responsiveness or a running headless synchronization
service. No image was published and no live mail or network endpoint was used.
