# Offline IMAP parser fuzzing

The separate `fuzz/` Cargo workspace exercises the production `mail-engine` APIs
without sockets, credentials, providers, model downloads or persistent application
data. It is optional development tooling; normal workspace builds do not compile
libFuzzer. The existing deterministic `parser_properties` integration tests remain
part of `cargo test --workspace`.

## Tooling and dependency preparation

Provisioning requires explicit permission under the repository network policy.
Install cargo-fuzz and a nightly Rust toolchain into a development environment;
do not modify shared toolchain binaries. This task used cargo-fuzz 0.13.2,
libfuzzer-sys 0.4.13 and rustc 1.101.0-nightly (2d20b42b8 2026-11-04) on x86_64
Linux. The harness lockfile retains the root lockfile's dependency versions and
adds only libfuzzer-sys/arbitrary. A future root dependency update must update
the fuzz lockfile too. Nightly/compiler changes can alter coverage and throughput.

After authorized provisioning, fetch the committed fuzz lockfile once:

```bash
cargo fetch --locked --manifest-path fuzz/Cargo.toml
```

All commands below run offline. Set the appropriate Cargo/rustup environment if
tools live outside the normal PATH. No runtime network exception is needed.

## Deterministic seed replay and lint

```bash
CARGO_NET_OFFLINE=true cargo test --locked --manifest-path fuzz/Cargo.toml
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
CARGO_NET_OFFLINE=true cargo clippy --locked --manifest-path fuzz/Cargo.toml --all-targets -- -D warnings
```

These checks use stable Rust and do not establish coverage-guided exploration.
The replay tests exercise all reviewed seeds plus empty, boundary-sized and
oversized inputs. Fixtures preserve literal CRLF/binary bytes via `.gitattributes`.

## Bounded campaigns

```bash
mkdir -p fuzz/corpus/bodystructure fuzz/corpus/fetch
cp fuzz/seeds/bodystructure/* fuzz/corpus/bodystructure/
cp fuzz/seeds/fetch/* fuzz/corpus/fetch/

CARGO_NET_OFFLINE=true cargo +nightly fuzz run bodystructure -- -max_total_time=300 -max_len=65536 -timeout=5 -rss_limit_mb=2048 -malloc_limit_mb=256 -seed=4242 -print_final_stats=1
CARGO_NET_OFFLINE=true cargo +nightly fuzz run fetch -- -max_total_time=300 -max_len=65536 -timeout=5 -rss_limit_mb=2048 -malloc_limit_mb=256 -seed=4242 -print_final_stats=1
```

Cargo-fuzz enables AddressSanitizer, coverage instrumentation, debug assertions
and overflow checking. The campaign budget is five minutes per target, excluding
build time. Per-input execution is limited to five seconds, process RSS to 2 GiB
and individual allocations to 256 MiB. libFuzzer checks these limits in-process;
they are development containment, not production parser limits or an OS sandbox.
Always use finite budgets. A seed does not reproduce a whole campaign exactly:
compiler, initial corpus, execution budget and scheduling affect mutation order.

`bodystructure` accepts only byte-exact valid UTF-8 at the existing `str` boundary,
compares repeated parses, and exercises extracted attachment/text metadata on
success. Malformed strings are allowed to return errors; no panic is swallowed.

`fetch` parses arbitrary bytes under 4 KiB and small fragmented reads and compares
both the first command and, on success, the following command. Independently
generated valid FETCH frames wrap up to 60 KiB of opaque payload, with header,
empty section, metadata after literals and a following tagged response. Three read
modes include one-byte reads; assertions compare exact expected fields and bytes.
The complete generated wire remains under 64 KiB. Every stream returns immediate
EOF and fails after 100,000 read calls. A one-second Tokio timeout detects pending
I/O; libFuzzer limits are needed for synchronous loops, stack overflow or allocation
faults that never yield to Tokio. No authentication command is executed.

## Findings and replay

Generated corpora, coverage output and crash/timeout/OOM artifacts are ignored.
Use synthetic inputs only; never import real server captures or private mail.
Commit only manually reviewed seeds and deterministic regression cases. Before
sharing a failure, inspect its bytes and log for private data.

```bash
# Replace the artifact path with the local file reported by libFuzzer.
CARGO_NET_OFFLINE=true cargo +nightly fuzz run fetch fuzz/artifacts/fetch/crash-LOCAL -- -runs=1 -max_len=65536 -timeout=5 -rss_limit_mb=2048 -malloc_limit_mb=256
```

Replaying a file is not a new campaign and does not enforce mutation length limits
on that file; the harness rejects files over 64 KiB. Minimize a finding with the
same target and limits, preserve its original artifact locally, and add a small
ordinary regression test before fixing a demonstrated defect. Production resource
policy or broad parser redesign requires a separately approved plan. A crash-free
bounded campaign does not prove grammar correctness, exhaustive coverage or safe
production resource use.

## Initial evidence

Campaign executed on 2026-11-04 using cargo-fuzz 0.13.2 / libfuzzer-sys 0.4.13 with
`rustc 1.101.0-nightly (2d20b42b8 2026-11-04)` on x86_64 Linux, bounded to 300s,
64 KiB max input, 5s timeout, 2048 MB RSS, 256 MB malloc, seed 4242:

- **Target `bodystructure`:** 2,343,067 executed units in 301 seconds (~7,784 exec/s).
  Peak RSS 568 MB. 804 corpus units (376 KB), cov 922, ft 3,386.
  Observed defects: 0 panics, 0 timeouts, 0 OOMs, 0 crashes.
- **Target `fetch`:** 563,885 executed units in 301 seconds (~1,873 exec/s).
  Peak RSS 419 MB. 1,163 corpus units (149 KB), cov 1,633, ft 4,426.
  Observed defects: 0 panics, 0 timeouts, 0 OOMs, 0 crashes.
