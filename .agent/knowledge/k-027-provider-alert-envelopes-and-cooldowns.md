---
id: "k-027"
title: "Provider response envelopes and shared alert cooldowns"
date: 2026-11-02
tags: ["alerts", "telegram", "discord", "retry", "vault"]
severity: "high"
affected_components: ["crates/triage-ai", "crates/vault", "crates/storage", "apps/desktop/src-tauri"]
symptoms: ["HTTP success with rejected Telegram message", "Other destinations bypass a provider retry delay"]
---

# Provider response envelopes and shared alert cooldowns

Telegram HTTP 2xx alone is insufficient: require a bounded response envelope with
`ok:true`. Extract only error codes and retry delays; never retain the result/message,
chat details or description. Malformed success fails visibly. Discord defaults to an
empty 204 webhook response; 429 can contain fractional retry_after seconds. Round
milliseconds upward, cap hints to the queue's one-hour bound, and retain HTTP retry
hints as well. Bound total I/O time and streaming response bytes independently.

A per-job delay does not stop another destination using the same bot or webhook.
Persist conservative provider-kind cooldowns in the existing next_start column on
retry, alongside finishing the job transaction. This intentionally pauses unrelated
same-provider destinations too; no credential fingerprint or chat ID enters SQL.
Other providers/local notices remain independent. Restart must retain the cooldown.

Telegram input token/chat ID normalize into the legacy vault URL/bearer envelope for
backward compatibility. The bearer slot holds the chat ID only for this kind and must
NEVER become an Authorization header. Validate the kind-specific fixed route on save
and dispatch; preserving old material requires the same kind. Do not follow returned
chat-migration hints automatically: a changed recipient needs fresh configuration consent.

Provider text ceilings use UTF-16 units conservatively so astral characters cannot
exceed limits. Never split alerts automatically. Truncation can cut the displayed JSON
mid-field; it must carry a visible suffix. Empty Discord allowed_mentions.parse and
SUPPRESS_EMBEDS prevent mail content from pinging recipients or generating embeds.
Telegram uses plain text with link previews disabled. These controls do not guarantee
provider-side idempotency or visibility. Validation remains offline.
