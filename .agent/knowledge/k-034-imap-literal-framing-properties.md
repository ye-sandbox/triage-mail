---
id: "k-034"
title: "IMAP literal framing and deterministic parser properties"
date: 2026-11-04
tags: ["imap", "parser", "testing", "utf8"]
severity: "medium"
affected_components: ["crates/mail-engine"]
---

# IMAP literal framing and deterministic parser properties

## Constraints

- FETCH literal payloads are opaque bytes, including NUL, invalid UTF-8, CRLF,
  parentheses and apparent tagged completions. Only control chunks participate in
  framing. Preserve surplus buffered bytes for the next command.
- A malformed or overflowing literal marker must fail, rather than being interpreted
  as an ordinary completed FETCH. Empty separators after literals do not close FETCH;
  keep its parenthesis state across continuations and ignore quoted parentheses.
- BODYSTRUCTURE literals require CRLF and a complete checked byte range. Quoted
  strings must close, retain UTF-8 through unescaping, and a complete root expression
  must consume all tokens. Never clamp truncated literals into accepted metadata.
- The BODYSTRUCTURE API accepts `str`; reject literal ranges that split a UTF-8 code
  point before resuming tokenization. Otherwise a later string slice can panic even
  though the original complete input is valid UTF-8. FETCH payloads still accept any
  bytes, independently of this metadata-only string API.

## Reproduction and coverage

`cargo test -p mail-engine --test parser_properties` runs offline without sockets or
new dependencies. The finite in-memory reader limits input to 64 KiB and 100,000
reads; asynchronous command calls additionally have a five-second timeout. Read
budgets guard immediate EOF retry loops that would not yield to a Tokio timeout.

Seeds 1, 42, 4242 and 0x5eed reproduce generated fixtures. Five fragmentation modes
cover CRLF boundaries, one-byte reads, small prime chunks, 4 KiB chunks and seeded
chunks. Expected metadata/payloads are constructed independently of parsing.
Additional cases cover every truncated prefix of a small FETCH, malformed lengths
and framing, escaped/literal UTF-8 descriptions at depths 0..16, and 1,024 bounded
generated ASCII S-expressions. These deterministic tests run as part of normal
workspace test suites.

## Coverage-guided fuzzing harness

- The `fuzz/` package provides isolated libFuzzer harnesses for `bodystructure` and
  `fetch` without adding fuzz dependencies to production crate builds. Deterministic
  seed replays run offline via `cargo test --manifest-path fuzz/Cargo.toml`.
- Harnesses bound inputs to 64 KiB, in-memory stream reads to 100,000, and enforce
  process-level limits (`-rss_limit_mb=2048`, `-malloc_limit_mb=256`, `-timeout=5`).
  Process memory bounds are necessary because synchronous allocation loops or deep
  recursion cannot rely on async Tokio timeouts for containment.
- Initial 5-minute campaigns (seed 4242) completed with 0 crashes, 0 timeouts, 0 OOMs:
  `bodystructure` executed 2.34M units (~7.7k/s, peak RSS 568 MB); `fetch` executed
  563k units (~1.8k/s, peak RSS 419 MB). Bounded campaigns demonstrate local parser
  stability under mutation, but do not prove grammar exhaustiveness or replace production
  resource limits.
