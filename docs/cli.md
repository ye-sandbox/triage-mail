# Offline cached-mail CLI

Build the standalone Rust executable without the desktop dependency graph:

```bash
cargo build --release --locked -p triage-mail-cli
cargo test --locked -p triage-mail-cli
target/release/triage-mail-cli --help
```

The package needs the Rust/native C build tools used by SQLCipher's bundled OpenSSL,
but no Node, Tauri, GTK or WebKit. Dependency installation is a development step;
the executable itself makes no IMAP/HTTP calls, downloads or deliveries. It starts
no background workers and never loads the ONNX runtime.

## Existing installation

Always supply `--data-dir PATH` pointing to the desktop's existing app-data directory
containing `mail.db`. The CLI does not consult `.env`, infer a path or initialize an
installation. It reads the existing `application/database-key` vault entry. Without
`--unlock-stdin` it uses the OS keyring; local keyring IPC may be required. It never
creates a key or changes a keyring entry.

With `--unlock-stdin`, only the existing `vault.enc` is used. Supply the passphrase
through a trusted password-manager pipe or private file descriptor, followed by EOF.
No secret value is accepted through arguments or environment variables. Interactive
terminal stdin is refused to avoid echo. Input is UTF-8, nonempty, at most 4,096 bytes
including an optional final LF/CRLF, with no embedded newline, CR or NUL; spaces are
preserved. Buffers are zeroized. Vault files are bounded to 16 MiB before parsing.
The `recovered-local-vault` marker requires this fallback-only mode, just as in startup
recovery. A missing, malformed or incorrectly unlocked vault fails without creation.
Direct symlinks for the data directory, database, fallback vault and recovery marker
are refused; this is not a security boundary against concurrent hostile filesystem
replacement or symlinked ancestors. Use a trusted private local directory.

## Commands and JSON

```bash
# OS-keyring-backed installation
target/release/triage-mail-cli --data-dir /path/to/triage-mail accounts
target/release/triage-mail-cli --data-dir /path/to/triage-mail search --query boleto --account acct-a --limit 20 --offset 0
target/release/triage-mail-cli --data-dir /path/to/triage-mail show --message-id msg-a

# Fallback vault; fd 3 was already opened privately by the caller
target/release/triage-mail-cli --data-dir /path/to/triage-mail --unlock-stdin accounts <&3
```

| Command    | Output and options                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `accounts` | JSON array of opaque account IDs ordered by ID; no addresses or connection settings. `--limit` and `--offset` paginate without imposing an account limit.                                                                                                                                                                                                                                                                            |
| `search`   | JSON array of existing `MessageRowSummary` objects: IDs, subject/sender, time, flags, category/extraction badges, tags and local folder/snooze state. `--query` defaults to empty (newest first); otherwise uses the existing literal lexical FTS phrase behavior and stored tokenizer. `--account` restricts by exact opaque account ID; an unknown account yields an empty array. No mailbox/category/review filtering is implied. |
| `show`     | JSON object `{ "id": "msg-a", "account_id": "acct-a", "text_plain": "cached text", "truncated": false }`. Required `--message-id`; no HTML, headers or attachments. Missing cached plain text is `null`, including HTML-only mail; no fallback conversion or lazy fetch occurs.                                                                                                                                                      |

`accounts` and `search` default to 100 rows; `--limit` accepts 1..500 and `--offset`
accepts nonnegative decimal integers whose page end fits SQLite's signed 64-bit range.
Queries are at most 32 KiB. Account/message arguments are nonempty opaque IDs at most
256 UTF-8 bytes, without control characters or `@`. Unknown/duplicate options and
options belonging to another command fail. `--help` alone needs no installation.

`show` loads at most 64 KiB of body bytes, dropping an incomplete final UTF-8 scalar
and reporting `truncated`. Each successful response is one JSON value plus LF,
at most 4 MiB including that LF. Control characters are JSON-escaped. Serialization
finishes before printing; oversized responses produce a fixed error and no partial
mail JSON. The output ceiling does not bound all internal FTS/hydration memory or
query latency. A broken stdout stream can still interrupt a response during writing.

Stdout contains private mail. Keep pipes and destinations private; the CLI creates
no plaintext output files or logs. Stderr contains fixed diagnostics, never query,
path, passphrase, provider response or message values. Exit codes are stable:

| Code | Meaning                                                                  |
| ---- | ------------------------------------------------------------------------ |
| 0    | Successful command or help                                               |
| 2    | Invalid arguments                                                        |
| 3    | Existing vault/key unavailable, malformed, or wrong passphrase           |
| 4    | Cache/path unavailable, invalid key, query failure or unsupported schema |
| 5    | Requested message absent from cache                                      |
| 6    | Input/output unavailable or response/input exceeds bounds                |

## Storage and compatibility

The current executable accepts schema 11 only, validating the complete migration
sequence in the same read snapshot as each command. Older/newer databases fail;
open supported older data in a compatible desktop to migrate it first. It does not
migrate, repair, rebuild indexes, mark messages read or process pending work.

The reader uses SQLCipher `READONLY`, `query_only=ON`, memory temporary storage and
a five-second SQLite busy timeout. It does not change journal mode. Committed WAL
data is included; never copy `mail.db` alone while desktop writes are active. SQLite
can require readable existing `-wal`/`-shm` sidecars or directory permissions to create
shared-memory sidecars; logical read-only access does not promise byte-for-byte
filesystem immutability. Do not use SQLite immutable mode to bypass this requirement.
Concurrent commits appear on subsequent commands; pagination across invocations is
not a pinned snapshot. Desktop schema upgrades can make a later command fail closed.

Offline tests cover fallback unlock, recovery gating, wrong/missing keys, schema
rejection, WAL visibility/concurrent writes, exact lexical behavior, account scopes,
pagination, direct symlinks, control bytes and output bounds. They do not establish
live OS-keyring interoperability on every platform or native platform packaging.
Sync, daemon operation, account setup, mutations, semantic indexing and all
external actions remain outside this executable's command surface.

## Linux container packaging

Use a trusted Linux x86_64 GNU build host with Cargo, cached locked dependencies,
C/native build tools, Node.js (Cargo metadata parsing), GNU coreutils and Docker.
The host must provide Debian/Ubuntu runtime notices at
`/usr/share/doc/libc6/copyright` and `/usr/share/doc/libgcc-s1/copyright`:

```bash
scripts/build-cli-image.sh triage-mail-cli:local
docker image inspect triage-mail-cli:local --format '{{.Id}} {{.Size}}'
```

The script builds the CLI with `--offline --locked --release` for the explicit
`x86_64-unknown-linux-gnu` target, then creates a private
throwaway build context containing only that executable, its allowlisted ELF loader
and runtime libraries, and license notices. The base is Docker's empty `scratch`;
there is no base-image tag to pull. Docker build uses `--pull=false --network none`.
No package installation, compiler image, registry access, source tree, mail, vault,
models, Node runtime or desktop libraries enter the image. The context is deleted
on exit. The image contains no shell or background entrypoint. Its default command
is `--help` and its default UID/GID is `65532:65532`.

Packaging currently supports Linux x86_64 GNU only, with `libc.so.6`, `libm.so.6`,
`libgcc_s.so.1` and `ld-linux-x86-64.so.2`; unexpected linkage fails the build for
review. It copies the build host's resolved libraries, so artifacts depend on the
host/toolchain and are not reproducible across arbitrary machines. Rebuild with
updated trusted host libraries for runtime fixes. Inspect the resulting image ID
before use. No image is published by the script. Redistribution requires the
corresponding source and applicable dependency license compliance.

Supply an existing private app-data directory as a **read-only** bind mount. Use a
nonzero host UID/GID with permission to read its files; do not change production
permissions merely to satisfy the image's default UID. With fd 3 already privately
opened by the caller to supply the fallback passphrase:

```bash
docker run --rm --pull=never --network=none --read-only \
  --user "$(id -u):$(id -g)" --cap-drop=ALL \
  --security-opt=no-new-privileges --log-driver=none \
  --tmpfs=/tmp:rw,noexec,nosuid,nodev,size=16m,mode=1777 \
  --mount type=bind,src=/path/to/triage-mail,dst=/data,readonly \
  -i triage-mail-cli:local --data-dir /data --unlock-stdin accounts <&3
```

Run as a non-root host user. Replace `accounts` with the existing `search` or `show`
arguments as needed. Keep `-i` to forward stdin and do not add `-t`. Never put the
passphrase in a command, environment variable, image layer or bind-mounted secret
file. Do not mount a host keyring/D-Bus socket. The container uses only the existing
fallback vault; installations without one are unsupported by this recipe. The
recovered-vault marker remains authoritative. No ports are published, capabilities
are dropped, privilege escalation is disabled, and Docker's log driver is disabled
because stdout contains private mail. The caller still owns stdout/pipe privacy.
The temporary filesystem is limited to 16 MiB and is not the data mount.

The same WAL/SHM requirements apply inside the container. Existing readable,
initialized `mail.db-wal` and `mail.db-shm` allow committed WAL reads. Missing or
unusable sidecars can cause exit 4 on a read-only mount, including a cleanly closed
WAL-mode installation. Do not switch to a writable data mount or SQLite immutable
mode. Use a supported existing installation with usable sidecars, or inspect it
with the native CLI under its documented filesystem permissions. Never copy only
the live main database, and never fabricate a SHM file. The container does not
repair sidecars, checkpoint, migrate or initialize storage.

Opt-in Docker regression (uses synthetic encrypted data only; never pulls an image):

```bash
TRIAGE_CLI_TEST_IMAGE=triage-mail-cli:local cargo test --offline --locked -p triage-mail-cli --test offline container_reads_readonly_encrypted_wal_and_fails_closed -- --ignored --exact
```

This covers default help/non-root image configuration, native/container JSON parity
for all three commands, recovered fallback unlock, wrong/empty passphrases, missing
vault/cache, unsupported schema, committed WAL visibility, missing SHM refusal and
mail/queue preservation with read-only mounts. Ordinary workspace tests skip it
when Docker is unavailable. Runtime validation has been performed on Linux x86_64
with Docker Desktop; other architectures and native host keyrings are not covered.
This container performs offline cached-mail inspection only. Active synchronization,
daemon operation, account setup and automation still require separate work.
