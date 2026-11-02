# Saved-view validation ([04.10])

Measured 2026-11-02 using `cargo run --offline --release -p storage --example saved_view_performance`.
The disposable encrypted SQLCipher fixture contains 100,000 messages, 100,000 384-dimensional
int8 vectors, four accounts and 200,000 label memberships. No providers, credentials, model
downloads or external requests are involved. Two warmups precede 30 samples per workload.
Timings include native definition validation, filtering, ranking and row hydration, excluding
IPC, embedding generation and rendering. p50/p95 use nearest-rank samples 15/29 of 30.

| Mode | Account selection | Eligible rows | p50 ms | p95 ms | Max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| Blank lexical | All | 124 | 17.192 | 19.111 | 21.162 |
| Blank lexical | Two selected | 62 | 20.602 | 21.812 | 21.886 |
| Semantic | All | 124 | 9.164 | 10.178 | 10.812 |
| Semantic | Two selected | 62 | 10.235 | 13.331 | 14.794 |
| Hybrid | All | 124 | 22.633 | 25.249 | 26.341 |
| Hybrid | Two selected | 62 | 25.773 | 28.801 | 28.812 |

All six workloads require starred mail. Semantic/hybrid use a deterministic synthetic vector;
hybrid also searches `newsletter`. Independent fixture logic checks complete membership,
uniqueness, account isolation and canonical folder/UID across pages before timing. Storage
unit regressions independently cover 600-message full page unions, lexical/semantic/hybrid
eligibility and order, multi-account and mailbox scopes, duplicate memberships, missing
accounts, encrypted reopen, revisions/tombstones, missing extraction facts, parsed sender
identity, triage membership changes and UTC date boundaries. Desktop tests cover stale
responses, save/delete flows, dirty/busy guards and completion after teardown. Registered
Tauri mock handlers test additive command argument casing, revision and pagination rejection.

These are narrow starred populations, not broad sender/extraction or dense semantic scans.
They do not establish universal saved-view latency, native first-frame budgets or 60fps.
The lexical runs fit ADR-008's 40ms lexical budget, semantic runs its 15ms filtered budget,
and hybrid runs its 60ms budget for these fixtures only. Existing mailbox/native deviations
in [mailbox-validation.md](mailbox-validation.md) remain unchanged. No target is weakened.
