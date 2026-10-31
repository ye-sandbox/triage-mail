---
id: "k-009"
title: "Validate Unsubscribe DNS Answers at Connection Time"
date: 2026-10-30
tags: ["unsubscribe", "dns", "ssrf", "reqwest"]
severity: "high"
affected_components:
  [
    "crates/mail-engine/src/unsubscribe.rs",
    "apps/desktop/src-tauri/src/remote.rs",
  ]
symptoms:
  - "A syntactically public unsubscribe hostname can resolve to a private address"
  - "unsubscribe DNS returned a non-public or empty address set"
---

## Problem / Symptom

The desktop validates stored RFC 8058 headers before dispatching an explicit unsubscribe
action. Hostname syntax checks reject IP literals and internal names, but a public-looking
hostname can still resolve to loopback, private, link-local, or other special-use addresses.
Redirect blocking alone does not close this path. Environment proxies can also resolve the
destination independently of a client's DNS policy.

## Approved Solution

The unsubscribe client's private resolver validates the complete address set returned by
the system resolver and passes that same set to reqwest's connector. It rejects empty or
mixed public/private answers. No separate validation lookup precedes a second connection
lookup. IPv4 special-use ranges and IPv6 local, mapped, transition, documentation, and
special-protocol ranges are excluded conservatively.

Disable inherited proxies with `ClientBuilder::no_proxy()` and retain redirect blocking,
HTTPS validation, fixed RFC 8058 form data, and the existing timeout. System TLS validation
and hostname/SNI remain unchanged. The pre-existing test mode permits loopback literals;
the DNS exception is restricted to `localhost` resolving exclusively to loopback addresses.

## What NEVER to Do

- Do not treat a dotted DNS name as proof of a public destination.
- Do not validate one DNS answer and let the HTTP connector resolve the host again.
- Do not silently discard unsafe answers and allow fallback through a mixed answer set.
- Do not inherit a proxy that bypasses destination address checks.
- Do not weaken TLS checks or enable loopback test mode in desktop production code.

## Verification

Four new regressions cover address ranges, empty/mixed answers, test-mode isolation, and
actual reqwest connector rejection before connection. DNS answers are injected locally;
no live DNS or external endpoint is required. Existing loopback POST integration tests
still verify the exact form body.

Conservative exclusions may reject uncommon special-purpose globally reachable addresses.
The policy does not attempt to detect arbitrary local routing of otherwise public IPs.
