---
id: "k-013"
title: "Diagnose Node Test Subprocess Denials Before Changing Code"
date: 2026-10-31
tags: ["node", "sandbox", "ci", "testing"]
severity: "medium"
affected_components: ["scripts/ci-guard.test.mjs"]
symptoms:
  - "scripts/ci-guard.test.mjs: 'test failed'"
  - "spawnSync git EPERM"
---

## Problem / Symptom

In the restricted desktop execution sandbox, `node --test` can report only a child test
file failure. Running the file directly exposes the underlying `spawnSync git EPERM`.
The policy scanner integration test creates a synthetic temporary Git repository and
spawns the scanner; these subprocess operations require an authorized execution context.

## Approved Solution

After the user authorized diagnosis following the circuit breaker, direct execution
identified the denied subprocess. Running `node --test scripts/ci-guard.test.mjs` with
approved local subprocess access passed all five tests. No external network is needed.
Use the same Node 24.19.0 as CI; a runtime version change alone does not fix permissions.

## What NEVER to Do

- Do not remove the CLI integration regression or silently skip it to hide a denial.
- Do not treat a generic test-file failure as evidence of a scanner defect.
- Do not repeat failing commands past the repository circuit breaker without approval.
