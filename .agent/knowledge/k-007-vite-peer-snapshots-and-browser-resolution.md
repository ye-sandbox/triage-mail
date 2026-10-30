---
id: "k-007"
title: "Unify Vite Peer Snapshots and Preserve Browser Resolution Defaults"
date: 2026-10-30
tags: ["vite", "pnpm", "svelte", "typescript"]
severity: "medium"
affected_components: ["pnpm-lock.yaml", "apps/desktop/vite.config.ts"]
symptoms:
  - "Excessive stack depth comparing types"
  - "Two different types with this name exist, but they are unrelated."
  - "Module node:async_hooks has been externalized for browser compatibility"
---

## Problem / Symptom

Combining cached dependency trees introduced two Vite 6.4.5 snapshots: one without optional
peers and one with `jiti`/`lightningcss`. Tailwind's Vite plugin and Vitest then imported different
recursive `Plugin`/`Environment` types. The resulting error appeared in `defineConfig` even
though both packages used the same Vite version.

A separate configuration issue occurs when `resolve.conditions` is set to `[]` outside tests:
this replaces Vite's browser defaults, allowing Svelte's server runtime into the client bundle.

## Approved Solution

Declare the optional Vite peers consistently and align all Vite consumer snapshot references
in the lockfile to the same peer-qualified release. Preserve package integrity hashes and
reinstall the reconciled lockfile using frozen-lockfile mode. Verify consumer symlinks point
to the same Vite package. An offline reconstruction may require explicit snapshot reconciliation
when the package manager cannot read an older metadata-cache format; do not change versions
or integrity hashes to work around absent metadata.

Enable `resolve.conditions: ['browser']` only in Vitest's test mode, so component tests use
Svelte's DOM renderer. Omit the override in production to retain Vite's default browser
conditions. jsdom does not implement layout or ResizeObserver: stub only that API in unit tests
and verify real layout/scrolling separately in a browser.

## What NEVER to Do

- Do not cast plugins to `any`, disable strict checking or suppress the recursive type error.
- Do not set empty production resolution conditions to accommodate a test-only renderer.
- Do not interpret jsdom timing as a browser frame-rate measurement.

## Verification

Frontend check must report zero errors/warnings; frontend tests and SPA build must pass.
The production build must not import Svelte server code into the browser bundle.
