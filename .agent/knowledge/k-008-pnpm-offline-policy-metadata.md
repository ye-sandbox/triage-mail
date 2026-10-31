---
id: "k-008"
title: "Distinguish Offline Package Contents from pnpm Policy Metadata"
date: 2026-10-30
tags: ["pnpm", "offline", "dependencies"]
severity: "medium"
affected_components: ["pnpm-lock.yaml", "apps/desktop/package.json"]
symptoms:
  - "ERR_PNPM_NO_OFFLINE_META"
  - "Lockfile failed supply-chain policy check"
---

## Problem

pnpm 11 may require its registry metadata mirror to evaluate supply-chain policies even for
`--offline --frozen-lockfile --lockfile-only`. Cached installed package contents and a valid
lockfile are insufficient. The mirror can be absent for packages already in the original
lockfile, such as `@asamuzakjp/css-color`.

## Local recovery and verification

Under the repository's network restriction, use only explicitly authorized cached packages.
For [04.4], the user approved cached bits-ui v2. Its exact version, dependency versions,
registry integrity hashes and peer metadata were recovered from an existing local npm
lockfile; package contents were restored only into ignored node_modules. No absolute host
paths, vendored dependencies or policy bypass settings were committed. pnpm normalized the
resulting lockfile but could not finish its offline policy check without the metadata mirror.

Validate the lockfile dependency references and original registry integrity hashes, then run
frontend check, tests and build against the restored dependency tree. These checks establish
application compatibility; they do not establish that a fresh pnpm installation succeeds.
A future clean installation still requires the matching package and policy metadata cache,
or explicit user authorization for registry access. Never silently disable supply-chain
policies or turn an offline cache miss into a network request.
