# NOTES.md — Active Contracts and Runtime Memory

> Stores the active runtime contracts (WHAT interacts with WHAT), live communication topology, and critical gotchas.
> Historical architectural decisions live in [`docs/adr/decisions.md`](../docs/adr/decisions.md).

---

## Active Contracts

### 1. Rust Workspace Crate Boundaries (`Cargo.toml`)

All core domain crates MUST remain pure Rust (zero dependency on `tauri`) so the engine can run headless (CLI/daemon) or embedded in `apps/desktop/src-tauri`:

| Crate | Responsibility | Public Contract / Traits | Internal Dependencies |
|---|---|---|---|
| `crates/vault` | OS Keyring (`keyring`) & encrypted fallback vault (`argon2` + `aes-gcm` / `chacha20poly1305`) | `SecretStore` trait, `VaultKey`, `MasterCipher` | None (leaf crate, `#![forbid(unsafe_code)]`) |
| `crates/storage` | Single-file SQLite WAL + Full-DB SQLCipher + Configurable `FTS5` (default `unicode61 remove_diacritics 2`) + `sqlite-vec` (`int8` quantized KNN + pre-filtering) + IMAP `UIDVALIDITY`/`MODSEQ` sync state | `MailRepository`, `SearchEngine` (`HybridQuery` -> `<15ms`), `MigrationRunner` | `crates/vault` (`#![deny(unsafe_code)]` with isolated `#[allow(unsafe_code)]` only on `sqlite-vec` FFI init) |
| `crates/mail-engine` | Agnostic IMAP transport (`ImapConnector` trait), incremental sync (`UIDVALIDITY`/`CONDSTORE`), transparent multi-auth (App Passwords, OAuth2 Public PKCE, user-supplied BYO OAuth `client_id`/`client_secret`, Proton Bridge), RFC 8058 One-Click Unsubscribe, `mail-parser` | `ImapConnector`, `IdleManager`, `MimeExtractor`, `SyncEventSink` trait | `crates/vault` (`#![forbid(unsafe_code)]`) |
| `crates/triage-ai` | 3-Tier Triage: Tier 1 (Regex 2FA/OTP, Pix EMV/keys, FEBRABAN Boletos with exact BRL value & due date, NF-e metadata, Tracking, `List-Unsubscribe` + RFC 8058 One-Click), Tier 2 (Local CPU ONNX embeddings downloaded on demand to `$XDG_DATA_HOME`), Tier 3 (Ollama/BYOK HTTP) | `DeterministicTriage`, `EmbeddingProvider` trait, `LlmSummarizer` trait, `TriageEventHook` trait | None (pure data in/out, `#![forbid(unsafe_code)]`) |
| `apps/desktop/src-tauri` | Tauri v2 IPC bridge, system tray, native window lifecycle | Typed `#[tauri::command]` handlers & `tauri::ipc::Channel<SyncEvent>` | `mail-engine`, `storage`, `vault`, `triage-ai` |

### 2. Tauri v2 IPC & Streaming Event Contracts (`apps/desktop`)

| IPC Command / Channel | Direction | Payload Schema | Performance / Security Invariant |
|---|---|---|---|
| `cmd_list_accounts` / `cmd_add_account` | UI $\rightarrow$ Rust | `AccountConfigDto` (supports App Password, OAuth2 Public PKCE, BYO-OAuth credentials, Proton Bridge; never returns raw secrets to UI) | Secrets stored immediately in `vault` (`keyring` or Argon2 fallback); UI only sees opaque `account_id` |
| `cmd_search_messages` | UI $\rightarrow$ Rust | `SearchQueryDto { query, mode: Lexical \| Semantic \| Hybrid, limit, offset }` $\rightarrow$ `Vec<MessageRowDto>` | Target execution `< 15ms` via SQLite `FTS5` + `sqlite-vec` (`int8` quantized + pre-filtered) |
| `cmd_triage_action` | UI $\rightarrow$ Rust | `TriageMutationDto { message_ids, action: MarkRead \| Archive \| Trash \| Tag(String) \| OneClickUnsubscribe }` | Optimistic local SQLite update + queued async IMAP `STORE FLAGS` sync (or RFC 8058 HTTP POST on explicit unsubscribe) |
| `cmd_get_message_body` | UI $\rightarrow$ Rust | `MessageId` $\rightarrow$ `MessageDetailDto { headers, text_plain, html_raw, triage_badges, attachments_meta }` | HTML sanitized in UI with `DOMPurify` + `<iframe sandbox="">` + normalized base dark-mode CSS |
| `cmd_fetch_attachment` | UI $\rightarrow$ Rust | `AttachmentRequestDto { account_id, folder, uid, part_id }` $\rightarrow$ `AttachmentBytesDto` | **Lazy Attachment Invariant:** Fetched on-demand via `IMAP BODY.PEEK[section]` |
| `Channel<SyncProgressEvent>` | Rust $\rightarrow$ UI | `SyncProgressEvent { account_id, state, new_messages_count, triage_alerts }` | Non-blocking streaming updates from `tokio` IMAP `IDLE` workers |

### 3. Extensibility Trait Boundary

To keep the core 100% free and open-source (AGPLv3) with zero artificial limits while supporting optional post-triage hooks:
- `TriageEventHook` / `SyncEventSink` trait: Clean observer boundary invoked after message ingestion and triage extraction (used by the desktop UI notification channel and extensible for future local/webhook automations without premature cloud-sync abstractions).

---

## Gotchas & Pitfalls

- **Transparent Multi-Mode Authentication (OAuth2 vs App Passwords):** Providers like Gmail require a `client_secret` and paid CASA verification for public desktop OAuth2 apps, whereas Microsoft supports public PKCE (`client_id` only). `crates/mail-engine` MUST transparently support all modes for the user: App Passwords, Public OAuth2 PKCE, and user-provided BYO OAuth (`client_id` + optional `client_secret` stored in `crates/vault`), while ensuring zero credentials are ever hardcoded or committed to git.
- **Proton Mail Bridge (Localhost IMAP):** Proton Bridge runs on `127.0.0.1:1143` using STARTTLS or a self-signed local TLS certificate. `crates/mail-engine` MUST only allow custom root/self-signed certificate pinning when the IMAP host is strictly loopback (`127.0.0.1` / `::1`), never for remote hosts.
- **IMAP Provider Quirks & Zero-Copy MIME (`pimalaya` & Stalwart `mail-parser`):** Normalize provider-specific IMAP differences (Gmail `[Gmail]/All Mail` deduplication & `XOAUTH2`, Outlook `CONDSTORE` fallbacks, Proton Bridge `IDLE` behavior) behind `ImapConnector` following `pimalaya/email-lib` patterns, and parse MIME structures zero-copy using Stalwart Labs' `mail-parser` (see `.agent/knowledge/k-002-imap-provider-quirks-and-mime-parsing.md`).
- **SQLite FTS5 + SQLCipher + `sqlite-vec` Amalgamation:** Use full-database SQLCipher encryption (never encrypt text columns individually, as that breaks `FTS5`). Default `FTS5` tokenizer to `unicode61 remove_diacritics 2` (configurable by user) and register `sqlite-vec` via `sqlite3_auto_extension` inside an isolated `#[allow(unsafe_code)]` block in `crates/storage` (`#![deny(unsafe_code)]`).
- **ONNX Model Weights on Demand:** Do not bundle multi-megabyte ONNX model files inside the compiled binary; download/cache quantized models lazily in the user data directory (`$XDG_DATA_HOME/triage-mail/models`).
- **Git History Scrubbing (Private $\rightarrow$ Public Transition):** Every test fixture for MIME parsing, 2FA/Boleto/NF-e extraction, and IMAP responses MUST use synthetic RFC 2606 domains (`user@example.com`, `billing@example.org`) and synthetic tokens.

---

## Deliberate Technical Debt

| Debt | Rationale | Revisit When |
|---|---|---|
| Outbound SMTP / Email Composer omitted | MVP v0.1 is strictly "Triage-First" (Inbound reading, search, and triage) | Post-v0.1 (`v0.2+`) after inbound sync and hybrid search reach stability |

## Desktop IPC Implementation Details ([04.1])

- The six established commands retain snake_case DTO fields and externally tagged Rust enums.
  Tauri invoke argument names use its default camelCase (`messageId` for message detail).
  `cmd_add_account` takes separate `config: AccountConfigDto` and input-only `credentials`
  (`AccountAuthMaterial` wire representation), and returns only an opaque account ID string.
  An empty input ID requests a generated opaque ID; nonempty IDs cannot contain email addresses.
  `cmd_list_accounts` returns safe `AccountConfigDto` metadata without consulting credential values.
- `MessageRowDto` aliases `storage::MessageRowSummary`. `MessageDetailDto` preserves the scaffold's
  `id`, `subject`, `sender`, and `triage_result`, and adds contracted `headers` (raw header string)
  and `triage_badges`. `AttachmentBytesDto` is `{ bytes: Vec<u8> }`.
- `cmd_subscribe_sync(onProgress: Channel<SyncProgressEvent>)` registers one subscription per local
  webview. Replacement/disconnection does not stop mail ingestion. Lag emits `ResyncRequired` so
  consumers refetch authoritative data. Other states are `Syncing`, `Idle`, `Retrying`,
  `MutationPending`, and `MutationFailed`. Alerts are `Vec<TriageBadge>` with bounded previews.
- SQLCipher migration v4 adds `account_connections`, `pending_mutations`, and
  `message_folder_overrides` without altering existing tables. Endpoint JSON is serialized only
  from the secret-free configuration DTO. Remote mutation payloads contain message identity and
  action only; local updates and outbox writes commit together. Pending read actions survive
  background flag reconciliation. Remote commands check UIDVALIDITY before using a queued UID.
- Runtime initialization provisions the SQLCipher key through `SecretStore`; a missing key for
  an existing database fails closed. `cmd_unlock_vault(passphrase)` explicitly enables the
  Argon2id/AES-GCM fallback when the OS keyring is unavailable. Passphrases and incoming auth
  buffers are zeroized and never appear in responses, events, errors, or configuration JSON.
- Headless `mail_engine::mutations` exports UIDVALIDITY-checked STORE/MOVE and lazy attachment
  helpers with pure Rust request/outcome types. Desktop DTOs adapt to these helpers; existing
  `ImapConnector`, `SyncEventSink`, `TriageEventHook`, and repository trait signatures are unchanged.
- Workers use incremental sync, IDLE with heartbeat renewal, and adaptive polling fallback.
  Network operations never hold the local SQLite mutation lock. Native transport is injectable
  for offline service tests; the default `native-runtime` feature runs the actual Tauri app.
- Archive/trash discover server special-use folders and require IMAP MOVE; unsupported servers
  retain their queued action and emit `MutationFailed`. There is no unsafe blanket-expunge fallback.
  OAuth account registration accepts the existing vault auth material, including acquired tokens;
  a provider-specific authorization UI remains separate. Embeddings use the existing provider
  implementation and cached model configuration; IPC never downloads models or calls an LLM.
