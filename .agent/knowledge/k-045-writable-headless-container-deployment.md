---
id: "k-045"
title: "Writable Headless Container Deployment and Lock Topology"
date: 2026-11-07
tags: ["docker", "container", "cli", "ownership", "tls", "signals"]
severity: "medium"
affected_components: ["apps/cli", "packaging/cli", "scripts/build-cli-image.sh", "docs/cli.md"]
---

# Writable Headless Container Deployment and Lock Topology

- **Sibling Lock Topology Invariant:** `WriterOwnership` acquires an advisory file lock
  at `.triage-mail-writer-<name>.lock` residing in the **parent directory** of `--data-dir`
  with mode `0o600`. When deploying in a container with a read-only root filesystem (`--read-only`),
  never configure `--data-dir` as the mount root (e.g. `--data-dir /data`), because its parent `/`
  is read-only and lock creation fails with exit code 4. Always use a subpath within the writable
  volume (e.g. `--data-dir /data/app` with `/data` mounted writable), ensuring the lock file is created
  at `/data/.triage-mail-writer-app.lock` under the container's non-root UID/GID (`65532:65532` or `--user`).
- **Cross-Boundary Contention:** Because `flock` operates at the Linux inode and open file description
  level across namespaces, lock contention between host writers and container writers is fully respected.
  When the host or another container holds the lock, subsequent `sync` or `daemon` processes refuse to start
  and exit cleanly with deterministic status code 7 without mutating data or vault files.
- **Rootless Network & In-Tree TLS Roots:** Outbound TLS for standard IMAP (port 993) uses compiled-in
  Mozilla root certificates (`webpki-roots::TLS_SERVER_ROOTS`), removing any need for host certificate
  bundle mounts (`/etc/ssl/certs`). Self-signed loopback TLS (`ProtonBridgeLocal`) is strictly bounded
  to loopback destinations (`127.0.0.1`, `::1`, `localhost`). On container bridge networks or Docker Desktop,
  bridge container network namespaces (`--network=container:<bridge>`) or dedicated host bridges provide
  the loopback route.
- **PID 1 Signal Drainage:** The CLI registers POSIX `SIGTERM` and `SIGINT` handlers. When running as
  PID 1 in a container, `docker stop` signals the daemon directly. The process finishes active in-flight
  sync ticks, releases `WriterOwnership`, flushes NDJSON lifecycle markers, and exits with code 0.
- **Offline Hermetic Testing:** Container regressions run against local synthetic fixtures without
  network pulls (`--pull=never`) using opt-in test flags (`TRIAGE_CLI_TEST_IMAGE`).
