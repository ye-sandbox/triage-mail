---
id: "k-010"
title: "Transient Onboarding Inputs and Native OAuth Readiness"
date: 2026-10-31
tags: ["svelte", "forms", "oauth", "ipc", "credentials"]
severity: "medium"
affected_components: ["apps/desktop/src/lib/onboarding", "apps/desktop/src-tauri/src/onboarding.rs"]
---

## Svelte form cleanup

`form.reset()` can clear inputs whose Svelte `value` property is not reflected in the native
`defaultValue` attribute. In onboarding this erased the initial IMAP port. A cleanup that
reads the latest keyed form reference can also miss the detached form being replaced.

Keep secret inputs unbound; clear only `[data-credential]` values after invoking IPC. Capture
the current form inside the effect and clear that captured element during cleanup. This
clears credentials in detached inputs on mode changes and teardown while retaining safe
connection defaults. Regression tests keep references to the old inputs to verify cleanup.
JavaScript string memory cannot be reliably zeroized; no credential is retained in reactive
state, browser storage, error text or logs. Native input buffers use zeroizing wrappers.

## Native readiness and cancellation

Tauri schedules asynchronous commands separately from synchronous cancellation. Offering
Cancel before Rust registers the session can lose a cancellation request. The authorization
command sends `onReady: Channel<()>` immediately after acquiring its per-window lease. The UI
enables cancellation only after this acknowledgement; if the UI has already been disposed,
the acknowledgement instead triggers cancellation. Settled channel handlers ignore late data.

Cancellation and the five-minute deadline race the callback/token exchange only. Persistence
then completes without cancellation, and the UI reloads accounts to reconcile a concurrent
successful commit. Temporary client credentials are stored through `SecretStore` during the
callback wait and deleted on normal completion, cancellation and failure. An abrupt process
termination can leave an orphan staging entry in the vault; it contains no mail account
metadata and is never used to resume an authorization attempt.

Tests use synthetic reserved domains, injected browser/token exchange behavior and actual
loopback callbacks. They do not establish compatibility with a live provider registration.
OAuth providers must support a dynamic-port loopback redirect with the `/callback` path.
Automatic token refresh remains unimplemented; onboarding stores the existing auth-material
representation without changing core traits or the database schema.
