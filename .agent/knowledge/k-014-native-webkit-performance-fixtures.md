---
id: "k-014"
title: "Resolve Extracted WebKit Helpers and Static Fixture Asset Paths"
date: "2026-10-31"
tags: ["webkitgtk", "tauri", "sveltekit", "performance"]
severity: "medium"
affected_components: ["tools/performance", "apps/desktop/src-tauri/examples/native_performance.rs"]
symptoms:
  - "Unable to spawn a new child process"
  - "WebKitNetworkProcess (No such file or directory)"
  - "Native measurement times out without phase reports"
---

## Problem / Symptom

Linking against an extracted GTK/WebKit sysroot does not relocate WebKit's compiled helper
path. `LD_LIBRARY_PATH` finds libraries, but WebKit still launches helpers from
`/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1`. On this cached WebKit 2.52.6 build, no executable-path
environment override was present in the library's strings.

A second, separate issue appears when a flat SvelteKit static route is opened with a trailing
slash. The fixture emits `__performance.html` and relative `./_app` assets. Loading
`__performance/` resolves those assets beneath that directory; the fixture never initializes.

## Approved Solution

Prefer a normally installed supported GTK/WebKit runtime. For the already authorized extracted
sysroot, a temporary bubblewrap mount namespace was verified without modifying host packages:

1. Create an ignored library-directory mirror. Its entries are symlinks to a temporary alias
   of the host's original `/usr/lib/x86_64-linux-gnu` directory. Add a `webkit2gtk-4.1` entry
   pointing to the extracted helper directory, including its injected bundle.
2. Read-only bind the host library directory to that temporary alias, then read-only bind the
   mirror over `/usr/lib/x86_64-linux-gnu`. Retain `/dev`, `/proc`, the graphical session, cached
   library search path and writable isolated temporary XDG data/cache. The root filesystem
   remains read-only; the host namespace is unchanged. WebKit's own sandbox stays enabled.
3. Open the fixture at `__performance` without a trailing slash. Tauri resolves its `.html`
   asset fallback and SvelteKit resolves `./_app` from the root. Wait for `complete`, not merely
   a running window, before accepting frame or memory evidence.

A direct bind to the absent helper directory failed because its parent was read-only. The
library-directory mirror avoids creating a destination beneath the host's real `/usr/lib`.
The verified namespace and local paths are session tooling, not application configuration.

The native example requires `performance-validation`, so clean ordinary workspace checks do
not require generated benchmark assets. After fixture generation, restore the ordinary SPA
before building production binaries. Do not ship the generated fixture route.

## What NEVER to Do

- Do not disable WebKit sandboxing or patch shared libraries to bypass runtime setup.
- Do not install temporary helper symlinks into system directories or commit machine paths.
- Do not treat linking success, a window, startup RSS, or a timeout as native frame evidence.
- Do not claim requestAnimationFrame intervals prove GPU presentation or all-hardware 60fps.

## Wrapper timeout cleanup (2026-11-02)

When the sampler launches a bubblewrap wrapper, terminating only that PID can orphan the
native process and WebKit children. Inherited stdout pipes then prevent the sampler's
`close` event and final JSON output. The sampler now creates a dedicated Unix process group
and terminates that group on timeout. A disposable wrapper with a sleeping child verifies
that timed sampling returns normally. On Windows it retains direct child termination;
Windows descendant cleanup is not established by the Linux regression. Do not accept an
orphaned process or missing `complete` phase as a successful native measurement.
