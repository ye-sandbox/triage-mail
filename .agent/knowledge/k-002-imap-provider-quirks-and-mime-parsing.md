---
id: "k-002"
title: "Normalize IMAP Provider Quirks (pimalaya patterns) and Enforce Zero-Copy MIME Parsing (Stalwart mail-parser)"
date: 2026-10-29
tags:
  - "imap"
  - "mime"
  - "mail-parser"
  - "pimalaya"
  - "oauth2"
  - "mail-engine"
severity: "high"
affected_components:
  - "crates/mail-engine"
symptoms:
  - "Duplicate messages synced across Gmail labels and [Gmail]/All Mail"
  - "Outlook/Microsoft 365 IMAP BAD Command Error on CONDSTORE or XOAUTH2 SASL handshake"
  - "High RAM spikes and slow sync when fetching full RFC822 messages with binary attachments"
  - "Malformed MIME headers or non-UTF8 charsets (ISO-8859-1 / Windows-1252) failing extraction"
---

# [k-002] Normalize IMAP Provider Quirks (`pimalaya` patterns) and Enforce Zero-Copy MIME Parsing (Stalwart `mail-parser`)

## Problem / Symptom

Real-world IMAP servers deviate significantly from strict RFC 3501 / RFC 9051 compliance, and naive MIME fetching causes severe memory/bandwidth bloat:

1. **Gmail IMAP Quirks:**
   - Exposes labels as virtual folders alongside `[Gmail]/All Mail`, causing naive sync engines to download and index the same message multiple times unless deduplicated by `Message-ID` / `X-GM-MSGID` or scoped via RFC 6154 `SPECIAL-USE` (`\All`, `\Trash`, `\Junk`, `\Archive`).
   - Requires exact SASL `XOAUTH2` formatting (`user=<email>\x01auth=Bearer <token>\x01\x01`) and returns custom capabilities (`X-GM-EXT-1`, `CONDSTORE`).
2. **Outlook / Microsoft 365 IMAP Quirks:**
   - Frequently omits or Partially implements `CONDSTORE` (`HIGHESTMODSEQ`), requiring graceful fallback to `UIDVALIDITY` + `UIDNEXT` + `UID FETCH 1:* (UID FLAGS)` delta comparison.
   - Enforces strict connection throttling and drops idle sockets aggressively before the standard 29-minute RFC 2177 `IDLE` renewal window.
3. **Proton Mail Bridge (Localhost `127.0.0.1:1143`):**
   - Uses a locally generated self-signed TLS certificate over STARTTLS/TLS and maps encrypted proton labels to standard IMAP folders with specific `UIDVALIDITY` semantics.
4. **MIME Parsing & Attachment Bloat:**
   - Fetching `RFC822` or `BODY[]` eagerly pulls 25MB+ attachments into memory during background sync, and legacy MIME parsers fail on broken multipart boundaries or legacy Brazilian charsets (`ISO-8859-1`, `Windows-1252`).

---

## Approved Solution

### 1. Provider Quirk Normalization in `crates/mail-engine` (Inspired by `pimalaya / email-lib`)

Isolate all server dialect differences inside `ImapConnector` implementations so callers only interact with normalized domain structs:

- **Capability-Driven Sync Strategy:** Inspect `CAPABILITY` response immediately post-authentication:
  - If `CONDSTORE` is advertised and non-zero `HIGHESTMODSEQ` is returned on `SELECT`/`EXAMINE`, use `FETCH 1:* (UID FLAGS) (CHANGEDSINCE <modseq>)`.
  - Otherwise (e.g., Outlook or minimal IMAP servers), fall back deterministically to `UIDNEXT` watermarks and lightweight `UID FETCH 1:* (UID FLAGS)` reconciliation.
- **RFC 6154 `SPECIAL-USE` Folder Discovery:** Use `LIST "" "*" RETURN (SPECIAL-USE)` (with fallback name heuristics matching `pimalaya` folder aliases) to map `\Inbox`, `\Archive`, `\Trash`, and `\Junk` consistently across Gmail, Outlook, Yahoo, Dovecot, and Proton Bridge.
- **Adaptive `IDLE` Heartbeat:** Refresh IMAP `IDLE` (`DONE` $\rightarrow$ `IDLE`) every **14 minutes** (instead of the theoretical 29-minute RFC 2177 limit) to prevent silent NAT/Outlook firewall drops.

### 2. Zero-Copy MIME & Lazy Attachments via Stalwart Labs' `mail-parser`

Use the [`mail-parser`](https://github.com/stalwartlabs/mail-parser) crate from Stalwart Mail Server for all RFC 5322 / MIME parsing:

```rust
// INVARIANT: Never fetch full RFC822 in bulk sync; fetch ENVELOPE, FLAGS, UID, BODYSTRUCTURE,
// and text/html body parts only. Binary attachments are fetched on-demand via BODY.PEEK[part].
use mail_parser::{MessageParser, MimeHeaders};

pub fn parse_lightweight_message(raw_headers_and_text_parts: &[u8]) -> Option<ParsedEmail> {
    let message = MessageParser::default().parse(raw_headers_and_text_parts)?;
    // Zero-copy extraction with automatic charset decoding (UTF-8, ISO-8859-1, Windows-1252, quoted-printable, base64)
    let message_id = message.message_id().map( str::to_owned );
    let in_reply_to = message.in_reply_to().as_text().map(str::to_owned);
    let subject = message.subject().unwrap_or_default().to_owned();
    let text_body = message.body_text(0).map(|b| b.into_owned());
    let html_body = message.body_html(0).map(|b| b.into_owned());
    // Extract List-Unsubscribe and RFC 8058 List-Unsubscribe-Post headers
    Some(ParsedEmail { message_id, in_reply_to, subject, text_body, html_body })
}
```

**Root Cause:** IMAP servers implement divergent subsets of RFC extensions, and email bodies in the wild frequently violate strict MIME encoding rules. Combining `pimalaya`-style capability/quirk adapters with Stalwart's zero-copy `mail-parser` guarantees resilience without sacrificing memory footprint.

---

## What NEVER to Do

- ❌ **Anti-Pattern:** Fetching `RFC822` or `BODY.PEEK[]` for whole messages during folder synchronization.
  - *Why:* Downloads multi-megabyte PDFs/ZIPs/images eagerly for every email, violating the Lazy Attachment invariant and bloating RAM and SQLCipher storage.
- ❌ **Anti-Pattern:** Hardcoding folder names like `"Archive"` or `"[Gmail]/Todos os e-mails"` in business logic.
  - *Why:* Folder names are localized by providers; always resolve via RFC 6154 `SPECIAL-USE` attributes (`\Archive`, `\Trash`, `\All`) first, with localized fallback tables inside `ImapConnector`.
- ❌ **Anti-Pattern:** Assuming `CONDSTORE` (`HIGHESTMODSEQ`) is supported by all IMAP servers.
  - *Why:* Several providers (including Microsoft/Outlook configurations and custom servers) reject `CHANGEDSINCE` queries with `BAD`. Always branch on server `CAPABILITY`.

---

## Verification

```bash
cargo test -p mail-engine
```

---

## References

- [Stalwart Labs `mail-parser` (GitHub)](https://github.com/stalwartlabs/mail-parser)
- [Pimalaya `email-lib` / `imap` Quirks & Adapters (GitHub)](https://github.com/pimalaya/core)
- RFC 3501 (IMAPv4rev1), RFC 6154 (IMAP LIST Extension for Special-Use Mailboxes), RFC 7162 (CONDSTORE), RFC 8058 (One-Click Unsubscribe)
