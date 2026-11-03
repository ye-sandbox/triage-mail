---
id: k-032
title: Digest civil slots and immutable constituent cancellation
date: 2026-11-03
tags: [alerts, scheduling, sqlcipher, dst]
severity: high
affected_components: [crates/storage, crates/triage-ai, apps/desktop/src-tauri]
---

# Digest civil slots and immutable constituent cancellation

Scheduling needs both UTC instants (leases/expiration) and civil slots (hourly/daily
occurrences). A fixed current timezone offset cannot resolve future DST changes.
ScheduleCalendar resolves civil minutes; native resolution chooses earlier overlaps
and scans missing civil minutes to the first valid instant. In a repeated hour, quiet
end's earlier occurrence can already be past while the current wall time is still
quiet: scan forward to its real exit, rather than allowing delivery.

Persist civil watermarks independently of history pruning. Backward civil-clock or
zone changes never reopen an already closed slot; conservative expiration can suppress
new items until civil time advances. Pending items recompute timezone-dependent due
times; sealed payloads never change. One pending summary per destination blocks new
sealing until it terminates. Resume coalesces eligible missed slots without renewing
the oldest constituent's 24-hour retention budget.

A summary's alert_job has no single account/message FK. Keep every constituent linked
in alert_schedule_items after sealing (clear its separate payload). BEFORE DELETE
cancels the entire pending parent summary; the terminal job trigger removes all
constituents. This preserves account/message deletion and unread/dismiss/config reminder
cancellation, including entries omitted by payload bounds. Never rebuild an ambiguous
retry with a new membership, event ID or payload, or automatically replay survivors.

Round-robin staging alone cannot enforce quiet hours: lease selection may pick a retry
from a destination not yet visited. Native callers use claim_alert_with_calendar; the
legacy claim_alert API supplies UTC only and fails closed for local-zone quiet hours. Missing calendar resolution fails closed.

OS timezone tests run in isolated child processes with TZ set before chrono initializes;
never mutate global TZ in a parallel Rust test. Kathmandu used UTC+05:30 at Unix epoch,
so use a modern synthetic instant to test its current +05:45 offset.
