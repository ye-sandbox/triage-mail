---
id: "k-022"
title: "Provider discovery trust boundaries and RFC 5322 quoted local parts"
date: 2026-11-02
tags: ["discovery", "imap", "mail-parser", "security"]
severity: "high"
affected_components: ["crates/mail-engine", "apps/desktop"]
---

# Provider discovery trust boundaries and quoted local parts

## Symptoms

- Automatic ISPDB lookup leaks a custom account domain before the user chooses online discovery.
- A TCP listener is incorrectly presented as a verified IMAP service.
- A quoted bare addr-spec such as `"local part"@example.com` is interpreted as a display name
  by mail-parser's permissive header parser, yielding no usable mailbox domain.
- Binding a disclosure's `open` directly to discovery status closes advanced settings as
  soon as a manual edit clears that status.

## Invariants

- Offline presets are automatic; online lookup and confirmed-host probing are distinct
  explicit actions. A lookup failure never implicitly triggers DNS/TCP probes.
- ISPDB receives only a normalized domain, with a three-second transfer deadline, no
  proxies/redirects, public-only connection-time DNS and a 64 KiB response cap. Parse
  implicit-TLS IMAP only; never trust XML OAuth endpoints, documentation URLs or DTDs.
- Probe the exact displayed host on port 993 only after confirmation. Verify one public
  DNS snapshot, TLS hostname/chain, and a bounded IMAP OK greeting without sending auth.
- Parse exactly one mailbox. Handle bare quoted local parts explicitly before the header
  parser; retain strict domain validation and reject multi-address/header injection input.
  Do not extract only the final mailbox from a list or include local parts in host templates.
- Invalidate late suggestions after email changes, manual edits, preset choices, credential
  entry and disposal. Capture actual input elements when clearing detached credentials.
- Keep disclosure openness independent of transient discovery status. Unresolved discovery
  may open the panel, but clearing status must not close it during manual input.

## Verification

`cargo test -p mail-engine discovery` covers parsing, XML limits, explicit networking and
paused-clock deadlines. `pnpm --filter desktop test` covers stale requests, manual edits,
credential cleanup and disclosure behavior. Tests use injected transports and reserved domains.

## Provider references

- [Thunderbird file format](https://github.com/thunderbird/autoconfig/wiki/Autoconfig-file-reference)
- [UOL IMAP setup](https://faq.uol.com.br/uolpd/como-configurar-imap-no-outlook-manualmente/)
- [BOL IMAP and paid-plan prerequisite](https://faq.uol.com.br/uolpd/quais-os-servidores-imap-pop-e-smtp-do-bol-mail/)
- [Terra IMAP setup](https://servicos.terra.com.br/para-voce/terra-mail/ajuda/configurando-outlook)
- [Zoho personal and organization endpoints](https://www.zoho.com/mail/help/imap-access.html)
- [Microsoft OAuth2/IMAP settings](https://support.microsoft.com/en-us/outlook/pop-imap-and-smtp-settings-for-outlook-com)
