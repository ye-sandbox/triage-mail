# Local CPU ONNX embeddings

The desktop uses learned MiniLM embeddings only after semantic search is explicitly
enabled. Missing or invalid files never cause downloads, cloud requests or projection
fallback. Lexical search remains available while the semantic index is unavailable.
Existing semantic consent and command payloads are retained. The legacy projection
provider remains an explicitly injectable Rust API; the native desktop does not select it.

## Pinned artifacts and installation

Weights/tokenizer: [Xenova/all-MiniLM-L6-v2](https://huggingface.co/Xenova/all-MiniLM-L6-v2/tree/751bff37182d3f1213fa05d7196b954e230abad9)
revision `751bff37182d3f1213fa05d7196b954e230abad9` (Apache-2.0).
CPU library: [Microsoft ONNX Runtime v1.22.0](https://github.com/microsoft/onnxruntime/releases/tag/v1.22.0)
(MIT). Rust integration pins `ort = 2.0.0-rc.10` (MIT/Apache-2.0) and
`tokenizers = 0.21.4` (Apache-2.0). Default/download/fetch features are disabled;
no GPU provider is registered and telemetry is disabled before creating a session.
Preserve upstream licenses/notices with any redistributed files. No weights or native
runtime binaries are committed or bundled in the executable.

All paths below are relative to `$XDG_DATA_HOME/triage-mail/models`, defaulting to
`$HOME/.local/share/triage-mail/models` (`USERPROFILE` substitutes for missing HOME on
Windows). The Linux binaries require a compatible glibc-based system. macOS/Windows
runtime artifacts are pinned, but native execution/signing/packaging on those systems
has not been validated by this Linux session. Unsupported OS/architecture pairs fail closed.

| File | SHA-256 |
| --- | --- |
| `all-MiniLM-L6-v2/model.onnx` (`onnx/model_quantized.onnx` upstream) | `afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1` |
| `all-MiniLM-L6-v2/tokenizer.json` | `da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0` |

Install the real native library file (not a symlink) inside `onnxruntime-1.22.0/`:

| Platform / upstream archive | Library filename | SHA-256 of library |
| --- | --- | --- |
| Linux x64 / `onnxruntime-linux-x64-1.22.0.tgz` | `libonnxruntime.so.1.22.0` | `3da6146e14e7b8aaec625dde11d6114c7457c87a5f93d744897da8781e35c673` |
| Linux ARM64 / `onnxruntime-linux-aarch64-1.22.0.tgz` | `libonnxruntime.so.1.22.0` | `0afd69a0ae38c5099fd0e8604dda398ac43dee67cd9c6394b5142b19e82528de` |
| macOS Intel / `onnxruntime-osx-x86_64-1.22.0.tgz` | `libonnxruntime.1.22.0.dylib` | `283e595e61cf65df7a6b1d59a1616cbd35c8b6399dd90d799d99b71a3ff83160` |
| macOS ARM64 / `onnxruntime-osx-arm64-1.22.0.tgz` | `libonnxruntime.1.22.0.dylib` | `2b885992d3d6fa4130d39ec84a80d7504ff52750027c547bb22c86165f19406a` |
| Windows x64 / `onnxruntime-win-x64-1.22.0.zip` | `onnxruntime.dll` | `579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559` |

Download explicitly from the linked revision/release and verify these hashes before
installation. The app never downloads the native library. The existing Model Files
form accepts **direct HTTPS mirrors serving exactly the pinned weights/tokenizer**,
only after consent. Redirects, URL credentials/query/fragment and automatic retries
remain forbidden. Hugging Face resolve URLs may redirect and therefore fail in that
form; use an explicitly downloaded local copy or a direct mirror. Limits remain 128 MiB
for weights, 8 MiB for tokenizer, two minutes per file. Both hashes must pass before
replacing cache files. Interrupted installs leave inert uniquely named temporary files;
retry uses a fresh name. After a native loader/dependent-library failure, fix the installation
and restart the application. Do not change the runtime while the process is using it.

## Model and indexing contract

The pinned tokenizer adds its special tokens and truncates at 256 tokens, without
padding for single-message inference. Input is first bounded at a UTF-8 boundary to
32 KiB. Index input is subject (at most 8,192 SQL characters), newline, then cached
plain text (at most 32,768 SQL characters), followed by the same byte/token bound.
An empty cached document uses `[empty message]`; empty queries fail. No HTML body or
attachment is fetched for indexing.

The `last_hidden_state` must be finite float32 `[1, tokens, 384]`. Attention-masked mean
pooling, L2 normalization and existing symmetric absmax quantization produce `int8[384]`.
A zero/nonfinite/wrong-shape output fails. One retained session serializes calls with
2 intra-op threads and 1 inter-op thread; the desktop indexes eight documents per batch
on blocking workers and yields between batches. Other work queues have separate loops.
No latency or memory ceiling is promised by those bounds.

Append-only schema v11 tracks the compiled model/preprocessing generation, a random
rebuild epoch and per-message input versions. An initial activation discards unversioned
projection vectors. Source changes invalidate the vector transactionally; inference
finishing after hydration, deletion, generation replacement or rebuild is rejected.
Restart preserves completed work. Failed documents remain visible in counts and require
an explicit rebuild to retry. Disable stops future work/commits; re-enable resumes.

Readiness is checked inside the ranking snapshot for ordinary/scoped/category/saved/
sender-review/conversation semantic queries. A partial or incompatible managed index
fails closed, including while a new message awaits indexing. Word search stays available.
Managed global hybrid search uses a single read transaction and the existing sequential
exact ranking path to share that readiness snapshot. Unmanaged headless search retains
its existing parallel path. Exact tie/NULL behavior and performance targets are unchanged.

`cmd_embedding_state` returns only status and indexed/pending/failed counts;
`cmd_rebuild_embeddings(consent)` requires explicit consent and semantic opt-in. The
EN/PT-BR controls poll only local state and discard late results on teardown. Existing
`experimental_semantic_search` wire names persist for compatibility.

Backup format version 1 keeps its schema-10 allowlisted payload. v11 adds no exported
columns/tables: vectors/generation/progress are rebuildable and excluded. Both old and
new backups restore into the current fresh schema, leaving semantic consent disabled
and requiring local model installation/reindexing. No indexing step executes automation.

## Offline reproduction

Provision the files above in an isolated data directory; the following commands do
not fetch artifacts. `TRIAGE_ONNX_TEST_DATA` is a test-only path to that directory,
which must contain `models/all-MiniLM-L6-v2/` and `models/onnxruntime-1.22.0/`.

```bash
TRIAGE_ONNX_TEST_DATA="$PWD/.agent/scratch/onnx/data" \
  cargo test --offline -p triage-ai real_model_and_synthetic_graph_execute_offline -- --ignored --nocapture
cargo run --offline --release -p storage --example onnx_index -- "$PWD/.agent/scratch/onnx/data"
/usr/bin/time -v target/release/examples/onnx_index "$PWD/.agent/scratch/onnx/data"
```

Ordinary workspace tests use synthetic fixtures and do not require any native runtime
or weights. The explicit ignored test additionally executes a tiny generated ONNX
Identity graph, rejects a malformed graph and runs the genuine pinned MiniLM model
on synthetic EN/PT-BR text. The example uses a disposable encrypted database and
256 short messages. Neither fixture is a representative relevance benchmark: MiniLM
is English-oriented, and executing Portuguese text does not establish Portuguese or
cross-language retrieval quality. Existing 100k-message/native UI performance deviations
remain open. See the task log for measured results.

## Initial execution evidence (2026-11-05)

Linux x86_64, Rust 1.99.0, pinned CPU runtime/model; no provider calls. The explicit
debug smoke passed the generated Identity graph, malformed-graph rejection and genuine
MiniLM execution. Its final run recorded session preparation 1,046.662 ms and four
short synthetic EN/PT-BR documents in 7.020 ms. These are timings, not relevance scores.

A release run of the standalone encrypted 256-message example recorded session preparation
140.039 ms, indexing 712.080 ms (359.510 rows/s), inference p50/p95 1.572/7.473 ms and
peak process RSS 98,868 KiB. Files were already in the OS cache and other development
checks were active. This is one small repeated-text fixture, not a stable benchmark,
100k-message extrapolation, application memory budget or native-platform certification.
Raw session logs are ignored scratch; the commands above reproduce the measurements.
