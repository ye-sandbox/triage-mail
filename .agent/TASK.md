# TASK.md — Current Task and Roadmap

> WHAT to do now. Detailed history lives in `git log` and `.agent/ARCHIVE.md`.
> User requests during conversation take precedence — report discrepancies before acting.

---

## Active Task

### Task [04.21]: Account Subscription Badges & Automatic IMAP Quota Detection

- **Status:** DONE
- **Priority:** 1 (User-selected — 2026-11-17)
- **Systems Involved:** `crates/mail-engine`, `crates/storage`, `apps/desktop/src-tauri`, `apps/desktop/src/lib/onboarding`, `apps/desktop/src/lib/shell`, `apps/desktop/src/lib/stores`, `apps/desktop/src/lib/i18n`.
- **Problem:** Users managing multiple email accounts (especially multiple Google/Gmail accounts with different plans such as Free 15 GB, Google One 100/200 GB, Google AI Pro 2 TB, or Google AI Ultra 5 TB/30 TB+) have no visual indicator or storage quota telemetry in the sidebar or account list to distinguish subscribed accounts from free accounts. Inferring account-level subscriptions from email bodies risks false positives from marketing/forwarded emails, whereas IMAP `GETQUOTAROOT` (RFC 2087) provides an authoritative, zero-false-positive automatic signal directly from the server, complemented by a manual user badge override.
- **Proposed Scope & Atomic Subtasks:**
  1. `[04.21.1]` `[BATCH: B1]` **RFC 2087 IMAP `QUOTA` Parser, Automatic Tier Inference & Storage Persistence (`crates/mail-engine`, `crates/storage`):**
     - Extend `ServerCapabilities` (`caps.quota` from `QUOTA` token) and `ImapSession::fetch_storage_quota(mailbox)` in `crates/mail-engine/src/protocol.rs` to execute `GETQUOTAROOT "INBOX"` and parse untagged `* QUOTA <root> (STORAGE <used_kib> <limit_kib>)` responses into `AccountStorageQuota { used_bytes: u64, limit_bytes: u64 }` (converting RFC 2087 1024-octet KiB blocks with checked arithmetic).
     - Add deterministic `infer_subscription_badge(provider: ProviderKind, host: &str, quota: Option<AccountStorageQuota>) -> Option<AccountPlanBadge>` mapping Google/Gmail (`>= 5 TiB` $\rightarrow$ `Ultra`, `>= 500 GiB` $\rightarrow$ `Pro`, `> 20 GiB` $\rightarrow$ `One`) and generic IMAP quotas without any email-body heuristics.
     - Persist per-account `quota` and manual `badge_override` (`Auto`, `Hidden`, `Preset(AccountPlanBadge)`, `Custom(String)`) additively inside `account_connections.config_json` in `crates/storage/src/desktop.rs` without altering `CURRENT_SCHEMA_VERSION = 11` or breaking existing `AccountConfigDto` / `AccountConnectionConfig` deserialization.
  2. `[04.21.2]` `[BATCH: B2]` **Desktop Worker Quota Refresh & Additive Subscription IPC (`apps/desktop/src-tauri`):**
     - Extend `RemoteMail` with best-effort `fetch_quota(&self, config: &AccountConfigDto) -> Result<Option<AccountStorageQuota>, IpcError>` and update the account sync worker to refresh and persist account storage quota automatically during folder discovery/sync (never failing message sync if a server omits `QUOTA`).
     - Expose additive IPC commands `cmd_account_subscriptions() -> HashMap<String, AccountSubscriptionDto>` and `cmd_set_account_badge(accountId: String, override_mode: AccountBadgeOverrideDto) -> AccountSubscriptionDto` returning `{ account_id, effective_badge, badge_override, inferred_badge, quota }`.
  3. `[04.21.3]` `[BATCH: B3]` **Sidebar Subscription Pills, Quota Bar & Manual Badge Override UI (`apps/desktop/src`):**
     - Render compact, color-coded subscription badges (`ULTRA`, `PRO`, `ONE`, `WORKSPACE`, `PLUS`, or custom label) with storage quota tooltip next to accounts in `SidebarNavAccounts.svelte` and `WdgConnectedAccountList.svelte`.
     - Add a discrete storage usage bar (`used / limit`) and inline **"Plan / Badge"** selector (`Auto (IMAP Quota)`, `ULTRA`, `PRO`, `ONE`, `WORKSPACE`, `PLUS`, `Custom`, `Hidden`) on each account card in `WdgConnectedAccountList.svelte` with full EN/PT-BR localization and unit tests.

#### Completion (2026-11-17)

- **Delivered:** RFC 2087 `GETQUOTAROOT` parsing and Google tier inference (`crates/mail-engine::quota`), additive quota/override persistence in `config_json` (`storage::desktop::AccountConnectionExtra`), throttled best-effort quota refresh during folder discovery, `cmd_account_subscriptions` / `cmd_set_account_badge`, sidebar and account-list subscription pills, quota bar and Plan / Badge selector (EN/PT-BR).
- **Validation:** `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test --workspace` all exit 0; `pnpm --filter desktop check` 0 errors/warnings; `pnpm --filter desktop test` 31 files / 191 tests; `pnpm --filter desktop build` succeeded; `export let` invariant, `ci-guard` (464 files, 0 findings) and `git diff --check` clean. Prettier is not installed locally, so it was not run.
- **Contracts:** `.agent/NOTES.md` (`Account Subscription Badges & IMAP Quota Detection ([04.21])`).
- **Next:** No prioritized active task; awaiting user selection.

---

## Latest Completed Task

### Task [02.9]: Actionable Connection, Credential & OAuth Error Diagnostics

- **Status:** DONE
- **Priority:** 3 (Email-Access Improvement Initiative — 2026-11-16)
- **Systems Involved:** `crates/mail-engine`, `apps/desktop/src-tauri`, `apps/desktop/src/lib/onboarding`, `apps/desktop/src/lib/inbox`, `apps/desktop/src/lib/i18n`.
- **Problem:** Authentication failures, IMAP-disabled accounts, OAuth consent errors (`access_denied`, missing Test User, `redirect_uri_mismatch`), and expired/revoked refresh tokens currently collapsed into opaque `IpcError::RemoteUnavailable` or silent `SyncState::Retrying` loops without actionable user guidance or a one-click re-authorize action.
- **Scope & Atomic Subtasks:**
  1. `[02.9.1]` `[BATCH: B1]` **Secret-Free Diagnostic Codes, Pre-Save Login Test & OAuth Re-Auth IPC (`crates/mail-engine`, `apps/desktop/src-tauri`):** Added closed `ConnectionDiagnosticCode` enum (`TlsHandshakeFailed`, `AuthenticationRejected`, `ImapDisabledHint`, `OAuthAccessDenied`, `OAuthInvalidClient`, `OAuthCancelled`, `NetworkTimeout`), `EngineError::diagnostic_code()` / `classify_imap_auth_failure`, RFC 6749 `error` allowlist parsing in `HttpTokenRefreshTransport` & `LoopbackRedirectListener`, bounded `DefaultImapConnector::test_connection_as`, matching fixed unit `IpcError` variants, `cmd_test_account_connection` (zeroized transient check, zero persistence), `cmd_reauthorize_account` (in-place vault token update + `WorkerSupervisor::restart_account` preserving indexed mail), `SyncProgressEvent.diagnostic`, and `cmd_account_diagnostics`.
  2. `[02.9.2]` `[BATCH: B2]` **Actionable Remediation UI & Re-Authorize Button (`apps/desktop/src/lib/onboarding`):** Added `ConnectionDiagnosticCode` & `SyncProgressEvent.diagnostic` TypeScript types, `AccountTransport` wire bindings (`testConnection`, `reauthorize`, `diagnostics`), bilingual EN/PT-BR recovery hints for all 7 diagnostic codes, pre-save **"Test login"** flow in `WdgAccountDiscoveryForm` with immediate unbound `[data-credential]` DOM clearing, per-account diagnostic banners, and inline **"Reauthorize"** OAuth flow on existing OAuth2 account cards in `WdgConnectedAccountList` / `WdgOnboardingPage`.

#### Completion (2026-11-16)

- **Delivered:** End-to-end secret-free connection, credential, and OAuth error classification across `crates/mail-engine` and `apps/desktop/src-tauri`, bounded pre-save **"Test login"** (`cmd_test_account_connection`), in-place OAuth2 **"Reauthorize"** (`cmd_reauthorize_account`) preserving all indexed local mail/folders/watermarks/tags/rules, per-account retry diagnostic propagation (`SyncProgressEvent.diagnostic` + `cmd_account_diagnostics`), and localized EN/PT-BR onboarding remediation UI.
- **Validation:** `cargo fmt --all -- --check` passed (0); `cargo check --workspace` passed (0); `cargo clippy --workspace --all-targets -- -D warnings` passed (0 warnings); `cargo test --workspace` passed across all workspace crates (`mail-engine`, `storage`, `sync-runtime`, `triage-ai`, `triage-mail-cli`, `triage-mail-desktop` 46 unit + 33 `ipc_service` tests, `vault` 20 tests); `pnpm --filter desktop check` passed (0 errors/warnings); `pnpm --filter desktop test` passed (30 files, 185 tests); `pnpm --filter desktop build` succeeded; `! grep -rn "export let" apps/desktop/src/` exited 0; `node scripts/ci-guard.mjs` (`464 tracked text files, 0 findings`) & `node --test scripts/ci-guard.test.mjs` (`5 passed`); `git diff --check` clean.
- **Contracts:** Documented in `.agent/NOTES.md` (`Actionable Connection, Credential & OAuth Diagnostics ([02.9])`).
- **Next:** No prioritized active task. All three Email-Access Improvement Initiative tasks (`[02.8]`, `[04.20]`, `[02.9]`) are complete; awaiting user selection for the next milestone.

#### Closing Checklist — [02.9]

- [x] `[02.9.1]` Secret-free diagnostic codes, pre-save login test (`cmd_test_account_connection`), `cmd_reauthorize_account`, and `SyncProgressEvent.diagnostic` / `cmd_account_diagnostics` implemented and tested.
- [x] `[02.9.2]` Localized EN/PT-BR remediation guidance, pre-save **"Test login"** UI, and in-place **"Reauthorize"** flow on OAuth2 accounts implemented and tested.
- [x] Full workspace DoD passed: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `pnpm check/test/build`, Svelte 5 invariant, `ci-guard` PII/secret scan, `git diff --check`.
- [x] Contracts recorded in `.agent/NOTES.md`; `.agent/TASK.md` updated cleanly.
- [x] Atomic Conventional Commit recorded; closed without pushing.

---

## Previous Completed Tasks

### Task [04.20]: Streamlined First-Run Vault & Onboarding UX

- **Status:** DONE
- **Priority:** 2 (Email-Access Improvement Initiative — 2026-11-16)
- **Systems Involved:** `apps/desktop/src-tauri`, `apps/desktop/src/lib/onboarding`, `apps/desktop/src/lib/i18n`.
- **Problem:**
  1. First-run fallback vault shows `"Unlock your mail / Vault passphrase"` without indicating the user is *creating* a new master passphrase when no `mail.db`/`vault.enc` exists yet (especially on Linux sessions without D-Bus Secret Service).
  2. Offline provider discovery only runs on email `onblur`, and existing backend presets (`Zoho`, `UOL`, `BOL`, `Terra`) are missing from quick-fill UI presets.
  3. Technical fields (`IMAP host`, `Port`, OAuth endpoints, `Scopes`) are always expanded even when resolved from a preset/ISPDB (unused `advancedSettings` i18n key), and raw protocol tabs confront non-technical users upfront.
- **Scope & Atomic Subtasks:**
  1. `[04.20.1]` `[PARALLEL: W1]` **First-Run Vault State IPC (`apps/desktop/src-tauri`):** Implemented read-only `cmd_vault_status` returning `{ exists: bool, keyring_available: bool }` in `dto.rs`/`runtime.rs` checking `mail.db`/`vault.enc` via `symlink_metadata` without creating directories, acquiring locks, or exposing paths.
  2. `[04.20.2]` `[PARALLEL: W1]` **Debounced Input Autodiscovery & Preset Parity (`apps/desktop/src/lib/onboarding`):** Added `hasValidEmailDomain` + 250ms debounced offline autodiscovery in `DiscoveryController.input(email?)` (preserving stale revision cancellation and `manual` overrides) and exposed `Zoho`, `UOL`, `BOL`, `Terra`, and `Other IMAP` presets with EN/PT-BR setup playbooks.
  3. `[04.20.3]` `[BATCH: B2]` **Create-vs-Unlock Vault Card (`WdgVaultUnlockCard.svelte`):** Wired `OnboardingSession` to `cmd_vault_status` and updated `WdgVaultUnlockCard` to present **"Create local master passphrase"** (confirmation `>= 8` chars + keyring-unavailable Argon2id notice) on first run vs. **"Unlock vault"** when `vault.enc`/`mail.db` already exist, retaining unbound DOM credential clearing.
  4. `[04.20.4]` `[BATCH: B3]` **Provider-Guided Flow & Collapsible Advanced Settings (`WdgAccountDiscoveryForm.svelte`):** Wired provider-tailored human-readable options (*App Password*, *Google Cloud OAuth JSON*, *Offline `.mbox`/`.eml` Import*, *Proton Mail Bridge*, *Custom IMAP*) and collapsible `advancedSettings` for `host`, `port`, `authorize`, `token`, and `scopes` with auto-expansion on `Other IMAP` / `unresolved` while retaining unbound `[data-credential]` DOM inputs.

#### Completion (2026-11-16)

- **Delivered:** End-to-end streamlined first-run vault creation UX (`cmd_vault_status` + `WdgVaultUnlockCard` confirmation/keyring-unavailable copy), debounced email `oninput` offline autodiscovery with regional/custom presets (`Zoho`, `UOL`, `BOL`, `Terra`, `Other IMAP`), and provider-guided collapsible `advancedSettings` onboarding form (`WdgAccountDiscoveryForm`) with EN/PT-BR translations.
- **Validation:** `cargo fmt --all -- --check` passed (0); `cargo check --workspace` passed (0); `cargo clippy --workspace --all-targets -- -D warnings` passed (0 warnings); `cargo test --workspace` passed across all crates (including 44 unit + 31 integration tests in `triage-mail-desktop`); `pnpm --filter desktop check` passed (0 errors/warnings); `pnpm --filter desktop test` passed (30 files, 181 tests); `pnpm --filter desktop build` succeeded; `! grep -rn "export let" apps/desktop/src/` exited 0; PII/secret scan clean; `git diff --check` clean.
- **Contracts:** Documented in `.agent/NOTES.md` (`Streamlined First-Run Vault & Onboarding UX ([04.20])`).
- **Next:** Promoted `[02.9]` (**Actionable Connection, Credential & OAuth Error Diagnostics**) to Active Task (`READY FOR PLANNING`), awaiting user approval before starting `[02.9.1]` & `[02.9.2]`.

#### Closing Checklist — [04.20]

- [x] `[04.20.1]` First-run vault state IPC (`cmd_vault_status`) implemented and tested.
- [x] `[04.20.2]` Debounced `oninput` offline autodiscovery and `Zoho`/`UOL`/`BOL`/`Terra`/`Other IMAP` presets implemented and tested.
- [x] `[04.20.3]` Create-vs-Unlock `WdgVaultUnlockCard` UI with confirmation & keyring-unavailable notice implemented and tested.
- [x] `[04.20.4]` Provider-guided flow & collapsible `advancedSettings` implemented and tested.
- [x] Full workspace DoD passed: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `pnpm check/test/build`, Svelte 5 invariant, PII/secret scan, `git diff --check`.
- [x] Contracts recorded in `.agent/NOTES.md`; `[02.9]` promoted to Active Task (`READY FOR PLANNING`).
- [x] Atomic Conventional Commit recorded; closed without pushing.

---

### Task [02.8]: Offline `.mbox` & `.eml` Local Archive Ingestion

- **Status:** DONE
- **Priority:** 1 (Email-Access Improvement Initiative — 2026-11-16)
- **Systems Involved:** `crates/mail-engine`, `crates/vault`, `crates/storage`, `crates/sync-runtime`, `apps/desktop/src-tauri`, `apps/desktop/src`.
- **Problem:** Users who cannot or prefer not to configure 2FA App Passwords or Google Cloud BYO OAuth (e.g., due to CASA verification limits or corporate restrictions) had no offline zero-auth way to import Google Takeout `.mbox` exports, Thunderbird mbox files, or standalone `.eml` archives into `triage-mail`.
- **Scope & Atomic Subtasks:**
  1. `[02.8.1]` `[BATCH: B1]` **Streaming `.mbox` & `.eml` Parser (`crates/mail-engine`):** Implemented bounded streaming `MboxReader` (`From ` delimiter splitting, `>From ` `mboxo`/`mboxrd` unescaping, CRLF/LF normalization, 32 MiB per-message cap, `X-Gmail-Labels` extraction) and `parse_eml_reader` / `parse_eml_bytes` delegating to `StalwartMimeParser` in `#![forbid(unsafe_code)]` Rust with RFC 2606 `@example.com`/`@example.org` test coverage.
  2. `[02.8.2]` `[BATCH: B2]` **Local Archive Account Mode & Offline Ingestion Contract (`crates/vault`, `crates/mail-engine`, `crates/storage`, `crates/sync-runtime`):** Added `AuthMethod::LocalArchive` and `AccountAuthMaterial::LocalArchive` (`imap_host = ""`, `imap_port = 0`), fail-closed guards in `DefaultImapConnector` / `mutations`, `sync_runtime::ingestion::ingest_archive_stream` with monotonic UIDs and `server_uidnext = 0`, outbox skip in `storage::desktop::apply_local_triage_mutation`, and `IpcError::Unsupported` on `fetch_attachment`.
  3. `[02.8.3]` `[BATCH: B3]` **Desktop IPC & Import UI (`apps/desktop/src-tauri`, `apps/desktop/src`):** Added `cmd_import_local_archive` via `apps/desktop/src-tauri/src/archive.rs` with native file picker integration, conflict guard (`AccountExists`), `WorkerSupervisor` skip, streaming `SyncProgressEvent`s, and bilingual EN/PT-BR Svelte 5 UI controls in Onboarding (`WdgAccountDiscoveryForm`, `WdgAccountHelpGuide`, `WdgConnectedAccountList`), Settings (`WdgBackupDefinitionsPanel`), and Reader (`WdgMessageReader`).

#### Completion (2026-11-16)

- **Delivered:** End-to-end offline `.mbox` and `.eml` archive ingestion from `crates/mail-engine` through `crates/sync-runtime`, `crates/storage`, `apps/desktop/src-tauri`, and Svelte 5 onboarding/settings/reader UI while preserving all existing Gmail BYO-OAuth onboarding flows.
- **Validation:** `cargo fmt --all -- --check` passed (0); `cargo check --workspace` passed (0); `cargo clippy --workspace --all-targets -- -D warnings` passed (0 warnings); `cargo test --workspace` passed across all workspace crates (`mail-engine`, `storage`, `sync-runtime`, `triage-ai`, `triage-mail-cli`, `triage-mail-desktop`, `vault`); `pnpm --filter desktop check` passed (0 errors/warnings); `pnpm --filter desktop test` passed (30 files, 177 tests); `pnpm --filter desktop build` succeeded; `! grep -rn "export let" apps/desktop/src/` exited 0; PII/secret domain scan clean; `git diff --check` clean.
- **Contracts:** Documented in `.agent/NOTES.md` (`Offline Local Archive Ingestion ([02.8])`).
- **Next:** Promoted `[04.20]` (**Streamlined First-Run Vault & Onboarding UX**) to Active Task (`PLANNING`), awaiting user approval before starting `[04.20.1]` & `[04.20.2]`.

#### Closing Checklist — [02.8]

- [x] `[02.8.1]` Streaming `.mbox` & `.eml` parser (`crates/mail-engine`) implemented and tested.
- [x] `[02.8.2]` `LocalArchive` account mode & offline ingestion contract (`vault`, `mail-engine`, `storage`, `sync-runtime`) implemented and tested.
- [x] `[02.8.3]` Desktop IPC (`cmd_import_local_archive`) & Svelte 5 UI (Onboarding, Settings, Reader, EN/PT-BR) implemented and tested.
- [x] Full workspace DoD passed: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `pnpm check/test/build`, Svelte 5 invariant, PII/secret check, `git diff --check`.
- [x] Contracts recorded in `.agent/NOTES.md`; `[04.20]` promoted to Active Task (`PLANNING`).
- [x] Atomic Conventional Commit recorded; closed without pushing.

---

### Task [00.13]: Fix Release Workflow releaseName and sync-runtime Windows Stable Compatibility

- **Status:** DONE
- **Selection:** Selected following GitHub Actions release v0.1.5 workflow failure where `tauri-action` lacked `releaseName` and `sync-runtime` failed on Windows due to nightly `#![feature(windows_by_handle)]`.
- **Systems Involved:** `.github/workflows/release.yml`, `crates/sync-runtime/src/lib.rs`, `crates/sync-runtime/src/ownership.rs`, `docs/adr/decisions.md`, `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
- **Problem:**
  1. `release.yml` run `42502102559` failed on Ubuntu/macOS with `"releaseName" not set but required to create release. ##[error]Release not found or created.` because `releaseName` was missing in `tauri-apps/tauri-action`.
  2. `release.yml` run `42502102559` failed on Windows with `error[E0554]: #![feature] may not be used on the stable release channel` because `crates/sync-runtime/src/lib.rs` configured `#![cfg_attr(windows, feature(windows_by_handle))]` and `ownership.rs` used unstable methods (`volume_serial_number`, `file_index`, `number_of_links`) which are forbidden on stable Rust.
- **Scope:**
  1. Remove `#![cfg_attr(windows, feature(windows_by_handle))]` from `crates/sync-runtime/src/lib.rs`.
  2. Update `valid` and `open_lock` in `crates/sync-runtime/src/ownership.rs` under `#[cfg(windows)]` to use standard library stable metadata attributes (`meta.is_file()`, `!meta.file_type().is_symlink()`, `(meta.file_attributes() & 0x400) == 0`, `meta.len() == 0`).
  3. Add `releaseName: 'triage-mail ${{ github.event.inputs.tag || github.ref_name }}'` to `tauri-apps/tauri-action` in `.github/workflows/release.yml`.
  4. Update `docs/adr/decisions.md` (ADR-025) and `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
  5. Verify full workspace DoD (`ci-guard` policy check, `pnpm check`, `pnpm test`, `pnpm build`, `cargo test -p sync-runtime`, `git diff --check`).

#### Approved Implementation Plan (2026-11-08)

1. `[00.13.1]` **Remove Nightly Feature Flags & Use Stable Metadata:**
   Remove `#![cfg_attr(windows, feature(windows_by_handle))]` from `crates/sync-runtime/src/lib.rs`. In `crates/sync-runtime/src/ownership.rs`, adjust `valid` and `open_lock` under `#[cfg(windows)]` to only query standard library `MetadataExt` attributes compatible with the Rust stable release channel.
2. `[00.13.2]` **Fix Tauri Action Release Name:**
   In `.github/workflows/release.yml`, pass `releaseName: 'triage-mail ${{ github.event.inputs.tag || github.ref_name }}'` into `tauri-apps/tauri-action`.
3. `[00.13.3]` **Document Stable Contract:**
   Update ADR-025 in `docs/adr/decisions.md` and `.agent/knowledge/k-047-windows-installation-writer-ownership.md` reflecting the stable safe Rust metadata verification contract.
4. `[00.13.4]` **Full Workspace DoD Validation:**
   Run full checks (`cargo test -p sync-runtime`, `pnpm check`, `pnpm test`, `pnpm build`, `ci-guard` policy scan, `git diff --check`).

#### Completion (2026-11-08)

- **Delivered:** Removed unstable `#![feature(windows_by_handle)]` and nightly method calls from `crates/sync-runtime`, ensuring 100% stable Rust compilation across platforms including Windows runners. Added `releaseName` to `tauri-apps/tauri-action` in `.github/workflows/release.yml` resolving release creation failure. Updated Cargo profile debug levels to prevent linker out-of-memory errors on CI runners. Updated ADR-025 and knowledge card k-047.
- **Validation:** `cargo test -p sync-runtime` (35 tests passed); `pnpm --filter desktop check` passed (0 errors/warnings); `pnpm --filter desktop test` (184 tests passed); `pnpm --filter desktop build` succeeded; `ci-guard` policy scan verified 460 tracked text files with 0 findings; `git diff --check` clean.
- **Knowledge:** Updated `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
- **Next:** No active task. Multi-folder sync and outbound SMTP composer remain unprioritized.

#### Closing Checklist — [00.13]

- [x] Implementation plan approved.
- [x] Nightly feature flags removed from `sync-runtime` and stable metadata verification implemented.
- [x] `releaseName` added to `tauri-apps/tauri-action` in `.github/workflows/release.yml`.
- [x] ADR-025 and knowledge card k-047 updated.
- [x] Full workspace DoD passed: cargo test, pnpm check/test/build, ci-guard policy scan, git diff --check.
- [x] Atomic Conventional Commit recorded; closed without pushing.

---

## Latest Completed Task

### Task [00.12]: Windows Installation Writer Ownership & Path Contract

- **Status:** DONE
- **Selection:** Selected following user defect report on Windows v0.1.4 upgrade where `WriterOwnership` fails closed on non-Unix platforms.
- **Systems Involved:** `crates/sync-runtime/src/ownership.rs`, `apps/desktop/src-tauri/src/ownership.rs`, `docs/adr/decisions.md`, `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
- **Problem:** ADR-025 and Task [00.10] implemented `WriterOwnership` exclusively for `#[cfg(unix)]`, leaving `#[cfg(not(unix))]` to unconditionally fail closed (`Err(OwnershipError::Unavailable)`). When Windows desktop clients launch after upgrading to v0.1.4, `cmd_list_accounts` attempts initialization, encounters `StorageUnavailable`, and reports `"Local mail could not be opened. Try again."`.
- **Scope:**
  1. Implement Windows non-blocking exclusive file locking and path canonicalization in `crates/sync-runtime/src/ownership.rs` under `#[cfg(windows)] mod windows`.
  2. Implement `valid` metadata checks for Windows (rejecting symlinks, non-empty files, and multiple hard links using `std::os::windows::fs::MetadataExt`).
  3. Validate handle identity using `volume_serial_number()` and `file_index()` to guard against spoofing/races.
  4. Update `WriterOwnership::matches_installation` and `installation` struct field for `windows`.
  5. Update ADR-025 in `docs/adr/decisions.md` to document the Windows contract.
  6. Create `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
  7. Verify full workspace DoD (`cargo check`, `cargo test`, `cargo fmt`, `cargo clippy`, `pnpm check/test`).

#### Approved Implementation Plan (2026-11-08)

1. `[00.12.1]` **Windows Module in `crates/sync-runtime/src/ownership.rs`:**
   Add `#[cfg(windows)] mod windows` with 100% safe Rust (`#![forbid(unsafe_code)]`). Implement `acquire`, `installation_path`, `validate_path`, `valid` (rejecting symlinks, non-empty files, and multiple links via `MetadataExt`), handle identity validation via `volume_serial_number()` and `file_index()`, and non-blocking locking via `File::try_lock()` mapping `WouldBlock` to `OwnershipError::Busy`.
2. `[00.12.2]` **`WriterOwnership` Struct & Match Contract:**
   Enable `installation: PathBuf` on `#[cfg(any(unix, windows))]` and update `matches_installation` to check `windows::installation_path` on Windows.
3. `[00.12.3]` **Desktop Ownership Tests Compatibility:**
   Update `apps/desktop/src-tauri/src/ownership.rs` test module cfg to `#[cfg(all(test, any(unix, windows)))] mod tests;`.
4. `[00.12.4]` **Architecture Decision Record (ADR-025) & Knowledge Card:**
   Update `docs/adr/decisions.md` to record the Windows contract in ADR-025 and create `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
5. `[00.12.5]` **Full Workspace DoD Validation:**
   Run full checks (`cargo check`, `cargo test`, `cargo fmt`, `cargo clippy`, `pnpm check/test`) to verify zero regressions.

#### Completion (2026-11-08)

- **Delivered:** Windows implementation of `WriterOwnership` in `crates/sync-runtime/src/ownership.rs` under `#[cfg(windows)] mod windows` using 100% safe Rust (`#![forbid(unsafe_code)]`). Configured non-blocking file locking via standard library `File::try_lock()` mapping `WouldBlock` to `OwnershipError::Busy`, path validation and parent canonicalization, metadata verification rejecting symlinks, reparse points, non-empty files, and multiple hard links, and handle identity matching via `volume_serial_number()` and `file_index()`. Extended `WriterOwnership` struct and `matches_installation` to Windows, updated desktop test module cfg to `#[cfg(all(test, any(unix, windows)))]`, documented the Windows contract in ADR-025 (`docs/adr/decisions.md`), and recorded operational details in `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
- **Validation:** Full workspace checks completed: `cargo check --workspace` passed; `cargo fmt --all -- --check` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings; `cargo test -p sync-runtime` (35 tests passed) and `cargo test -p triage-mail-cli` (15 tests passed) passed; `pnpm --filter desktop check` passed with 0 errors/warnings; `pnpm --filter desktop test` passed (32 test files, 184 tests passed); `pnpm --filter desktop build` succeeded; `ci-guard` policy scan reported 459 tracked files with 0 findings; `git diff --check` clean.
- **Knowledge:** Recorded in `.agent/knowledge/k-047-windows-installation-writer-ownership.md`.
- **Next:** No active task. Multi-folder sync and outbound SMTP composer remain unprioritized.

#### Closing Checklist — [00.12]

- [x] Implementation plan approved.
- [x] Windows Module in `crates/sync-runtime/src/ownership.rs` implemented with 100% safe Rust.
- [x] `WriterOwnership` Struct & Match Contract updated for Windows.
- [x] Desktop Ownership Tests Compatibility updated to `#[cfg(all(test, any(unix, windows)))]`.
- [x] Architecture Decision Record (ADR-025) updated and knowledge card k-047 created.
- [x] Full workspace DoD passed: cargo check/clippy/fmt, cargo test, pnpm check/test/build, ci-guard policy scan.
- [x] Atomic Conventional Commit recorded (`3297a19`); closed without pushing.


### Task [00.11]: Writable Headless Container Deployment

- **Status:** DONE
- **Selection:** Selected by user on 2026-11-07 from unprioritized backlog following completion of [00.10] Headless Background Daemon & Periodic Sync Scheduler.
- **Systems Involved:** Container packaging (`packaging/cli/`, `scripts/build-cli-image.sh`), `apps/cli`, `crates/sync-runtime`, offline Docker regression suites, and documentation (`docs/cli.md`).
- **Problem:** Task [00.4] packaged `triage-mail-cli` strictly as an offline, read-only inspection container (`--read-only`, `--network=none`, bind mounting readonly data). Now that [00.9] (`sync`) and [00.10] (`daemon`) provide active headless synchronization with exclusive writer ownership (`WriterOwnership`), users running in containerized/homelab environments (Docker Compose, headless servers, Kubernetes) need a verified pattern to run `sync` and `daemon` in containers with writable volume mounts, non-root user permissions, sibling lock file location compatibility, outbound IMAP TLS egress, and graceful `SIGTERM` drain without compromising security invariants.
- **Scope:**
  1. Define volume layout and ownership contract for writable container deployments (handling the sibling `.triage-mail-writer-<name>.lock` file inside mounted volumes or subpaths).
  2. Verify and document container networking requirements: outbound TLS (port 993 / loopback 1143) using compiled-in `webpki-roots`, with no inbound listening ports or telemetry.
  3. Validate PID 1 signal forwarding (`SIGTERM`) in container for graceful daemon drain and lock release on `docker stop`.
  4. Ensure fallback vault unlock via stdin (`--unlock-stdin`) works deterministically in headless container pipelines.
  5. Add opt-in Docker integration tests in `apps/cli/tests/offline.rs` covering writable container `sync`, `daemon`, lock contention rejection (exit 7), and signal shutdown.
  6. Document writable container usage in `docs/cli.md` (CLI usage, Docker run examples, volume mounts, permission mapping).

#### Approved Implementation Plan (2026-11-07)

1. `[00.11.1]` **Container Volume Layout & Writer Ownership Invariant:**
   Document and formalize volume layout for writable container execution under non-root users (`65532:65532` or `--user $(id -u):$(id -g)`). Ensure `--data-dir /path/to/data` has a writable parent directory for the sibling `.triage-mail-writer-<name>.lock` file (e.g. mounting `/var/lib/triage-mail` with `--data-dir /var/lib/triage-mail/app` or using a dedicated writable data volume), retaining mode `0o600` on the lock.
2. `[00.11.2]` **Container Outbound Network & Root TLS Verification:**
   Validate that `sync` and `daemon` outbound TLS traffic succeeds using compiled-in `webpki_roots::TLS_SERVER_ROOTS` without requiring host certificate mounts. Document loopback bridge requirements for local Proton Mail Bridge.
3. `[00.11.3]` **Signal Propagation & Graceful Drain as PID 1:**
   Verify `triage-mail-cli daemon` running as container PID 1 handles `SIGTERM` (delivered by `docker stop`), finishes in-flight ticks, releases `WriterOwnership`, and exits cleanly with code 0.
4. `[00.11.4]` **Opt-in Docker Integration Test Suite (`apps/cli/tests/offline.rs`):**
   Add offline tests under `#[ignore = "requires local Docker and TRIAGE_CLI_TEST_IMAGE; see docs/cli.md"]`:
   - `container_writable_sync_commits_and_releases_lock`: runs one-shot `sync` with mock IMAP, commits messages to SQLite, and releases lock.
   - `container_daemon_runs_and_drains_on_sigterm`: runs `daemon` in container, verifies structured NDJSON output, and tests clean `SIGTERM` / `docker stop` exit.
   - `container_writable_refuses_when_host_owns_lock`: verifies lock contention exits with code 7 without corrupting database or vault.
5. `[00.11.5]` **Documentation, Knowledge Card & Full DoD:**
   Update `docs/cli.md` with writable container deployment patterns and examples. Create `.agent/knowledge/k-045-writable-headless-container-deployment.md`. Validate full workspace DoD.

#### Completion (2026-11-07)

- **Delivered:** Writable headless container packaging and integration test suite. Configured volume mount and sibling lock layout conventions (`--data-dir /data/app` with writable parent mount), documented outbound TLS via compiled-in `webpki-roots` and Proton Mail Bridge loopback bridge patterns, verified PID 1 `SIGTERM` / `docker stop` graceful drain and lock release, and added 3 new opt-in Docker integration tests in `apps/cli/tests/offline.rs` (`container_writable_refuses_when_host_owns_lock`, `container_daemon_runs_and_drains_on_sigterm`, and `container_writable_sync_commits_and_releases_lock`).
- **Validation:** Full workspace `cargo test --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` reported 0 warnings; `cargo fmt --all -- --check` passed; all 4 opt-in container tests passed in Docker (`container_daemon_runs_and_drains_on_sigterm`, `container_writable_refuses_when_host_owns_lock`, `container_writable_sync_commits_and_releases_lock`, and `container_reads_readonly_encrypted_wal_and_fails_closed`). Frontend checks (`pnpm --filter desktop check`, `pnpm --filter desktop test`, `pnpm --filter desktop build`) succeeded with 0 errors/warnings. Policy scan reported 457 tracked text files with 0 findings.
- **Knowledge:** Recorded in `.agent/knowledge/k-045-writable-headless-container-deployment.md`.
- **Next:** No active task. Multi-folder sync and outbound SMTP composer remain unprioritized.

#### Closing Checklist — [00.11]

- [x] Implementation plan approved.
- [x] Volume layout and sibling lock invariant formalized and tested.
- [x] Container outbound network and compiled-in TLS roots verified.
- [x] PID 1 SIGTERM signal propagation and graceful drain verified with docker stop.
- [x] Opt-in Docker integration test suite added in apps/cli/tests/offline.rs (lock contention, daemon drain, and sync commit).
- [x] Documentation updated in docs/cli.md and knowledge card k-045 created.
- [x] Full workspace DoD passed: cargo test, clippy, fmt, pnpm check/test/build, ci-guard policy scan.
- [x] Atomic Conventional Commit ready; closed without pushing.


---

## Latest Completed Task

### Task [00.10]: Headless Background Daemon & Periodic Sync Scheduler

- **Status:** DONE
- **Selection:** Selected by user on 2026-11-07 following the completion of [00.9] Headless One-Shot Sync Host & CLI Sync Command.
- **Systems Involved:** Pure Rust `crates/sync-runtime`, `apps/cli`, offline regression suites, and architecture documentation (`docs/cli.md`).
- **Problem:** `apps/cli` only supported one-shot synchronization via `triage-mail-cli sync`. In headless server environments (such as systemd services, background daemons, or containers), a persistent background process was needed that continuously coordinates periodic sync passes across configured accounts, respects termination signals (`SIGINT`/`SIGTERM`) with graceful drain, retains exclusive writer ownership, and emits deterministic structured NDJSON events without requiring the Tauri GUI.
- **Scope:**
  1. Implement a headless daemon scheduler in `crates/sync-runtime::daemon` with configurable interval (`interval_secs`), optional account filter, and cancellation token / signal listening.
  2. Wire `daemon` subcommand into `apps/cli` with `--interval-secs` (default 60s, bounds 1..86400), optional `--account`, and optional `--max-iterations` (for deterministic testing/bounded runs).
  3. Wire POSIX signal handling (`tokio::signal::ctrl_c` and unix `SIGTERM`) for graceful drain, allowing in-flight sync passes to finish before releasing `WriterOwnership` and exiting cleanly.
  4. Stream NDJSON lifecycle and tick events to stdout (`daemon_started`, `sync_tick`, `daemon_stopping`, `daemon_stopped`) with zero secret/PII leakage.
  5. Build offline integration tests covering periodic ticks, graceful signal/token shutdown, lock contention refusal, and parameter bounds.
  6. Update CLI documentation in `docs/cli.md` and validate full repository Definition of Done.

#### Approved Implementation Plan (2026-11-07)

1. `[00.10.1]` **Shared Headless Daemon Scheduler (`crates/sync-runtime`):**
   In `crates/sync-runtime`, added module `daemon` with `DaemonConfig` (`interval_secs`, `account_filter`, `max_iterations`), `DaemonEvent` (structured lifecycle and sync events), and `run_daemon` taking storage, transport, config, and a cancellation receiver / `tokio::sync::watch::Receiver<bool>`.
2. `[00.10.2]` **CLI Daemon Subcommand Parsing (`apps/cli`):**
   In `apps/cli::args`, added `Command::Daemon { interval_secs: u64, account: Option<String>, max_iterations: Option<usize> }`, parsing `--interval-secs` (default 60, min 1, max 86400) and optional `--max-iterations`.
3. `[00.10.3]` **Daemon Execution & Graceful Signal Handling (`apps/cli`):**
   In `apps/cli::main`, acquired `WriterOwnership` and bootstrapped installation. Spawned signal listeners (`SIGINT` / `SIGTERM`) triggering graceful shutdown. Streamed NDJSON progress to stdout. Exits cleanly with code 0 on graceful shutdown, or code 7 on writer contention.
4. `[00.10.4]` **Offline Daemon Regressions:**
   In `crates/sync-runtime/tests/daemon.rs` and `apps/cli/tests/offline.rs`, added tests for multi-tick runs, graceful cancellation mid-interval and mid-sync, lock contention refusal, and argument validation.
5. `[00.10.5]` **Documentation & Full DoD Validation:**
   Updated `docs/cli.md` with daemon flags, signal handling, and examples. Validated `cargo check`, `cargo test`, `cargo fmt`, `cargo clippy`, `pnpm check`, `pnpm test`, `pnpm build`, and tracked-file secret scans.

#### Completion (2026-11-07)

- **Commit:** `c410ff2` (`feat(cli): add headless background daemon and periodic sync scheduler`).
- **Delivered:** Pure Rust `run_daemon` in `crates/sync-runtime::daemon` with `DaemonConfig` and `DaemonEvent` DTOs. Added `daemon` subcommand to `apps/cli` with continuous `WriterOwnership` retention, SIGINT/SIGTERM graceful drain, bounded `--interval-secs` and `--max-iterations`, and NDJSON lifecycle streaming to stdout.
- **Validation:** Full workspace `cargo test --workspace` passed (all crates and integration suites); all-target `cargo clippy --workspace --all-targets -- -D warnings` reported 0 warnings; `cargo fmt --all -- --check` passed. Frontend checks (`pnpm --filter desktop check`, `pnpm --filter desktop test`, `pnpm --filter desktop build`) succeeded with 0 errors/warnings. CI policy scan reported 454 tracked text files with 0 findings; git diff clean.
- **Knowledge:** Recorded in `.agent/knowledge/k-044-headless-background-daemon-and-signal-drain.md`.
- **Next:** No active task. Writable container deployment, multi-folder sync, and outbound SMTP composer remain unprioritized.

#### Closing Checklist — [00.10]

- [x] Implementation plan approved.
- [x] DaemonConfig, DaemonEvent, and run_daemon loop implemented in sync-runtime.
- [x] CLI daemon command wired with continuous writer ownership, SIGINT/SIGTERM drain, and NDJSON output.
- [x] Offline daemon and CLI integration tests passing.
- [x] Full repository DoD, SPA build, formatting, and secret/PII checks pass.
- [x] Knowledge card k-044 created and docs/cli.md updated.
- [x] Atomic Conventional Commit ready; closed without pushing.


---

## Latest Completed Task

### Task [00.9]: Headless One-Shot Sync Host & CLI Sync Command

- **Status:** DONE
- **Selection:** Selected by user on 2026-11-07 following the completion of resource hardening [05.1]
  and the shared sync foundation sequence ([00.5] coordinator, [00.6] exclusive writer ownership,
  [00.7] shared installation bootstrap, and [00.8] shared batch ingestion).
- **Systems Involved:** Pure Rust `crates/sync-runtime`, `apps/cli`, `crates/mail-engine`, `crates/storage`,
  offline regression suites, and architecture documentation.
- **Problem:** All foundational sync primitives are shared in `crates/sync-runtime`, but active IMAP
  connection lifecycle, credential lookup, and `SyncHost` composition currently reside exclusively
  in `apps/desktop/src-tauri`. `apps/cli` only provides read-only inspection (`accounts`, `search`, `show`).
  Headless environments (CLI, cron, and headless daemons) cannot execute an inbound sync pass for
  configured accounts without launching the Tauri GUI.
- **Scope:**
  1. Define a pure Rust headless `SyncHost` in `crates/sync-runtime` that resolves account connection
     configs from `storage::SqliteMailStore` (`account_connections`) and credentials from `vault::SecretStore`.
  2. Implement an agnostic IMAP sync host adapter utilizing `mail_engine::sync::execute_incremental_sync`
     and `mail_engine::DefaultImapConnector` behind `#![forbid(unsafe_code)]`.
  3. Add `sync` command to `apps/cli` (`triage-mail-cli --data-dir PATH [--unlock-stdin] sync [--account OPAQUE_ID]`),
     safely acquiring `WriterOwnership` before connecting and committing batches.
  4. Emit bounded JSON execution summary (account IDs, folder names, new messages, duration) to stdout;
     exit with deterministic status codes without leaking passwords, tokens, or PII.
  5. Build offline mock-IMAP regression tests covering successful one-shot sync passes, account filtering,
     lock contention refusal, and credential-missing failure modes.

#### Approved Implementation Plan (2026-11-07)

1. `[00.9.1]` **Shared Headless Account Connection Model:** In `crates/sync-runtime`,
   formalized the headless `AccountConnectionConfig` matching `account_connections`
   (`account_id`, `email_display`, `imap_host`, `imap_port`, `auth_method`).
2. `[00.9.2]` **Headless SyncHost Implementation:** Implemented `HeadlessSyncHost` in `crates/sync-runtime`
   implementing `sync_runtime::SyncHost` with decoupled `SyncTransport` trait and `DefaultSyncTransport`.
3. `[00.9.3]` **CLI Sync Subcommand Wiring:** Added `sync` subcommand to `apps/cli` with optional `--account`,
   enforcing exclusive runtime writer ownership via `sync_runtime::ownership::WriterOwnership::acquire`
   and bootstrap via `sync_runtime::bootstrap::open`.
4. `[00.9.4]` **Output Serialization & Error Handling:** Serialized outcomes to bounded JSON on stdout,
   mapping lock contention to exit code 7 and remote errors to exit code 8.
5. `[00.9.5]` **Offline Regressions & Invariant Tests:** Added integration tests in `crates/sync-runtime/tests/host.rs`
   and `apps/cli/tests/offline.rs` covering multi-account/filtered sync, lock contention, and unknown account refusal.
6. `[00.9.6]` **Full Repository DoD Validation:** Ran full DoD checks (`cargo check`, `cargo test`, `cargo fmt`,
   `cargo clippy`, `pnpm check`, `pnpm test`, `pnpm build`, `ci-guard`).

#### Completion (2026-11-07)

- **Delivered:** Pure Rust `HeadlessSyncHost` in `crates/sync-runtime` with injectable `SyncTransport` and
  production `DefaultSyncTransport`. Added `sync` subcommand to `apps/cli` with exclusive `WriterOwnership`
  enforcement, bounded JSON reporting, and deterministic exit codes.
- **Validation:** Workspace `cargo test --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings`
  reported 0 warnings; `cargo fmt --all -- --check` passed. Frontend checks (`pnpm --filter desktop check`,
  `pnpm --filter desktop test`, `pnpm --filter desktop build`) succeeded. CI policy scan reported 450 tracked
  text files with 0 findings; git diff clean.
- **Knowledge:** Recorded in `.agent/knowledge/k-043-headless-sync-host-and-cli-sync.md`.
- **Next:** No active task. Daemon lifecycle, background scheduler, and writable container deployment require separate plans.

#### Closing Checklist — [00.9]

- [x] Implementation plan approved.
- [x] Headless AccountConnectionConfig and HeadlessSyncHost implemented in sync-runtime.
- [x] CLI sync command wired with exclusive writer ownership and bounded JSON output.
- [x] Offline host and CLI sync integration tests passing.
- [x] Full repository DoD, SPA build, formatting, and secret/PII checks pass.
- [x] Knowledge card k-043 created and docs/cli.md updated.
- [x] Atomic Conventional Commit ready; closed without pushing.

---

## Latest Completed Task

### Task [05.1]: Low-I/O Compilation & Test Suite Resource Hardening

- **Status:** DONE
- **Selection:** Proposed after the host environment resource baseline was established
  on 2026-11-06. Aligns with ecosystem low-I/O governance to prevent rotational disk (HDD)
  I/O queue saturation during agent compilation and automated test runs.
- **Systems Involved:** Workspace Cargo profiles (`Cargo.toml`), frontend test runner
  configuration (`apps/desktop/vite.config.ts`), and offline regression suites.
- **Problem:** Default Rust debug level (`debug = 2`) produces multi-gigabyte DWARF
  intermediate artifacts, while Vitest concurrent workers trigger simultaneous random read
  spikes across mechanical disk heads, resulting in queue saturation (>300 IO/s) and
  avoidable test timeouts.
- **Scope:**
  1. Formalize `[profile.dev]` and `[profile.test]` with `debug = 1` in workspace `Cargo.toml`.
  2. Configure `apps/desktop/vite.config.ts` with bounded concurrency (`pool: 'forks'`,
     `maxConcurrency: 1`, `fileParallelism: false`) to avoid disk contention during UI tests.
  3. Validate that offline test suites, `cargo check`, clippy and `pnpm test` pass with zero
     regression, retaining 100% stack trace precision (`RUST_BACKTRACE=1`).

#### Approved Implementation Plan (2026-11-07)

1. `[05.1.1]` **Cargo workspace compilation profile hardening:** In workspace `Cargo.toml`,
   add explicit `[profile.dev]` and `[profile.test]` configurations with `debug = 1`
   (line-tables only). This emits file names and line numbers required for actionable stack
   traces and crash dumps without dumping multi-gigabyte variable and type DWARF definitions,
   sharply reducing disk write volume and intermediate target directory bloat.
2. `[05.1.2]` **Frontend test concurrency and thread isolation:** In `apps/desktop/vite.config.ts`,
   configure the `test` block with bounded sequential execution parameters:
   `pool: 'forks'`, `maxConcurrency: 1`, and `fileParallelism: false`. This avoids concurrent
   worker read storms across rotational disk blocks while preserving test isolation in jsdom.
3. `[05.1.3]` **Stack trace and test precision verification:** Verify that stack trace
   information remains fully actionable under `debug = 1` by testing backtrace output
   (`RUST_BACKTRACE=1`). Validate that all unit and integration tests across the workspace
   pass without regression under the serialized runner.
4. `[05.1.4]` **Full repository DoD validation:** Run the full repository Definition of Done:
   `cargo check --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm --filter desktop check`,
   `pnpm --filter desktop test`, `pnpm --filter desktop build`, Svelte 5 `export let` invariant,
   and secret/PII scanning. Commit atomically with Conventional Commits
   (`perf(build): configure low-io cargo profiles and vitest concurrency`).

#### Completion (2026-11-07)

- **Commit:** `0f84710` (`perf(build): configure low-io cargo profiles and vitest concurrency`).
- **Delivered:** Workspace `Cargo.toml` configured with `[profile.dev]` and `[profile.test]`
  at `debug = 1` (line tables only), preserving `.debug_line` / `RUST_BACKTRACE=1` precision
  while omitting multi-gigabyte variable and type DWARF debuginfo. Frontend test runner
  in `apps/desktop/vite.config.ts` configured with `pool: 'forks'`, `maxConcurrency: 1`, and
  `fileParallelism: false` to eliminate simultaneous random disk head read thrashing on rotational HDDs.
- **Validation:** `cargo check --workspace`, `cargo test --workspace` (375 tests passed),
  `cargo fmt --all -- --check`, and all-target `cargo clippy --workspace --all-targets -- -D warnings`
  exited 0. Desktop `svelte-check` reported 0 errors and 0 warnings; all 184 frontend unit tests
  passed sequentially across 32 test files in Vitest; SPA production build succeeded.
  CI policy scan reported 449 tracked text files with 0 findings; Svelte 5 runes invariant
  (`export let`) and clean `git diff --check` passed.
- **Knowledge:** Recorded in `.agent/knowledge/k-042-low-io-compilation-and-vitest-concurrency.md`.
- **Next:** No prioritized successor is available. Active headless execution, daemon
  lifecycle, and writable container deployment require separate plans. Nothing pushed.

#### Closing Checklist — [05.1]

- [x] Implementation plan approved.
- [x] Cargo dev/test profiles hardened with debug = 1 retaining line tables.
- [x] Vitest bounded concurrency configured with serial file execution and fork isolation.
- [x] Offline test suites, clippy (-D warnings), check, and frontend tests pass.
- [x] Full repository DoD, SPA build, formatting, and secret/PII checks pass.
- [x] Knowledge card k-042 created.
- [x] Atomic Conventional Commit recorded; task closed without pushing.

---

### Task [00.8]: Shared Local Sync Batch Ingestion

- **Status:** DONE
- **Selection:** Proposed after the user's `proceed` on 2026-11-05. [00.7] remains
  complete. Share local ingestion policy before introducing an active headless host.
- **Systems Involved:** Pure Rust sync-runtime ingestion, desktop sync adapter,
  offline encrypted fixtures and architecture/contracts documentation.
- **Problem:** The shared coordinator still delegates message conversion and local
  persistence to desktop-only code. A future host would otherwise duplicate canonical
  identity, deterministic extraction, baseline/frontier setup and batch commit policy.
- **Scope:** Extract message/flag conversion and local batch persistence into
  sync-runtime. Desktop retains blocking execution, mutation gates, post-commit hooks,
  event publication and all worker/provider composition.

#### Approved Implementation Plan (2026-11-05)

1. Record an additive ingestion boundary in an ADR. Reuse existing cached workspace
   dependencies, including triage-ai and serde_json; add no dependency downloads,
   unsafe code, GUI imports, schema changes or existing trait/signature changes.
   Keep learned inference out of this path; deterministic extraction only.
2. Extract ParsedInboundMessage-to-MessageRecord conversion and flag normalization.
   Preserve canonical JSON identity, attachment metadata-only handling, category and
   unsubscribe extraction, timestamps, defaults and fixed error classifications.
   Make time an explicit injectable input/provider without changing production timing
   semantics. Preserve UID-zero and attachment-size-overflow refusal.
3. Add synchronous local ingestion using borrowed storage access from the retained
   ownership-bound handle in production. Preserve rule baseline then sender-review
   frontier establishment before conversion and the atomic message/flag/checkpoint
   commit, including UIDVALIDITY reset and canonical deduplication. Return committed
   records for host hooks; do not move hooks into the storage transaction or imply
   rollback of separately established baselines/frontiers.
4. Adapt DesktopSyncHost to the shared ingestion operation under its existing mutation
   gate and blocking executor. Preserve triage/rule hooks, reminder scheduling,
   injected indexing, independent rule invalidations and final buffered progress in
   their existing order, including fatal versus best-effort failures. Retain guarded
   resource and cancellation lifetimes; no worker or remote action is added.
5. Add offline conversion and encrypted-fixture regressions for canonical identity,
   lazy attachment metadata, invalid inputs, flag deltas, reset/checkpoint atomicity,
   duplicates and rule/review baseline ordering. Preserve existing desktop encrypted
   adapter/event-order and cancellation regressions. Demonstrate pure Rust dependency
   boundaries without loading inference artifacts or contacting a credential service.
6. Run the full repository DoD, SPA build, applicable formatting and tracked-file
   secret/PII checks with cached offline tooling and the existing native sysroot.
   Preserve active contracts and applicable knowledge; use readme-standard if paired
   README changes become necessary. Commit atomically and log its hash; close only
   [00.8], without pushing or starting another task.

- **Approval:** User approved with `proceed` on 2026-11-05.
- **Approval boundary:** Local ingestion extraction and desktop adaptation only. No
  active CLI/daemon, transport/authentication extraction, account setup, new credential
  format, schema/IPC change, automation scheduling/delivery consent change, writable
  container, live validation, package/model download or publication. Missing cached
  prerequisites or material behavior/dependency changes return to planning.
- **Build exception:** User explicitly approved build-only toolchain, package and
  dependency downloads on 2026-11-05 after the session prerequisite check. Restore
  private temporary tooling/sysroot without changing runtime dependency versions or
  system packages. No provider, AI/model or delivery traffic is authorized. This
  exception supersedes the offline-only prerequisite restrictions above.
- **Open blockers:** None; validation environment restoration is in progress.

#### Completion (2026-11-06)

- **Commit:** `90222ba` (`refactor(sync): share local batch ingestion and arrival frontiers`).
- **Delivered:** Pure Rust `sync_runtime::ingestion::ingest_batch` with deterministic
  triage extraction, case-insensitive flag normalization, arrival frontier establishment
  (rule baseline followed by sender review) prior to message conversion, atomic
  commit of messages, flags, checkpoint and UIDVALIDITY reset deletions, and return of
  canonical deduplicated records for host hooks. Desktop sync adapter adapted to the
  shared ingestion operation while retaining mutation gates, blocking execution, post-commit
  triage/rule hooks, reminders, injected indexing and buffered progress.
- **Validation:** Locked workspace check, test, clippy (`-D warnings`) and format check pass.
  New conversion and encrypted-fixture unit/integration tests cover identity tuples,
  injected clocks, invalid inputs, frontier retention on error, canonical duplicate identity,
  and atomic rollback on checkpoint failure. All 30 desktop IPC integration tests pass.
- **Frontend/perimeter:** Desktop `svelte-check` reports 0 errors and 0 warnings; all 184
  frontend unit tests pass; SPA production build succeeds. CI policy scan reports 446
  tracked text files with 0 findings; Svelte 5 runes invariant (`export let`) and clean
  `git diff --check` pass.
- **Perimeter:** ADR-027, active contracts and k-041 preserve arrival-frontier precedence,
  clock injection semantics and canonical deduplication invariants.
- **Next:** No prioritized successor is available. Active headless execution, daemon
  lifecycle, writable container deployment and low-I/O resource hardening require
  separate plans. Nothing pushed.

#### Closing Checklist — [00.8]

- [x] Implementation plan approved.
- [x] Shared ingestion preserves conversion, baseline/frontier and commit behavior.
- [x] Offline ingestion and desktop hook/event-order regressions pass.
- [x] Full repository DoD, SPA build, formatting and secret/PII checks pass.
- [x] ADR, active contracts and applicable knowledge preserved.
- [x] Atomic Conventional Commit recorded; task closed without pushing.

---

### Task [00.7]: Shared Local Installation Bootstrap

- **Status:** DONE
- **Selection:** Proposed after the user's `proceed` on 2026-11-05. [00.6] remains
  complete. Extract local installation opening before adding an active headless host.
- **Systems Involved:** Pure Rust sync-runtime bootstrap, desktop ownership adapter,
  offline encrypted fixtures and architecture/contracts documentation.
- **Problem:** Desktop ownership::bootstrap still combines installation permissions,
  vault selection, database-key provisioning and storage opening with GUI-host service
  composition. A future headless host must reuse these local safety rules without
  importing desktop code or duplicating credential and recovery behavior.
- **Scope:** Extract the existing local bootstrap and ownership-retaining secret-store
  wrapper into sync-runtime. Desktop retains remote/provider/service composition and
  all worker startup. No executable headless writer or new runtime behavior.

#### Approved Implementation Plan (2026-11-05)

1. Record an ADR for additive GUI-independent bootstrap types in sync-runtime. Use
   existing workspace/cached vault, zeroize and randomness dependencies only, with
   no downloads, unsafe code or Tauri/GTK/WebKit imports. Preserve existing traits,
   schema, IPC and CLI interfaces; a new dependency requirement returns to planning.
2. Extract installation permission setup, current keyring/fallback/recovery-marker
   selection, database-key lookup/provisioning and SQLCipher opening. Require retained
   WriterOwnership before mutations and bind bootstrap to its installation identity.
   Keep passphrases/key buffers zeroized and errors fixed and free of private values.
   Preserve refusal to generate a replacement key when an existing database lacks one.
3. Return local storage/vault handles with explicit ownership lifetime guarantees.
   Move the delegating owned-secret wrapper into the core boundary so detached vault
   work retains the lock. Prevent storage handles exposed by the new API from silently
   dropping ownership while still usable. Preserve runtime retry and cancellation
   ordering, including recovery-gate retention through running blocking work.
4. Adapt desktop bootstrap to compose NativeRemoteMail, optional learned embeddings
   and DesktopService from the shared local installation. Keep desktop cleanup,
   reconciliation, embedding setup, backup restore and worker startup in their existing
   order and host. Do not introduce workers, inference loads or network calls in core
   bootstrap; retain existing injected constructors for synthetic tests.
5. Add offline encrypted-fixture and subprocess coverage for provision/reopen,
   recovery fallback-only selection, invalid passphrase, missing/malformed existing
   database keys, mismatched ownership identity, failed bootstrap/retry and lock
   retention through independently retained storage/vault handles. Preserve desktop
   ownership/recovery regressions and cached CLI committed-WAL reader compatibility.
   Keyring tests use injection rather than a live user credential service.
6. Run the full repository DoD, SPA build, applicable formatting and tracked-file
   secret/PII scans with cached tools and offline dependencies. Preserve active
   contracts and applicable knowledge. Use readme-standard if documented interfaces
   or validation commands require paired README changes. Commit atomically and log
   its hash; close only [00.7], without pushing or starting another task.

- **Approval:** User approved with `proceed` on 2026-11-05.
- **Approval boundary:** Shared local bootstrap and desktop adaptation only. No
  active CLI/daemon, account setup, new credential format, schema/IPC change, worker
  scheduling/delivery consent change, writable container, live provider validation,
  package/model download or publication. Existing build sysroot may be reused offline;
  missing validation prerequisites must be reported without downloading replacements.
- **Open blockers:** None identified for this bounded proposal. Material behavior or
  dependency changes return to planning before implementation.

#### Completion (2026-11-05)

- **Commit:** `549e154` (`refactor(runtime): share ownership-bound local installation bootstrap`).
- **Delivered:** Additive sync-runtime local bootstrap with ownership identity
  validation, existing keyring/fallback/recovery selection and database-key/storage
  opening. Independent storage/vault clones retain ownership after the installation
  envelope or desktop service drops. Private repository access exposes no consuming
  unwrap. Desktop retains service/provider/worker composition and startup ordering;
  existing public injected constructor, IPC, schemas and core traits remain unchanged.
- **Validation:** Locked/offline workspace check/test and all-target Clippy with
  -D warnings pass; cargo fmt passes. Eight new bootstrap tests cover provisioning,
  reopen, malformed/missing/wrong keys without replacement, recovery unlock, failed
  retry, mismatched guard/symlink refusal, canonical parent aliases and independently
  retained resource lifetimes. Real subprocess storage/vault retention passes. Existing
  desktop cancellation/restore and CLI committed-WAL reader regressions pass. Existing
  artifact-dependent ONNX and opt-in Docker checks remain ignored.
- **Frontend/perimeter:** Desktop check has zero errors/warnings; all 184 frontend
  tests and SPA build pass. Five policy-scanner regressions pass; 446 tracked text
  files have zero secret/PII/legacy findings. Added Markdown and knowledge card pass
  cached Prettier; Rust format, legacy-prop and diff checks pass. Normal sync-runtime
  dependencies contain no Tauri/GTK/WebKit. CLI flags, environment variables and
  documented validation commands did not change, so paired README edits were unnecessary.
- **Environment:** Reused the existing private native-build sysroot and cached Rust,
  frontend and formatter tooling offline. No dependency/model/package download, live
  provider/delivery action, new test keyring access or personal credentials were used.
- **Knowledge:** ADR-026, active contracts and k-040 preserve ownership identity,
  independently retained resource/drop ordering, caller-held retry ownership and
  host-owned startup effects. No new executable headless writer or runtime policy.
- **Limits:** Existing cooperative Unix/local-filesystem ownership limits remain;
  native validation is Linux x86_64. Synchronous bootstrap needs host blocking
  execution and retained caller ownership across failures. Injected keyring behavior
  and encrypted fallback tests do not establish live keyring interoperability.
- **Next:** No prioritized successor is available. Active headless execution,
  scheduling/delivery consent, daemon lifecycle and writable container deployment
  require a separate plan. No successor started; nothing pushed.

#### Closing Checklist — [00.7]

- [x] Implementation plan approved.
- [x] Shared local bootstrap preserves vault, storage and ownership behavior.
- [x] Offline bootstrap/lifetime and desktop recovery regressions pass.
- [x] Full repository DoD, SPA build, formatting and secret/PII checks pass.
- [x] ADR, active contracts and applicable knowledge preserved.
- [x] Atomic Conventional Commit recorded; task closed without pushing.

---

### Task [00.6]: Exclusive Runtime Writer Ownership

- **Status:** DONE
- **Selection:** Proposed after the user's `proceed` on 2026-11-05. [00.5] remains
  complete. Establish ownership before introducing an active headless writer.
- **Systems Involved:** Pure Rust runtime ownership helper, desktop bootstrap and
  backup recovery, offline process integration tests and architecture documentation.
- **Problem:** Desktop recovery_gate serializes only one RuntimeState. Independent
  processes can still provision/open the same installation and start duplicate workers.
  SQLite transactions alone do not serialize vault changes or remote side effects.
- **Scope:** Cooperative, nonblocking, process-exclusive ownership for an installation.
  Preserve offline CLI/container readers and existing schemas, traits and IPC shapes.
  No active CLI command, daemon, writable container or delivery behavior change.

#### Approved Implementation Plan (2026-11-05)

1. Record an ADR for an additive GUI-independent ownership guard in sync-runtime.
   Use safe OS-backed advisory file locking through the installed standard library
   if available; no dependency downloads or unsafe code. Verify cached toolchain
   support before implementation; a new dependency or toolchain requirement returns
   to planning. Lock contention fails immediately with a fixed, non-private error.
2. Define one stable private sibling lock location derived from the canonical parent
   and installation name, usable while the installation directory is absent. Reject
   symlink/nonregular lock targets and unsupported paths/platforms conservatively.
   Keep the lock file free of credentials, mail, paths and PID metadata. Never unlink
   it on release or infer ownership from file existence/PIDs; retain the open locked
   handle for the owner's lifetime. Document cooperative/local-filesystem limits and
   avoid claims of protection against hostile filesystem replacement or older apps.
3. Acquire ownership before desktop provisioning, vault mutation, migration or worker
   startup. Retain it through all service/worker and in-flight blocking-operation
   lifetimes, including failed/retried initialization, without introducing an unlock
   window. Map contention to an existing safe IPC error variant. Preserve current
   unlock modes, recovery marker behavior and mutation/hook ordering.
4. Use the same ownership protocol for startup backup restore, before target checks,
   staging and publication. Serialize restore versus initialize across processes
   without creating the target directory merely to lock it. Preserve absent-target
   restore semantics, authenticated staging, isolated fallback vault and cleanup.
   The in-process recovery_gate remains in place with consistent lock ordering.
5. Verify with offline synthetic subprocesses: contention before any provisioning,
   clean and abrupt process-exit release, persistent lock-file reuse, independent
   installations, failed initialization/retry, symlink/invalid-file refusal, and
   restore-versus-initialize exclusion. Confirm cached CLI/WAL reads remain available
   under writer ownership. Exercise actual OS locks, not only injected mocks, and
   check that rejected contenders cannot change database/vault or start remote work.
6. Run the complete repository DoD, SPA build, applicable formatting and tracked-file
   secret/PII scans using cached tooling and offline dependencies. Preserve contracts
   and relevant knowledge. Load readme-standard if documented setup/validation changes
   require paired README updates. Commit atomically, record the hash and close only
   [00.6]; do not push or start a successor.

- **Approval:** User approved with `proceed` on 2026-11-05.
- **Approval boundary:** Local ownership and desktop/recovery integration only. No
  live network tests, runtime downloads, secrets/credential formats, schema/IPC shape changes,
  active headless bootstrap, scheduling/delivery consent, daemon or image publication.
- **Prerequisite:** Cached Rust 1.99.0 supports safe standard-library file locking;
  no Rust dependency download is needed. On 2026-11-05 the user explicitly approved
  a build-only P0 exception to download GTK/WebKit development packages into a
  private temporary sysroot for full native validation. No system installation,
  provider call, live delivery or broader runtime network permission is authorized.

#### Completion (2026-11-05)

- **Commit:** `ab7dd36` (`feat(runtime): enforce exclusive installation writer ownership`).
- **Delivered:** Safe standard-library nonblocking ownership guard in sync-runtime,
  with persistent private sibling lock identity before and after installation
  publication. Desktop acquires before bootstrap and retains ownership through
  failed unlock/retry, workers, blocking storage and detached credential reads.
  Backup restore uses the same guard. Owned recovery-gate handles prevent canceled
  blocking initialization/restore from overlapping same-process retries.
- **Validation:** Locked/offline workspace check/test and all-target Clippy with
  -D warnings pass; cargo fmt passes. All 375 Rust tests pass, with the existing
  artifact-dependent ONNX and opt-in Docker tests ignored. Desktop typecheck has zero
  errors/warnings; all 184 frontend tests and SPA build pass. Five scanner regressions
  pass; 443 tracked text files have zero secret/PII/legacy findings. Added Markdown
  sections and changed standalone documents pass cached Prettier; legacy-prop and
  diff checks pass. Core normal dependencies contain no Tauri/GTK/WebKit.
- **Coverage:** Real subprocess contention, normal/abrupt exit release, persistent
  inode reuse, independent installations, canonical parent aliases, invalid paths,
  symlink/nonregular/nonempty/multilink/private-mode refusal, failed unlock/retry,
  detached vault and canceled blocking-storage lifetimes, restore/bootstrap handoff,
  unchanged rejected-contender database/vault files and actual CLI committed-WAL reads
  while another process holds ownership. Native IPC regression uses a private
  temporary path; no fixture bypass was added to production acquisition.
- **Build exception:** User explicitly approved build-only GTK/WebKit package
  downloads into a private temporary sysroot. No system packages or shared wrappers
  were modified. Session-only real pkg-config, resolved development symlinks and
  dynamic-ONNX probing configuration enabled native validation. No live provider,
  AI or delivery calls, personal mail, credentials or runtime downloads were used.
- **Perimeter:** ADR-025, active contracts, CLI reader documentation and k-039 preserve
  lock identity, cancellation lifetimes and build-tool gotchas. Existing CLI flags,
  .env.example variables and documented validation commands did not change, so paired
  README edits were unnecessary. No Rust dependency, schema or IPC shape changed.
- **Limits:** Cooperative current Unix hosts on local filesystems only; native
  validation is Linux x86_64. Unsupported platforms fail closed. Older binaries,
  direct core consumers, hostile path replacement, bind aliases and network
  filesystems remain outside the guarantee. Writers require parent write access;
  a leftover lock file is normal and must not be deleted while a writer runs.
- **Next:** No prioritized successor is available. Active headless bootstrap,
  scheduling/delivery consent, daemon lifecycle and writable container deployment
  require a separate plan. No successor started; nothing pushed.

#### Closing Checklist — [00.6]

- [x] Implementation plan approved.
- [x] Safe ownership guard protects desktop initialization and backup recovery.
- [x] Offline subprocess contention/release and reader compatibility tests pass.
- [x] Full repository DoD, SPA build, formatting and secret/PII checks pass.
- [x] ADR, active contracts and applicable knowledge preserved.
- [x] Atomic Conventional Commit recorded; task closed without pushing.


---

### Task [00.5]: Shared Headless Sync Orchestration Foundation

- **Status:** DONE
- **Selection:** Proposed after the user's `proceed` on 2026-11-05. [00.4] remains
  complete. This is a prerequisite for the unprioritized active headless runner.
- **Systems Involved:** New pure Rust `crates/sync-runtime`, desktop worker adapter,
  offline synthetic integration tests and architecture/contracts documentation.
- **Problem:** Incremental IMAP primitives are headless, but folder selection, batch
  orchestration and ingestion ordering currently live in the desktop worker and
  depend on DesktopService/IpcError. An active CLI must not duplicate this logic or
  import Tauri bootstrap, which also provisions storage and starts unrelated workers.
- **Scope:** Extract a reusable single-account sync-pass coordinator and use it from
  the existing desktop worker. Preserve observable desktop behavior and leave native
  transport, credentials, persistence and post-commit work behind explicit adapters.
  No runnable daemon, active CLI command or container behavior change in this task.

#### Approved Implementation Plan (2026-11-05)

1. Record the coordinator/host-adapter boundary in an ADR. Add a workspace
   `sync-runtime` crate with `#![forbid(unsafe_code)]`, using cached dependencies
   only and no Tauri, GTK/WebKit or desktop imports. Keep existing core traits,
   schemas, IPC payloads and CLI commands unchanged; introduce additive types only.
2. Extract folder discovery/selection, INBOX ordering, cached-folder fallback,
   checkpoint loading, bounded batch progression, existing 120-second remote timeout
   and UIDVALIDITY reset handling into the coordinator. Define small injectable host
   operations for remote access, checkpoint/catalog reads and batch persistence;
   translate core errors to fixed desktop diagnostics at the adapter boundary.
3. Retain desktop mutation-gate and blocking-storage execution in its adapter.
   Preserve rule baseline and sender-review frontier establishment before ingestion,
   atomic message/flag/checkpoint commits, canonical IDs, deterministic extraction,
   rule eligibility and reminder/index hooks. Deliver committed progress only after
   persistence succeeds. Preserve current best-effort versus fatal error behavior.
   Post-commit hooks remain host-owned and must not be silently omitted or reordered.
4. Route desktop sync_account_once through the coordinator. Keep supervisor lifetime,
   IDLE/adaptive polling, native authentication/refresh, remote mutations, outbox,
   alerts, reminders, local calendar and embeddings in their current host. Preserve
   lazy attachments and existing cancellation semantics. Do not add network calls,
   background workers, installation provisioning or runtime credential formats.
5. Use offline scripted remote/storage/observer adapters to verify folder selection,
   multi-batch progress, UIDVALIDITY reset, invalid frontiers, remote/storage failure,
   timeout/cancellation and commit-before-progress ordering. Exercise the desktop
   adapter with synthetic encrypted storage to preserve canonical deduplication,
   baseline/frontier behavior and hook ordering. Verify GUI-free dependency closure.
6. Run the full repository DoD, SPA build, relevant formatting and tracked-file
   secret/PII scans. Preserve active contracts and applicable knowledge. Update paired
   READMEs under readme-standard only if their documented interface or validation
   commands change. Make an atomic Conventional Commit and log its hash. Close only
   [00.5]; do not push or automatically start an active CLI/daemon successor.

- **Approval:** User approved with `proceed` on 2026-11-05.
- **Approval boundary:** Internal shared sync orchestration and desktop adaptation
  only. No live provider test, dependency download, new network exception, schema/IPC
  change, CLI write access, process ownership protocol or container publication.
  Headless unlock/bootstrap, exclusive runner ownership, scheduling/delivery consent,
  daemon shutdown and writable container deployment require a successor plan.
- **Open blockers:** None for this bounded proposal. Material dependencies or changes
  to the stated behavior/security boundary return to planning.

#### Completion (2026-11-05)

- **Commit:** `010a857` (`refactor(sync): share account sync orchestration through host adapters`).
- **Delivered:** Pure Rust sync-runtime workspace crate with additive account-bound
  SyncHost and single-pass coordinator. Desktop sync uses its host adapter while
  retaining native transport/credentials, gate/blocking policy, ingestion/frontier
  ordering, rules/reminders/index hooks and buffered progress. Existing schemas,
  core traits, IPC and offline CLI/container interfaces remain unchanged.
- **Validation:** Offline locked workspace check/test and all-target Clippy with
  -D warnings pass; cargo fmt passes. All 364 Rust tests pass, with the existing
  artifact-dependent ONNX and opt-in Docker tests ignored. Ten new coordinator
  regressions and the file-backed encrypted desktop adapter probe pass. Desktop
  typecheck reports zero errors/warnings; all 184 frontend tests and SPA build pass.
  Five scanner regressions pass; 438 tracked text files yield zero secret/PII/legacy
  findings. Changed Markdown passes cached Prettier; legacy-prop and diff checks pass.
  Normal dependency closure contains no Tauri/GTK/WebKit. Final coordinator fixture
  refinement passes its targeted suite and the repeated formatting/Clippy checks.
- **Coverage:** Selectable folders, discovered INBOX priority, cached fallback/exact
  comparison, best-effort catalog failure, sequential full/short batches, reset and
  saturating frontier handling, fatal remote/storage errors, paused-clock deadline,
  cancellation and checkpoint resume. Desktop regression checks encrypted reopen,
  canonical duplicate identity, history/arrival rule and review frontiers, rule effects
  before indexing, independent rule invalidation and final success ordering. Existing
  batch interruption, reset, lazy-attachment and embedding failure regressions pass.
- **Perimeter:** ADR-024, active contracts and k-038 preserve host ordering and
  cancellation limits. No documented CLI flags, environment variables or validation
  commands changed, so paired README edits were unnecessary. Existing cached
  toolchain/native sysroot were used without downloads or live provider/AI/delivery
  calls. No user mail, real credentials or image publication was involved.
- **Limits:** Entire-pass work remains proportional to selected folders and batches;
  only remote batch fetching has the existing 120-second deadline. Baseline/frontier
  writes precede the message transaction. Dropped remote work is canceled, but
  committed batches persist and already running host blocking work can continue.
  This does not establish cross-process writer ownership or live interoperability.
- **Next:** No prioritized successor is available. Active headless unlock/bootstrap,
  process ownership, scheduler/delivery consent, daemon lifecycle and writable
  container deployment require a separate plan. No successor started; nothing pushed.

#### Closing Checklist — [00.5]

- [x] Implementation plan approved.
- [x] Pure Rust reusable coordinator and desktop adapter preserve sync behavior.
- [x] Offline orchestration and encrypted-storage adapter regressions pass.
- [x] Full repository DoD, SPA build, formatting and secret/PII checks.
- [x] ADR, active contracts and applicable knowledge preserved.
- [x] Atomic Conventional Commit recorded; task closed without pushing.

---

### Task [00.4]: Offline Cached-Mail CLI Container Packaging

- **Status:** DONE
- **Selection:** Proposed from the unprioritized Docker runner backlog after the
  user's `proceed` on 2026-11-05. [00.3] remains complete.
- **Systems Involved:** Container packaging, offline synthetic integration validation,
  CLI usage documentation and paired README updates.
- **Scope:** Package the existing offline CLI for Linux containers. Preserve its
  command surface, schema-11 read-only contract and vault-only unlock. No active sync,
  daemon, provisioning, automation, model loading or remote actions.

#### Approved Implementation Plan (2026-11-05)

1. Inventory the local Docker daemon, cached base images, Rust dependencies and native
   binary linkage without downloads. Select one locally verifiable Linux target and
   explicit base-image identity. Package only the CLI and necessary runtime libraries;
   omit source caches, mail, vault files, credentials and desktop/ONNX dependencies.
   Use a narrowly allowlisted build context. If missing images/tooling prevent offline
   validation, return to planning with the exact provisioning requirement; no pulls,
   package downloads or registry publication are authorized by this plan.
2. Add a container recipe using the existing locked CLI build and a non-root runtime.
   Require an explicit existing data-directory mount and fallback unlock through stdin;
   do not expose the host keyring socket, accept secrets through environment/arguments,
   or create/copy an installation. Document `--network none`, no published ports,
   dropped capabilities, no-new-privileges, read-only root filesystem, bounded private
   temporary storage and host UID/GID permissions. Preserve the existing CLI defaults
   and fixed diagnostics without an entrypoint that starts additional processes.
3. Validate encrypted synthetic installations in the actual container: help, accounts,
   lexical search, plain-text show, fallback/recovery unlock, wrong/missing passphrases,
   absent installations and incompatible schema. Exercise committed WAL visibility
   with existing sidecars under a read-only data mount; fail closed when sidecar
   access is insufficient, without immutable mode or a writable-data workaround.
   Assert unchanged logical mail/queues and no installation creation. Inspect image
   contents, user, libraries and network-isolated execution. Keep fixtures/passphrases
   ephemeral and outside the image/build context and tracked files.
4. Document supported platform, local build/run commands, private stdout, passphrase
   stdin, permissions, schema compatibility and WAL/SHM requirements in docs/cli.md.
   Load readme-standard before synchronizing README.md and README.pt-br.md. State
   that this is offline inspection; active headless sync/daemon remains separate.
5. Run the full repository DoD, SPA build, applicable formatting and tracked-file
   secret/PII scans. Record container-specific limitations in active contracts and
   an atomic knowledge card if applicable. Make an atomic Conventional Commit, log
   its hash and close only [00.4]. Do not push, publish an image or start a successor.

- **Approval:** User approved with `proceed` on 2026-11-05.
- **Approval boundary:** Packaging and offline validation only, using existing local
  tooling/artifacts. No network exception, core trait/schema/IPC change, new runtime
  command, live personal data test or writable database mount. Missing prerequisites
  or material security/architecture changes require an updated plan.

#### Completion (2026-11-05)

- **Commit:** `b1b4c21` (`feat(cli): package offline cache inspection in a minimal container`).
- **Delivered:** Linux x86_64 GNU CLI image assembled from `scratch` using an explicit
  locked/offline release target, four allowlisted ELF loader/runtime libraries and
  license notices. Generated private context contains no source, mail, vault or models
  and is removed on exit. Numeric non-root default, direct CLI entrypoint and help
  default; no shell, desktop runtime, ONNX runtime, new dependencies or core changes.
- **Runtime:** Documented read-only data/root mounts, nonzero authorized host UID/GID,
  network none, dropped capabilities/no-new-privileges, disabled Docker logs and 16 MiB
  private tmpfs. Existing fallback vault receives its passphrase through stdin only;
  no host keyring socket, secret arguments/environment or provisioning is added.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy (-D warnings)
  exit 0: 353 Rust tests pass; optional ONNX and Docker tests are ignored ordinarily.
  The explicit Docker regression passes against the final image. Desktop check has
  zero errors/warnings; all 184 frontend tests and SPA build pass. Five scanner tests
  pass; 433 tracked text files have zero secret/PII/legacy-Svelte findings. Shell syntax,
  README perimeter, applicable Markdown Prettier, legacy-prop and diff checks pass.
- **Coverage:** Default help/non-root configuration; exact native/container accounts,
  search and show JSON parity; recovered fallback unlock, wrong/empty passphrases,
  missing vault/cache, unsupported schema, committed uncheckpointed WAL, missing-SHM
  refusal and durable DB/WAL/vault bytes plus unread/queue/schema preservation. Image
  contents were inspected. Native-reader SHM read-mark changes are excluded from byte
  comparisons; the read-only container mount still enforces SHM write refusal.
- **Artifact:** Local `triage-mail-cli:local`, image ID
  `sha256:ffeaa375183008d42ff4fa03bb1715fcd0fc4d0fb12fcac23e3577a71238c43b`,
  Docker-reported size 6,894,795 bytes. Host-derived runtime libraries/toolchain mean
  cross-host reproducibility is not claimed. No image or Git changes were published.
- **Perimeter:** EN/PT-BR READMEs updated under readme-standard, docs/cli.md includes
  build/run/test commands and WAL restrictions, active contracts and k-037 preserved.
  No downloads, provider/AI calls, personal data or live deliveries were used.
- **Limits:** Linux x86_64/Docker Desktop validated only. Host Debian/Ubuntu-style runtime
  license notices are required. Fallback-only unlock; initialized readable WAL/SHM
  sidecars are needed on read-only mounts, otherwise fail closed. No active sync,
  daemon, automation or new runtime network permission. No large-cache/native-platform
  interoperability or performance claim.
- **Next:** No prioritized successor is available. Active headless sync/daemon and its
  container deployment remain unprioritized and need a separate plan. No task started.

#### Closing Checklist — [00.4]

- [x] Implementation plan approved.
- [x] Minimal non-root offline CLI image with bounded build context.
- [x] Actual offline container tests with synthetic encrypted cache and WAL cases.
- [x] Full repository DoD, SPA build, formatting and secret/PII checks.
- [x] EN/PT-BR documentation and applicable contract/knowledge preservation.
- [x] Atomic conventional commit with recorded hash; task closed without pushing.


---

### Task [00.3]: Offline Headless CLI for Cached Mail

- **Status:** DONE
- **Selection:** Proposed from the unprioritized headless CLI / Docker runner backlog
  after the user's `proceed` on 2026-11-05. [03.10] remains complete.
- **Systems Involved:** New `apps/cli` Rust workspace member, additive read-only
  storage/vault access where necessary, offline integration tests and documentation.
- **Scope:** A first headless executable for existing encrypted cached mail: list
  opaque account IDs, lexical search and explicit plain-text message inspection.
  No daemon, Docker image, sync workers, remote actions or account provisioning.

#### Approved Implementation Plan (2026-11-05)

1. Add a pure Rust `triage-mail-cli` workspace executable with `#![forbid(unsafe_code)]`
   and no dependency on Tauri or desktop runtime modules. Reuse cached dependencies;
   record the composition and read-only boundary in an ADR. Preserve all existing
   core traits, IPC payloads, migrations and exact search behavior.
2. Require an explicit existing app-data directory. Retrieve the existing database
   key through `vault` only; never generate a replacement key, initialize storage,
   migrate a database or repair a missing installation. Respect recovered-vault
   fallback-only semantics. Support explicit fallback unlock through bounded stdin,
   zeroize input, and accept no secrets in command arguments or environment variables.
   Missing keys, wrong passphrases and unsupported schemas fail with fixed errors.
3. Add an additive read-only repository opener using SQLite read-only flags and
   query-only enforcement, preserving encrypted WAL visibility without immutable-mode
   assumptions. Reuse stored tokenizer configuration. Never mutate logical data,
   queue work or change journal mode; document any SQLite WAL/SHM sidecar requirements.
   Validate current supported schema before querying; do not run schema migrations.
4. Implement `accounts`, `search` and `show` commands with stable bounded JSON output
   and documented exit codes. Accounts exposes opaque IDs only. Search is lexical,
   supports an optional exact account ID and bounded limit/offset, and returns cached
   row summaries. Show explicitly returns cached plain text for one opaque message ID,
   with size limits and truncation metadata; never fetch or render HTML/attachments.
   JSON-escape control characters and keep private mail out of diagnostics.
5. Test the executable against synthetic encrypted fixtures: correct/wrong/missing
   keys, fallback unlock, recovered-vault selection, incompatible schemas, committed
   WAL data, concurrent desktop-style writes, pagination/account isolation, malformed
   arguments, output bounds and control characters. Assert no logical writes, schema
   upgrades, new-installation creation or pending-work processing. Verify the CLI
   package builds without Tauri/GTK/WebKit dependencies.
6. Load `readme-standard` before paired README edits; locate the skill or stop if it
   is unavailable. Document commands, secret input, private stdout, schema compatibility
   and the cached-only scope in EN/PT-BR. Run the full repository DoD, SPA build,
   applicable formatting and tracked-file secret/PII scans. Preserve active contracts
   and relevant knowledge, make an atomic Conventional Commit, log its hash and close
   only [00.3]. Do not push or start a daemon/Docker successor.

- **Approval:** User approved this plan with `proceed` on 2026-11-05.
- **Approval boundary:** This implementation adds
  one workspace executable and additive local read APIs only. No network provisioning,
  new runtime network permission, key export, desktop-runtime extraction, semantic
  indexing or automation execution is included. Material dependencies or changes to
  the storage/vault security contract return to planning.

#### Completion (2026-11-05)

- **Commit:** `9910526` (`feat(cli): add offline read-only encrypted mail inspection`).
- **Delivered:** Standalone `triage-mail-cli` Rust workspace executable with `accounts`,
  lexical `search` and cached plain-text `show`; no desktop runtime dependency or workers.
  Existing vault-only keys, explicit bounded non-terminal fallback unlock and recovery
  marker gating; missing installations/keys fail without creation. Additive read-only
  vault and SQLCipher readers preserve existing traits, migrations and IPC contracts.
- **Storage/output:** READONLY and query_only enforce no logical writes; schema-11
  validation/ranking/hydration share snapshots and include committed WAL. Stored
  tokenizer and lexical ranking remain authoritative. Paginated account IDs impose
  no account limit. Plain text is bounded to 64 KiB; complete JSON to 4 MiB, with
  C0/DEL/C1 escaping, fixed errors and no oversized partial payload on stdout.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy (-D warnings)
  exit 0: 353 Rust tests pass, one existing artifact-dependent ONNX smoke ignored.
  Desktop check has zero errors/warnings, all 184 frontend tests and SPA build pass.
  Five policy-scanner regressions pass; 429 tracked text files have zero secret/PII/
  legacy-Svelte findings. Whitespace and legacy-prop checks pass; new Markdown files
  pass cached Prettier. No frontend source changed. Standalone package build and
  dependency/ldd inspection confirm no Tauri/GTK/WebKit dependency.
- **Coverage:** Encrypted WAL/concurrent commits, write refusal even with query_only
  disabled, lexical parity/tokenizer, scopes/pagination, read/queue preservation,
  fallback/recovery unlock, wrong/missing keys, schemas, absent installations, direct
  symlinks, UTF-8 bounds, escaped controls and output rejection. Full validation found
  an early-exit BrokenPipe race in the test harness; only that expected pipe closure
  is now tolerated, with child exit/output assertions retained.
- **Perimeter:** ADR-023, active contracts, k-036, docs/cli.md and paired README usage
  synchronized under readme-standard. Existing cached dependencies/sysroot were used;
  no downloads, live mail/provider/AI calls or external deliveries. Nothing pushed.
- **Limits:** Read-only logical storage can still need WAL/SHM sidecars/directory
  permissions. Output caps do not bound all internal search hydration memory/latency.
  Tests do not establish cross-platform native keyring/packaging interoperability.
  Active sync, daemon/Docker, provisioning, mutations and semantic CLI remain deferred.
- **Next:** No prioritized successor is available; remaining backlog needs selection
  and planning. No additional task started.

#### Closing Checklist — [00.3]

- [x] Implementation plan approved.
- [x] Headless cached-mail commands with vault-only keys and read-only SQLCipher access.
- [x] Offline CLI/security regressions and GUI-free package build.
- [x] Full repository DoD, SPA build, formatting and secret/PII checks.
- [x] EN/PT-BR documentation, ADR/contracts and applicable knowledge preservation.
- [x] Atomic conventional commit with recorded hash; task closed without pushing.

---

### Task [03.10]: Real Local Tier 2 ONNX Embeddings

- **Status:** DONE
- **Selection:** Proposed from the unprioritized backlog after the user's `proceed`
  on 2026-11-05. User approved the plan and development provisioning with `proceed`
  on 2026-11-05. [02.7] remains complete.
- **Systems Involved:** `crates/triage-ai`, vector lifecycle in `crates/storage`,
  desktop native runtime/model settings, EN/PT-BR documentation and regression tests.
- **Problem:** `LocalCpuEmbeddingProvider` currently produces feature-hash projections;
  cached ONNX artifacts only seed that projection. Downloading a model does not run it.
- **Scope:** Native CPU inference for one supported 384-dimensional model, explicit
  artifact installation, safe replacement of incompatible cached vectors and bounded
  resumable indexing. Preserve existing core traits, IPC payloads, exact int8 ranking,
  local-first consent and lazy attachments. No model training or cloud inference.

#### Approved Implementation Plan (2026-11-05)

1. Inventory cached dependencies, native runtime libraries and model artifacts.
   Evaluate `ort`/`fastembed` integration against `#![forbid(unsafe_code)]`, headless
   builds, CPU-only execution, distribution licensing and explicit-download policy.
   Select and pin one supported 384-dimensional model/runtime combination; document
   tokenizer, truncation, pooling, normalization and quantization in an ADR. Disable
   library-provided automatic downloads and telemetry. If required tooling cannot
   be provisioned within the approval below, return to planning before proceeding.
2. Add a real inference provider behind the unchanged `EmbeddingProvider` trait.
   Bound input/token lengths, batch size, concurrency and CPU threads; reuse loaded
   sessions off async/UI threads. Validate shapes and finite outputs before int8
   quantization. Missing/corrupt/incompatible artifacts produce fixed errors, never
   an implicit projection fallback. Retain existing projection APIs for compatibility
   while removing their use from the learned-embedding runtime path.
3. Establish authenticated artifact identity using a pinned manifest and checksums,
   including tokenizer and preprocessing identity. Install only after explicit action;
   keep weights outside the binary in the existing XDG cache. Preserve bounded HTTPS
   downloads, no redirects/proxies and no initialization/query-triggered network work.
   Do not treat nonempty cached files as proof that inference is ready.
4. Add append-only encrypted vector-generation metadata as needed, preserving earlier
   migrations, traits and int8[384] storage. Existing unversioned/projection vectors
   are incompatible. Prevent mixed-generation query/document comparisons across every
   semantic search entry point, including saved views and conversations. Invalidate
   stale writes on model changes and rebuild from cached text in bounded resumable
   batches, with transactional progress and deletion/hydration handling. Lexical search
   stays available; indexing never fetches mail or executes rules/reminders/deliveries.
   Keep the existing backup format compatible by excluding rebuildable generation/index
   state or explicitly rebuilding it under the existing restore contract.
5. Integrate native readiness/index progress and explicit enable/rebuild controls using
   additive IPC where necessary. Retain semantic opt-in, disabled-by-default behavior,
   stale-request/dirty/busy guards and EN/PT-BR parity. Clearly distinguish ready,
   missing model, rebuilding and failed states. Load `svelte-dev` before component edits
   and `readme-standard` before paired README updates; stop if a required skill is missing.
6. Verify actual ONNX execution with small synthetic graph fixtures, malformed-artifact
   and output tests, and an explicitly provisioned real-model smoke evaluation using
   synthetic EN/PT-BR mail only. Test upgrade/restart, model replacement, concurrent
   ingestion/deletion, stale writes, restored caches and all semantic consent gates.
   Record cold/warm inference, indexing throughput and memory; distinguish model quality
   evidence from exact search correctness and retain existing performance targets.
7. Run the full repository DoD, SPA build, applicable formatting and secret/PII scans.
   Preserve contracts and platform gotchas, update paired usage documentation, make an
   atomic Conventional Commit, record its hash and close only this task. Do not push.

- **Approved provisioning:** This implementation plan plus narrowly scoped development
  provisioning: official dependency/model documentation, Rust package dependencies,
  CPU ONNX runtime artifacts and the selected public model/tokenizer needed for local
  validation. Record versions, sources, licenses and checksums; use ignored scratch/cache
  for binaries and weights, never system-wide installation or external corpus uploads.
  User approved this development-only network exception with `proceed` on 2026-11-05.
  Runtime model downloads remain separately user-initiated.
- **Approval boundary:** No live mail/provider calls, external deliveries, new cloud AI
  consent, ANN ranking changes, arbitrary model catalog, new crate or daemon. Material
  dependency, compatibility or security changes outside this plan return to planning.

#### Completion (2026-11-05)

- **Commit:** `dc84fd1` (`feat(embeddings): run pinned local ONNX inference with resumable indexing`).
- **Delivered:** Genuine CPU MiniLM inference behind the unchanged embedding trait;
  full model/tokenizer/native-runtime hash verification; no automatic fetch or projection
  fallback in the native desktop. Append-only v11 tracks source versions and rebuild
  epochs, rejects stale writes and resumes local indexing after restart. All semantic
  variants gate complete compatible indexes in their ranking snapshots. Existing
  injected-provider behavior, exact ranking and backup v1/schema-10 payload stay compatible.
- **Controls:** Additive readiness/rebuild IPC and EN/PT-BR settings expose safe counts,
  opt-in and confirmed rebuild with teardown/busy guards. Missing files leave lexical
  startup/search available. Runtime and weights are installed separately; direct model
  downloads require explicit consent and pinned hashes. No live mail, cloud inference,
  external deliveries, telemetry or binary attachments were used.
- **Validation:** Workspace check/test/fmt/all-target Clippy (-D warnings) exit 0;
  desktop check reports zero errors/warnings, all 184 frontend tests and SPA build pass.
  Changed frontend files pass Prettier using existing cached tooling. Five policy
  regressions pass; the full 419 tracked-text-file scan reports zero secret/PII/Svelte
  findings. Legacy-prop and whitespace checks pass. The optional artifact-dependent
  ONNX smoke is ignored in ordinary tests and passed separately offline, including a
  generated Identity graph, malformed-graph rejection and the genuine pinned model.
- **Coverage:** Manifest rejection, pooling/shape/finite/UTF-8 bounds, source hydration,
  deletion, failure/retry, generation replacement, stale results from another writer,
  encrypted reopen, upgrade, filtered/saved/review/conversation gates, backup restore,
  missing-runtime startup, explicit consent, EN/PT-BR controls and late-result cleanup.
- **Evidence:** Standalone release 256-message encrypted fixture: session preparation
  140.039ms; indexing 712.080ms / 359.510 rows/s; inference p50/p95 1.572/7.473ms;
  peak process RSS 98,868KiB. Debug smoke: preparation 1,046.662ms and four short
  EN/PT-BR documents in 7.020ms. These are small repeated-text runs with cached files
  and concurrent development checks, not large-mailbox/relevance/UI performance claims.
- **Perimeter:** ADR-022, active contracts, k-035, docs/onnx.md, paired READMEs and the
  reserved development model example are synchronized. Approved provisioning used
  official package/model/runtime sources and ignored scratch without system installs.
  Native execution was validated on Linux x64; other platform hashes do not establish
  native signing/packaging/interoperability. Existing latency and language-quality
  limitations remain documented. No new crate or public trait/payload rewrite.
- **Next:** No prioritized successor is available. Remaining backlog requires selection
  and planning; no additional task started. Nothing pushed.

#### Closing Checklist — [03.10]

- [x] Plan and narrowly scoped development provisioning approved.
- [x] Genuine CPU ONNX execution with verified artifacts and no hidden downloads.
- [x] Generation-safe migration/reindex, restart and semantic entry-point coverage.
- [x] Full Rust/frontend DoD, SPA build, formatting and secret/PII checks.
- [x] EN/PT-BR documentation, ADR/contracts and applicable knowledge preservation.
- [x] Atomic conventional commit with recorded hash; task closed without pushing.


### Task [02.7]: Coverage-Guided IMAP Parser Fuzzing

- **Status:** DONE
- **Selection:** Proposed from the unprioritized backlog after the user's `proceed`
  on 2026-11-04; user approved the plan and development-only tooling provisioning
  with `proceed` on 2026-11-04. [02.6] remains complete.
- **Systems Involved:** Development-only fuzz harnesses for `crates/mail-engine`,
  synthetic seed corpus, parser regression tests and reproduction documentation.
- **Scope:** Coverage-guided exploration of BODYSTRUCTURE and fragmented FETCH
  parsing through existing public APIs. Preserve production traits, IPC, schemas,
  lazy attachments and core unsafe-code restrictions. No live provider calls.

#### Approved Implementation Plan (2026-11-04)

1. Establish an isolated development-only fuzz package using cargo-fuzz/libfuzzer-sys,
   without adding fuzz dependencies to production crate builds. Inventory installed
   tooling and cached dependencies before edits. Preserve the existing workspace DoD.
2. Add BODYSTRUCTURE and FETCH targets using existing public parser/session APIs.
   Feed FETCH through bounded in-memory AsyncRead/AsyncWrite, without sockets,
   credentials or authentication. Compare equivalent fragmentation modes and retain
   exact payload/command-boundary assertions for structured valid inputs. Invalid
   UTF-8 is rejected at the str API boundary, not lossily converted.
3. Seed from synthetic [02.6] cases: binary/empty literals, CRLF boundaries, escaped
   UTF-8, malformed lengths, truncation and nesting. Bound input to 64 KiB and reads
   to 100,000; configure process-level time/memory limits so synchronous recursion
   or allocation failures cannot rely on a Tokio timeout for containment.
4. Run a bounded initial campaign (five minutes per target), retaining commands,
   tool versions, seed, execution counts and observed failures. Minimize reproducible
   findings and add ordinary deterministic regressions. Fix only demonstrated local
   parser defects; production recursion/allocation policy or protocol redesign
   requires separate approval before implementation.
5. Document offline build/run/replay commands and corpus hygiene; keep generated
   corpora, crash artifacts and build output ignored. Commit only reviewed synthetic
   seeds. Synchronize paired READMEs under readme-standard if validation commands
   change. Record coverage limits without claiming exhaustive correctness.
6. Run the full repository DoD, SPA build, applicable formatting and secret/PII
   scans. Preserve non-obvious findings in k-034 or a new atomic card. Make an atomic
   Conventional Commit, record its hash and close only this task. Do not push.

- **Tooling authorization:** User explicitly approved development-only cargo-fuzz,
  libfuzzer-sys and compatible instrumented Rust toolchain provisioning. Use ignored
  project scratch for installed tools; do not alter shared toolchain binaries.
- **Approval boundary:** Provisioning does not authorize CI services, external corpus
  uploads, runtime network access, broad parser redesign or another backlog item.

#### Completion (2026-11-04)

- **Commit:** `24a7374` (`test(mail-engine): add coverage-guided imap parser fuzzing harness and campaign`).
- **Delivered:** Isolated `fuzz/` Cargo package with `bodystructure` and `fetch` libFuzzer
  targets, bounded in-memory streams, seed corpora (binary/empty literals, split UTF-8,
  nesting, malformed tags, overflow, CRLF boundaries), and deterministic seed replay
  tests running offline.
- **Campaign results:** Bounded 5-minute initial campaigns using cargo-fuzz 0.13.2 /
  libfuzzer-sys 0.4.13 on x86_64 Linux (seed 4242, 64 KiB max input, 5s timeout,
  2048 MB RSS, 256 MB malloc):
  - `bodystructure`: 2,343,067 runs in 301s (~7,784 exec/s), peak RSS 568 MB, cov 922,
    ft 3,386, 0 crashes / 0 timeouts / 0 OOMs.
  - `fetch`: 563,885 runs in 301s (~1,873 exec/s), peak RSS 419 MB, cov 1,633,
    ft 4,426, 0 crashes / 0 timeouts / 0 OOMs.
- **Validation:** Workspace check/test/fmt/Clippy pass with 335 Rust tests and zero
  Clippy warnings. Desktop check has zero errors/warnings; all 181 frontend tests and
  SPA build pass. Fuzz package check/test/fmt/Clippy pass with zero warnings. Five
  policy-scanner regressions pass; all 408 tracked text files report zero secret/PII/legacy-Svelte
  findings. `git diff --check` passes cleanly.
- **Perimeter:** No production dependencies, public traits, IPC, schemas or UI changes.
  Tooling runs offline with local seeds and scratch tools. Documentation in `docs/testing/imap-fuzzing.md`,
  READMEs, and `k-034` synchronized.
- **Limits:** Bounded coverage-guided campaigns explore mutation space without crashes,
  but do not establish exhaustive grammar correctness or substitute for production
  resource limits.
- **Next:** No prioritized successor; select and plan another backlog item. Nothing pushed.

#### Closing Checklist — [02.7]

- [x] Plan and development-only tooling provisioning approved.
- [x] Isolated targets, reviewed synthetic seeds and reproducible bounded campaign.
- [x] Demonstrated defects covered by ordinary deterministic regression tests.
- [x] Full Rust/frontend DoD, SPA build, formatting and secret/PII checks.
- [x] Documentation/knowledge preservation and atomic commit with recorded hash.
- [x] Task closed; no prioritized successor available to promote.


### Task [02.6]: IMAP FETCH/Literal Parser Property Tests

- **Status:** DONE
- **Selection:** Proposed from the unprioritized backlog after the user's `proceed` on
  2026-11-04; user approved the plan with `proceed` on 2026-11-04.
- **Systems Involved:** `crates/mail-engine/src/protocol.rs`,
  `crates/mail-engine/src/bodystructure.rs`, and mail-engine test fixtures.
- **Scope:** Offline deterministic generated/property-style tests for FETCH framing,
  literal byte preservation and BODYSTRUCTURE parsing, with narrowly scoped fixes
  for demonstrated parser defects. Preserve IPC, storage schemas, core traits and
  lazy attachment behavior. No new dependencies or live provider calls.

#### Approved Implementation Plan (2026-11-04)

1. Add a bounded in-memory AsyncRead/AsyncWrite test adapter and deterministic
   generators using existing dependencies. Exercise identical synthetic responses
   under one-byte, boundary-aligned and seeded fragmented reads; use explicit input,
   iteration and execution bounds, without sockets or credentials.
2. Generate valid FETCH responses with multiple messages/sections, reordered metadata,
   empty and binary literals, embedded CRLF/parentheses/tag-like text, and a following
   tagged response. Compare results against independently constructed expected metadata
   and exact payload bytes; verify fragmentation does not change command boundaries.
3. Exercise malformed/truncated literals, numeric overflow, invalid UTF-8 control lines,
   escaped/unterminated strings, mismatched parentheses and bounded nested
   BODYSTRUCTURE inputs. Require deterministic errors for malformed tested grammar,
   no panics and termination. Include reproducible seeds and small regression cases.
4. Fix only defects demonstrated by those tests, including checked literal arithmetic
   and parser rejection where necessary. Document safety invariants inline. If fixes
   require broader protocol redesign or material new resource-limit policy, return
   to planning before implementation of that change.
5. Run the full repository DoD, frontend build and applicable formatting/policy scans.
   Record actual case counts, seeds and limits without claiming exhaustive fuzzing.
   Preserve any non-obvious findings in an atomic knowledge card; update contracts
   only as needed. Make an atomic Conventional Commit, record its hash and close the
   task. Do not push or start another backlog item.

- **Approval boundary:** Approval authorizes the deterministic test harness and
  demonstrated local parser fixes above. Continuous coverage-guided fuzzing tooling,
  dependency downloads and unrelated sync changes remain outside this task.

#### Completion (2026-11-04)

- **Commit:** `6dc0c8d` (`fix(mail-engine): harden literal parsing with generated regressions`).
- **Delivered:** Six offline deterministic property/regression tests using bounded
  in-memory IO. Seeds 1, 42, 4242 and 0x5eed cover 20 valid fragmented command runs
  (160 messages), 40 malformed FETCH runs, 56 truncated prefixes in five modes (280
  runs), 136 valid nested/escaped/literal BODYSTRUCTURE cases, ten malformed metadata
  cases and 1,024 generated ASCII expressions parsed twice for determinism.
  Payload sizes include exactly 4095/4096/4097 bytes and empty sections. Input/read
  budgets are 64 KiB/100,000; async command timeout is five seconds; generated valid
  nesting is 0..16 and generated arbitrary strings are at most 127 bytes.
- **Reproduced and fixed:** Quoted UTF-8 corruption; BODYSTRUCTURE literal addition
  overflow and UTF-8 boundary panics; acceptance of trailing tokens and missing CRLF;
  malformed/overflowing FETCH literals and unclosed framing reported as success.
  FETCH literal bytes remain opaque and surplus bytes remain available to the next
  command. Quoted parentheses and metadata following literals have regression coverage.
- **Validation:** Workspace check/test/fmt/Clippy pass with 335 Rust tests and zero
  Clippy warnings. Desktop check has zero errors/warnings; all 181 frontend tests and
  SPA build pass. Five policy-scanner regressions pass; all 385 tracked text files
  report zero secret/PII/legacy-Svelte findings. Legacy-prop and diff checks pass.
- **Perimeter:** No dependency, CLI flag, environment variable, validation command,
  public trait, IPC, migration or UI changes. README synchronization is not needed.
  NOTES and k-034 preserve framing/UTF-8 invariants, test bounds and coverage limits.
  Existing offline toolchain/sysroot were used; no downloads or provider calls.
- **Limits:** Deterministic properties do not establish exhaustive grammar correctness,
  coverage-guided fuzzing or production resource limits. No unrelated protocol redesign.
- **Next:** No prioritized successor; select and plan another backlog item. Nothing pushed.

#### Closing Checklist — [02.6]

- [x] Rust/frontend strict typing, automated tests, formatting and Clippy.
- [x] SPA build, Svelte invariant, secret/PII policy scan and clean diff.
- [x] Reproducible bounded offline coverage and demonstrated defect fixes.
- [x] Active contracts and k-034 knowledge preservation; documentation perimeter checked.
- [x] Atomic conventional implementation commit and recorded hash.
- [x] Task closed; no prioritized successor available to promote.

---

## Next Milestone (v0.2.0)

### 📌 Task [02.1]: Intelligent Account Autodiscovery & Streamlined Onboarding

- **Description:** Add offline provider presets, explicit domain-only ISPDB lookup and confirmed-host TLS probing in `crates/mail-engine`, exposed through additive Tauri IPC. Streamline email-first onboarding, Microsoft PKCE with a user-provided client ID, Gmail App Password assistance and advanced manual settings.
- **Systems Involved:** `crates/mail-engine` (`discovery/`), `apps/desktop/src-tauri` (`onboarding.rs`, `runtime.rs`), `apps/desktop` (`src/lib/onboarding/`).
- **Action Type:** Feature & UX Architecture.
- **Status:** DONE
- **Subtasks:**
  - `[02.1.1]` **Discovery Engine Core (`crates/mail-engine`):** Parse domain from RFC 5322 email addresses, match against instant offline presets table (Gmail, Outlook/Hotmail, Yahoo, iCloud, Fastmail, Zoho, popular BR domains), query Mozilla ISPDB (`autoconfig.thunderbird.net/v1.1/{domain}`) with 3s timeout, and offer a separately confirmed TLS port 993 probe.
  - `[02.1.2]` **IPC Bridge & DTOs (`apps/desktop/src-tauri`):** Implement `cmd_discover_email_provider` returning `DiscoveredProviderDto` with recommended auth method, OAuth endpoint metadata, and contextual help URLs without leaking credentials.
  - `[02.1.3]` **Streamlined Onboarding UI (`apps/desktop`):** Refactor `AccountForm.svelte` to feature a Microsoft PKCE setup button requiring the user’s own client ID, debounce autodiscovery on email input blur, provide direct link assistance for Gmail App Passwords (`myaccount.google.com/apppasswords`), and place host/port/BYO-OAuth/Proton controls in a collapsible advanced details panel.
- **Constraints:**
  - Zero credential leakage in logs or debug strings.
  - Strict timeout ($\le 3$s) on external ISPDB requests; never freeze UI.
  - Svelte 5 Runes strictly enforced (`$state`, `$derived`, `$effect`, zero `export let`).
  - Unit tests MUST use RFC 2606 reserved domains (`@example.com`, `@example.org`).

#### Completion (2026-11-02)

- **Commit:** `3c25660` (`feat(onboarding): add consent-based provider discovery`).
- **Validation:** Workspace check/test/fmt/Clippy passed (222 Rust tests). Desktop check
  passed with zero errors/warnings; all 133 frontend tests and SPA build passed.
  Changed frontend files pass Prettier; policy guard regression tests pass (5 tests)
  and the tracked-file secret/PII/Svelte policy scan reports zero findings. Diff check
  and the legacy Svelte syntax invariant pass.
- **Environment:** Used the existing native sysroot and toolchains described in k-001
  and k-006. Full Rust tests require loopback access; policy guard tests require child
  processes, so those validations ran outside the restrictive sandbox. No provider
  credentials, model downloads or live-provider authentication were used.
- **Contracts:** Existing IPC/account authorization and storage schemas are unchanged.
  New discovery defaults offline; probing is a separate explicit action. Microsoft
  setup uses user-provided credentials rather than a bundled application registration.
- **Documentation:** EN/PT-BR setup guides, active contracts and the approved narrow P0
  exception are synchronized. k-022 preserves discovery trust and parser/UI gotchas.
- **Next:** [01.1] remains PLANNING; implementation requires its own approved scope.

#### Approved Implementation Plan (2026-11-02)

1. `[02.1.1]` Add headless discovery types and an injectable discovery transport in
   `crates/mail-engine`. Validate and normalize the address domain, resolve offline
   presets first, and return an explicit unresolved result instead of guessing success.
   Confirm the supported Brazilian provider list during implementation review.
2. Under the approved network-policy exception below, add bounded ISPDB lookup (domain only,
   never the full address), defensive XML parsing, and bounded IMAP TLS probing.
   Disable redirects and inherited proxies; do not use provider XML for automatic OAuth
   endpoint trust or permit discovery to contact private/loopback destinations.
   Preserve explicitly configured local Proton Bridge behavior.
3. `[02.1.2]` Add the documented `cmd_discover_email_provider` DTO/command without changing
   existing account or authorization contracts. Keep errors fixed and credential-free;
   expose injectable offline services for tests. Discovery never authenticates or saves
   an account automatically.
4. `[02.1.3]` Refactor onboarding around email-first setup, offline discovery on blur,
   contextual Gmail assistance, and advanced manual settings. Ignore stale discovery
   responses and preserve manual edits. Retain transient credential inputs, existing
   OAuth cancellation, Proton Bridge, and EN/PT-BR parity. Load `svelte-dev` before UI edits.
5. Add offline discovery/parser/timeout and IPC tests plus UI regressions for stale
   results, manual overrides, and authorization. Run the full repository DoD commands,
   frontend build and formatting checks; review the diff for secrets and PII. Update
   contracts and applicable documentation, then make an atomic Conventional Commit
   and record its hash before promoting the next task. Do not push.

#### Approved Decisions (2026-11-02)

- **P0 network policy:** AGENTS.md permits only the approved, explicit discovery actions. Approved exception: retain automatic
  offline presets, require an explicit user action for domain-only ISPDB lookup, and
  require user confirmation of a discovered IMAP endpoint before any probe.
  User approved the proposed exception via `proceed`; online lookup and probing remain explicit runtime actions.
- **Microsoft OAuth:** No application client ID is configured in the current onboarding
  flow. Approved scope: streamline Microsoft PKCE using a user-provided client ID held
  transiently and stored only in vault, with a clear setup step before authorization.
  A first-use one-click flow without that setup requires a separate approved application
  registration and credential-provisioning design; do not embed borrowed client IDs.

### 📌 Task [01.1]: Binary Quantization & Global Vector Pre-filtering Evaluation

- **Description:** Evaluate 1-bit scalar/binary quantization (BQ) candidate generator + exact int8 reranking for sub-15ms global vector queries on 100k messages in SQLCipher, per ADR-008.
- **Systems Involved:** `crates/storage` (vector indexing, Hamming distance pre-filter, SQLCipher).
- **Action Type:** Investigation & Architecture.
- **Status:** DONE
- **Constraints:** Preserve single-file SQLCipher encryption, exact tie handling, and oracle equivalence.
- **Scheduling:** Promoted to planning after [04.1.7] closure; carries the unmet scoped-search budgets as motivation.

#### Authorized Evaluation (2026-11-02)

User explicitly requested proceeding with the ADR-008/mailbox evaluation. Build an
offline example over the existing 100k-message fixture, with a disposable encrypted
binary index, exact int8 cosine reranking, independent ordered-page oracle checks,
candidate-budget sweeps, deep-page fallback and adversarial correctness probes.
Record raw latency/recall/equivalence and an adoption recommendation. Production
schemas, IPC and ranking remain unchanged; no ANN graphs, downloads or provider calls.

#### Completion (2026-11-02)

- **Commit:** `568bedb` (`perf(storage): evaluate binary candidates against exact search`).
- **Evidence:** `docs/performance/binary-quantization.md`, raw CSV, complete summary,
  process report and offline reducer. 64 groups, 64 reopened and 1,920 warm samples.
- **Result:** Retain exact int8 cosine search (ADR-011). Global seed 4242 BQ-200
  p95 21.226ms retains 56/200 oracle hits; BQ-4096 p95 156.361ms retains 166/200.
  All 270 global warm BQ samples across three seeds fail ordered equality. All
  production and deep exact-fallback samples pass the independent oracle.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (222 Rust
  workspace tests); all three adversarial example probes pass. Desktop check has
  zero errors/warnings; all 133 frontend tests pass. Five policy regressions,
  tracked/new-file policy scan, Svelte invariant, reducer completeness, Prettier for
  the new JS tool and committed whitespace pass. No production code/schema/IPC changes.
- **Knowledge:** k-023 preserves binary/cosine coverage limits; k-024 records the
  vec0 int8 replacement-write fixture gotcha. EN/PT-BR reproduction docs synchronized.
- **Deviations:** Existing scoped/native/frame-cadence misses remain. Investigation
  completion does not declare the proposed sub-15ms exact optimization achieved.
- **Next:** [03.5] promoted to READY FOR PLANNING per the user's prioritized backlog.
  Plan the shared local rule evaluator, simulation, ordering and loop prevention;
  implementation awaits an approved plan. No next-task implementation is started.

---

## Definition of Done (DoD) Checklist — [03.9]

A task MUST NOT be marked done based on subjective appraisal. It MUST satisfy:
- [ ] Strict Typing: `cargo check --workspace` and `pnpm --filter desktop check` exit with code 0.
- [ ] Automated Tests: `cargo test --workspace` and `pnpm --filter desktop test` exit with code 0.
- [ ] Linters: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` exit with code 0.
- [ ] Svelte 5 Invariant: `! grep -rn "export let" apps/desktop/src/` exits with code 0.
- [ ] Secret & PII Scrubbing: Zero real email addresses, tokens, or secrets in tracked files; `git diff --check` exits with code 0.
- [ ] Documentation Perimeter: If CLI flags, `.env.example` variables, or validation commands changed, synchronize `README.md` and `README.pt-br.md` (following `readme-standard` skill).
- [ ] Knowledge Preservation: Document obscure platform/runtime gotchas or workarounds as an atomic card in `.agent/knowledge/` (if applicable).
- [ ] Atomic Commit: Conventional Commits in English (`feat(scope): ...`).
- [ ] Task Log: Active task logged in `.agent/TASK.md` with commit hash; next task promoted.

---

## Follow-up [04.1]: Inbox Settings Cleanup (2026-11-02)

- **Status:** IMPLEMENTED; full workspace validation blocked by concurrent discovery work.
- **User-requested scope:** Remove redundant Accounts and Settings buttons from message
  actions; keep the responsive settings gear and manage accounts inside its modal.
- **Commit:** `ff23564` (`fix(inbox): centralize account management in settings`).
- **Validation:** Desktop check (zero errors/warnings), all 126 frontend tests, production
  build, updated onboarding regression, Svelte syntax invariant, and diff check passed.
  Rust formatting reports discovery changes; Clippy reports discovery XML type errors.
  Cargo check/test were stopped after that blocker; existing [02.1] work and local
  edits remain owned by that task.

---

## Prioritized Feature Backlog (2026-11-02)

User-requested feature order below; [02.1] and [01.1] are complete. Each entry
requires its own approved plan before implementation and does not establish a release
commitment. [03.5] completed under its separately approved plan. Dependencies are implementation prerequisites, not parallel work assignments.
All tasks inherit the repository DoD when implemented.

### Task [03.5]: Local Rule Engine & Automatic Filters

- **Status:** DONE
- **Priority:** 1
- **Systems Involved:** `crates/triage-ai`, `crates/storage`, desktop IPC/UI.
- **Scope:** Share one local rule evaluator across filters and alerts. Match account,
  sender/domain, subject, body keywords, attachment metadata and extracted amounts/dates
  with all/any/except conditions. Support tags, stars and archive through existing
  mutation boundaries; define rule ordering, stop-processing behavior and loop prevention.
  Expose post-ingestion/triage integration through the existing observer boundaries.
- **Acceptance:** Create/edit/disable rules, create a rule from a message, and simulate
  against selected existing messages without mutations or deliveries. Explain matches.
  Regression fixtures cover compound conditions, exceptions and conflicting actions.
  Initial sync/history imports never trigger rules unless explicitly requested.

#### Approved Implementation Plan (2026-11-02)

1. `[03.5.1]` **Pure evaluator (`crates/triage-ai`):** Add versioned typed rule,
   condition, message-facts and explanation models in a `rules` module. Preserve
   existing traits. Account IDs match exactly; parsed sender addresses/domains match
   normalized exact values (domain suffix matching is an explicit operator). Subject,
   body and attachment filename/MIME keywords use documented case-insensitive literal
   matching, without scripts or user regex. Attachment predicates use metadata only.
   Compare extracted BRL amounts as integer cents and validated ISO calendar dates;
   missing facts never satisfy positive predicates. Define matching as all required
   conditions AND (any optional condition, when present) AND no exception. Reject
   empty match-all definitions and bound definition size/nesting and simulation batches.
2. `[03.5.2]` **Deterministic action planning:** Evaluate enabled rules in ascending
   explicit position, then stable rule ID, against one immutable message snapshot.
   Accumulate distinct local tags, star and archive actions; repeated actions coalesce.
   A matching stop-processing rule prevents later evaluation. V1 supports additive
   tags, star and archive only; no inverse actions, trash or automatic unsubscribe.
   Simulation uses this exact evaluator/planner and explains matched, excluded,
   missing-fact and stopped rules without writing state or invoking mutation adapters.
3. `[03.5.3]` **Encrypted persistence (`crates/storage`):** Add append-only migration
   v3 for rule definitions/revisions/order, per-folder initial-history baselines,
   durable eligible evaluation jobs and processed-event identities. Keep schema v1/v2
   and existing IPC/traits compatible. Persist eligibility in the ingestion transaction
   with the checkpoint so crashes cannot lose newly eligible work. Persist application
   markers, local changes and existing remote outbox entries atomically. Use canonical
   account-bound identity; a new folder membership is not a new message event. Reject
   invalid/stale revisions; validate account references and definitions natively.
4. `[03.5.4]` **Post-ingestion execution:** Compose a headless-compatible rule consumer
   through the existing post-triage observer boundary without changing its signature.
   Only newly inserted canonical messages after an established folder baseline are
   eligible. First sync, history catch-up/import, newly discovered folders and
   UIDVALIDITY rebuilds establish/refresh baselines without automatic rule actions;
   capture the initial UID frontier so interrupted multi-batch imports stay excluded.
   Messages received while an established account is offline remain eligible on return.
   Resume durable jobs after restart; rule edits/enablement never replay old mail.
   Rule-generated flags/moves, body hydration and duplicate sync events cannot trigger
   evaluation recursively. Reuse existing account-bound mutation/outbox primitives;
   preserve canonical folder/UID targeting and archive snooze clearing. Fixed safe
   errors and bounded job processing must not block sync or silently drop failures.
5. `[03.5.5]` **Additive IPC and desktop UI:** Add rule list/save/delete/reorder and
   simulation commands with typed DTOs, optimistic revision checks and bounded selected
   message IDs. Add an EN/PT-BR Rules settings module to create/edit/disable/order rules,
   choose stop-processing and inspect simulation explanations. Add create-from-message
   as an editable draft requiring save; prefill account and parsed sender without
   persisting message bodies. Simulation and saving never apply rules retroactively.
   Historical application/replay is deferred; existing mail can be simulated only.
   Load `svelte-dev` before UI implementation; retain modal guards and mounted inbox.
6. `[03.5.6]` **Evidence and closure:** Test compound/except/missing predicates,
   amount/date boundaries, deterministic order, coalescing, stop behavior, account
   isolation, migration upgrades, rollback/restart recovery, concurrent edits,
   deduplication, initial multi-batch history and reset suppression, observer loop
   prevention, simulation purity and UI lifecycle guards. Use synthetic reserved-domain
   fixtures and offline adapters. Run every repository DoD command plus frontend build,
   relevant formatting and policy scans. Update active contracts, document architectural
   decisions and applicable knowledge cards; load `readme-standard` if README changes
   are needed. Make an atomic Conventional Commit, record its hash and promote [03.6]
   to planning. Do not push or implement notifications/webhooks in this task.

- **Approval:** User approved the complete plan and proposed defaults via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `4b2e15d` (`feat(rules): add durable local filters and simulation`).
- **Delivered:** Shared pure evaluator/planner, ordered all/any/except conditions,
  tag/star/archive actions and stop behavior; encrypted append-only migration v3,
  revision-safe CRUD/reordering, durable ingestion eligibility, restart recovery,
  move-loop suppression and atomic mutation/outbox completion. Additive typed IPC,
  EN/PT-BR Rules settings, account selectors, create-from-message drafts and side-effect-free
  selected-message simulation preserve the mounted inbox and existing trait/IPC contracts.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass: 234 Rust tests.
  Desktop check reports zero errors/warnings; all 137 frontend tests and SPA build pass.
  Changed frontend files pass Prettier. Five policy scanner regression tests pass;
  tracked/new-file scan covers 272 text files with zero findings. Svelte legacy syntax,
  README perimeter and diff whitespace checks pass. No live-provider calls, credentials,
  model downloads or external deliveries were used.
- **Environment:** Existing k-001/k-006 toolchain/native sysroot. Workspace socket fixtures
  and policy child-process regressions require execution outside the restrictive sandbox;
  all remain local/offline. No system packages or shared toolchain files were modified.
- **Contracts:** ADR-012, active notes, k-025 and paired README usage guides record
  matching semantics, history frontiers and recovery. Rule IDs cannot be reused after
  deletion. Rule changes invalidate pending revisions without retroactive replay.
- **Limits:** Rules require an active runner. Each folder establishes a first post-upgrade
  UID frontier; initial history/rebuilds remain excluded. Missing UIDNEXT suppresses
  automation safely. Account-bound identity tombstones persist until account removal;
  indistinguishable headerless copies can be conservatively suppressed. Amount/date
  predicates test facts independently. Existing search/native performance deviations
  remain outside this feature. Historical application and alert deliveries are deferred.
- **Next:** [03.6] promoted to READY FOR PLANNING. Its external-delivery P0 exception
  still needs an explicit approved plan; no notifications/webhooks implementation began.

### Task [03.6]: Local Notifications & Generic Webhook Alerts

- **Status:** DONE
- **Priority:** 2
- **Depends On:** [03.5]
- **Systems Involved:** `crates/storage`, `crates/vault`, desktop runtime/IPC/UI;
  headless-compatible delivery adapter through existing observer boundaries.
- **Scope:** Notify on matching new mail locally or through explicitly configured
  webhooks, including user-owned n8n workflows. Select exported fields per destination;
  full bodies are off by default. Store secret URLs/authentication only in vault.
  Persist delivery jobs and bounded execution history in SQLCipher, with retry/backoff,
  destination rate limits, stable event IDs and local duplicate suppression.
- **Acceptance:** Offline adapter tests cover restart/reconnect, retries, disablement,
  payload field selection and secret-free errors/history. Ambiguous remote delivery
  outcomes are documented; do not promise exactly-once delivery to arbitrary receivers.
  No initial-history alert flood; manual replay requires explicit selection. Explain
  that processing requires the app or a separately deployed headless runner to be active.
- **Planning Gate:** Propose and obtain approval for a narrow P0 network-policy exception
  before enabling external deliveries. Specify destination consent, TLS, DNS/redirect
  protections and any explicitly configured private-network webhook behavior. This
  backlog entry does not amend `AGENTS.md` or authorize outbound requests.

#### Approved Implementation Plan (2026-11-02)

1. `[03.6.1]` **Shared alert planning:** Extend the typed local rule model additively
   with destination-referencing alert actions; preserve existing version-1 rules,
   ordering, stop semantics and mutation actions. Use the same immutable facts and
   evaluator for simulation and execution. Coalesce matching rules to one automatic
   delivery per account-bound event and destination. Add headless delivery request,
   outcome and adapter contracts in existing core modules; no new crate or Tauri
   dependency in core. Desktop composes the native notification and HTTP adapters.
2. `[03.6.2]` **Consent and vault lifecycle:** Destinations start disabled. Explicit
   enablement confirms destination identity, selected exported fields and external
   disclosure. Keep complete webhook URLs (including paths/query secrets) and optional
   bearer credentials exclusively in vault, behind opaque destination IDs. Return only
   safe metadata, public origin and credential-presence flags. Credential inputs stay
   transient and are cleared after submission/teardown. Stage immutable secret revisions
   before SQL configuration commit; recover/clean orphan revisions without exposing
   secrets. Configuration or field changes invalidate queued old revisions; re-enabling
   never silently replays canceled work. Serialize disable/delete against dispatch;
   an already-started request cannot be recalled. Saving never contacts a destination.
3. `[03.6.3]` **Durable encrypted delivery:** Add append-only migration v4 for safe
   destination metadata/revisions, delivery jobs, leases, attempts and bounded history.
   Enqueue alert jobs in the same transaction that completes rule jobs, local actions
   and existing IMAP outbox entries. Reuse [03.5] eligibility/frontiers and account-bound
   identity tombstones; retain compact automatic-delivery deduplication markers until
   account/destination deletion, independently of pruned history. Persist selected-field
   payload snapshots only in SQLCipher, with bounded size; never store destination
   secrets there. Account/message deletion cancels related pending jobs and removes
   payloads. Completed/canceled jobs clear payloads; keep at most 30 days/1,000 safe
   history records per destination. No credentials, subjects, bodies, URLs or receiver
   response bodies enter errors/history. No network operation holds a SQLite lock.
4. `[03.6.4]` **Bounded execution and delivery semantics:** Resume leased work after
   restart through the existing worker; processing requires an active unlocked app
   or a future separately deployed headless runner. Use one in-flight attempt per
   destination, default at most six starts/minute/destination, persisted scheduling,
   a ten-second total attempt deadline and bounded concurrency across destinations.
   Retry transport failures, 408/429 and 5xx with jittered exponential backoff (30s
   initial, one-hour cap), capped at eight attempts/24 hours; honor bounded Retry-After.
   Treat redirects and other 4xx as terminal; only 2xx counts as receiver acceptance.
   Send stable versioned event IDs on every attempt for receiver deduplication. Crash
   and timeout ambiguity can cause duplicates, including OS notifications; promise
   neither exactly-once delivery nor proof that the user saw an accepted notification.
   Expired work fails visibly rather than producing an unbounded resume burst.
5. `[03.6.5]` **Payload and native notification controls:** Default local alerts to
   a generic new-mail notice without sender/subject/body on the lock screen; request
   OS permission only on explicit enablement and show denied/unavailable status.
   Webhook JSON defaults to schema version, opaque event ID and event timestamp.
   Independently opt in account identifier, sender, subject, bounded plain-text excerpt,
   extracted facts, attachment metadata or full plain-text body per destination.
   Never include raw HTML/headers, attachment bytes, OTP/Pix values or credentials
   implicitly; sensitive extraction/body export requires explicit disclosure. Bound
   payloads to 64 KiB with explicit truncation flags. Do not fetch missing bodies or
   attachments to enrich an alert. Simulation previews fields without delivering.
6. `[03.6.6]` **Additive IPC and EN/PT-BR settings:** Add typed destination CRUD,
   enable/disable, safe history, explicit synthetic test delivery and selected-message
   replay commands. Test requires an enabled/confirmed destination and visibly states
   it sends externally; replay requires selected messages/destinations and confirmation,
   creates a separate replay event ID and never invokes mail mutation actions. Preserve
   mounted inbox, revision checks and dirty/busy guards. Extend Rules settings with
   destination selection and side-effect-free explanations. Load `svelte-dev` before
   UI edits; keep Telegram/Discord setup, quiet hours/digests, reminders and a runnable
   headless CLI outside this task.
7. `[03.6.7]` **Evidence and closure:** Offline adapters and injected clocks/resolvers
   cover atomic rollback, encrypted reopen, lease expiry, retries/rate limits, ambiguous
   outcomes, disabled/edited/deleted destinations, vault/SQL partial failures, consent,
   field allowlists, history pruning, deduplication, manual replay isolation and initial
   history suppression. Cover TLS/DNS/redirect policy and secret-free failures without
   live outbound requests. Run all repository DoD commands, SPA build, formatting and
   tracked-file secret/PII scans. Record ADR, contracts and applicable knowledge cards;
   load `readme-standard` for paired usage documentation. Make an atomic Conventional
   Commit, log its hash and promote [03.7] to planning; do not push.

#### Approved P0 Exception (2026-11-02)

- Permit only user-enabled generic webhook JSON POST delivery, explicit synthetic tests
  and confirmed selected-message replay to that destination's vault-stored HTTPS URL.
  Background deliveries require prior destination/field consent and matching new-mail
  rules; no automatic destination discovery, browser requests or provider calls.
- Require verified TLS 1.2+ and public DNS hosts on port 443 in this iteration. Reject
  URL userinfo/fragments, IP literals, localhost and private/link-local/reserved/mixed
  DNS answer sets. Validate and pin the exact resolved public addresses used by each
  connection; revalidate each retry. Disable redirects and inherited proxies. Never
  forward secrets to another origin or log URLs/HTTP response content. Bound DNS,
  connection, request/response processing and payload sizes.
- Private-network, loopback and plain-HTTP webhook destinations are unsupported,
  including locally hosted n8n. Public HTTPS user-owned n8n remains supported. Any
  private-network exception requires a separately approved address/consent design.
- Approval authorizes implementation and amendment of AGENTS.md with this narrow
  exception. It does not authorize live test deliveries from the development session;
  validation remains offline. Runtime delivery always requires the user's configured
  destination consent. No other P0 exception or telemetry is introduced.

- **Approval:** User approved the complete plan and public-HTTPS-only P0 exception via
  `proceed` on 2026-11-02. Implementation is complete; development validation stayed offline.

#### Completion (2026-11-02)

- **Commit:** `0a76794` (`feat(alerts): add consent-bound notifications and webhooks`).
- **Delivered:** Additive rule destination actions and SQLCipher migration v4, ingestion-time
  destination revision snapshots, atomic rule/delivery enqueue, durable leases/rate limits,
  bounded retry/history, selected-field payloads, vault-only immutable URL/auth revisions
  and orphan cleanup. Additive typed IPC and EN/PT-BR Alerts settings provide CRUD, explicit
  origin/field consent, synthetic tests, confirmed selected-message replay and safe history.
  Linux generic notifications use D-Bus; rule simulation never sends or mutates mail.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (246 Rust tests).
  Desktop check has zero errors/warnings; all 140 frontend tests and the SPA build pass.
  Changed frontend files pass Prettier; all five policy regressions pass. Tracked/new-file
  scan covers 286 text files with zero findings; Svelte invariant and diff checks pass.
- **Environment:** Existing k-001/k-006 toolchain/native sysroot; cached chrono/zbus only,
  no dependency download or system modification. Existing loopback test fixtures and policy
  child-process tests ran outside restrictive sandbox. No live notifications, external
  webhook requests, provider credentials or model downloads were used during validation.
- **Contracts/documentation:** Approved narrow P0 exception is in AGENTS.md. ADR-013,
  active notes, k-026 and paired README guides document payload/privacy limits, vault/SQL
  recovery, destination revision consent and delivery ambiguity. Existing IPC, traits and
  prior migrations remain compatible.
- **Limits:** Requires an active unlocked runner; no runnable headless CLI is added.
  Local notifications currently require the Linux desktop notification service; other
  platforms report unavailable. Webhooks support public DNS HTTPS/443 only. Retry/timeout
  ambiguity can produce duplicates; acceptance is not proof of human visibility. Plain-text
  bodies and metadata are bounded with explicit truncation. Message/account removal cancels
  pending jobs, including cache deletion after MOVE without COPYUID. No live-OS visibility
  or receiver interoperability claim is made from offline adapter tests.
- **Next:** [03.7] promoted to READY FOR PLANNING; Telegram/Discord implementation requires
  its own approved plan. No subsequent task implementation has started.

### Task [03.7]: Telegram & Discord Alert Connectors

- **Status:** DONE
- **Priority:** 3
- **Depends On:** [03.6] and its approved network-policy exception.
- **Systems Involved:** Delivery adapters, `crates/vault`, desktop IPC/UI.
- **Scope:** Add guided Telegram bot/chat and Discord webhook setup using the shared
  rule engine, payload controls, queue and execution history. Include explicit test
  delivery, destination selection and revocation/disable controls.
- **Acceptance:** Synthetic offline connector tests cover formatting, escaping,
  provider size/rate limits, retryable failures and secret redaction. Tokens and
  secret webhook URLs never appear in logs, exports or returned configuration DTOs.

#### Approved Implementation Plan (2026-11-02)

1. `[03.7.1]` **Additive connector contracts:** Extend destination kinds with Telegram
   and Discord, preserving existing Local/Webhook wire formats, migration history,
   rule actions and delivery traits. Keep formatting and provider outcome parsing pure
   and headless. Reuse destination revisions, ingestion snapshots, field consent,
   durable queue, retry/age budgets, test/replay and safe execution history. Add an
   append-only migration only if required by the existing storage representation.
2. `[03.7.2]` **Vault-only setup:** Add input-only, redacted, zeroized connector secret
   types and backward-compatible vault decoding. Telegram stores a user-supplied bot
   token and chat ID only in vault; Discord stores the complete webhook URL there.
   Safe DTOs expose kind, fixed public origin and credential-presence flags only.
   Bind enablement to the provider, recipient configuration revision and selected fields;
   changing kind/recipient/credentials requires fresh confirmation and cancels pending
   revisions. Reuse immutable secret staging, orphan recovery and deletion cleanup.
3. `[03.7.3]` **Restricted delivery:** Telegram POSTs only to its fixed HTTPS Bot API
   sendMessage endpoint; Discord POSTs only to validated discord.com API webhook paths.
   Reject alternate hosts, userinfo, fragments, arbitrary queries and injected path
   segments; allow only explicitly constructed execution options. Preserve public-only
   connection-time DNS validation, TLS 1.2+, port 443, no redirects/proxies and bounded
   deadlines. No bot discovery, getUpdates, webhook inspection or credential-validation
   calls occur during setup/save. All development validation uses offline adapters.
4. `[03.7.4]` **Safe provider payloads/outcomes:** Render only the shared consent-filtered
   payload. Telegram uses plain text without parse_mode, with link previews disabled;
   Discord suppresses mentions and embeds. Bound Unicode text conservatively per
   provider, with visible truncation; send one message per event without attachment
   uploads or automatic splitting. Parse only bounded provider response envelopes for
   acceptance/error codes and retry delays; discard descriptions, message/chat objects
   and raw responses. Combine bounded provider/HTTP retry hints with existing persisted
   rate gates and attempt/age caps. Malformed responses fail visibly. Do not claim
   provider idempotency, exactly-once delivery or human visibility.
5. `[03.7.5]` **Guided EN/PT-BR setup:** Extend Alerts settings with Telegram bot/chat
   and Discord webhook instructions and detached secret inputs, clear external-export
   disclosure, recipient confirmation, selected-field preview, explicit synthetic test,
   replay and disable/delete controls. Existing Rules destination selection includes
   both kinds. No setup/test sends automatically; retain dirty/busy/unmount guards,
   stale-response protection and the mounted inbox. Load svelte-dev before UI edits.
   Provider-side revocation is guided manually; disabling/deleting stops local dispatch
   but cannot recall accepted messages or revoke a shared provider credential remotely.
6. `[03.7.6]` **Evidence and closure:** Offline tests cover formatting/escaping, Unicode
   bounds, mention/link suppression, endpoint validation, malformed/oversized responses,
   provider retries, consent/revision races, vault redaction/recovery, upgrades and
   existing generic/local behavior. Run the full repository DoD, frontend build,
   applicable formatting and secret/PII scans. Record ADR/contracts and knowledge cards;
   use readme-standard for paired usage documentation. Make an atomic Conventional
   Commit, log its hash and promote [04.10] to planning. Do not push.

#### Approved P0 Extension (2026-11-02)

- Extend [03.6]'s runtime exception to user-enabled Telegram sendMessage JSON POSTs
  at api.telegram.org and Discord webhook JSON POSTs at discord.com, including explicit
  synthetic tests and confirmed replay. Bot token, chat ID and full webhook URLs remain
  vault-only. Per-destination field and recipient consent is mandatory.
- Permit bounded, transient provider-response parsing for acceptance and retry metadata;
  never log, persist or return response bodies or provider descriptions. Keep all existing
  public-DNS/TLS/443/deadline/no-redirect/no-proxy restrictions. No discovery/polling,
  inbound bot listener, provider credential revocation calls or live development delivery.
- Approval authorizes this narrow AGENTS.md amendment and implementation only. Digests,
  quiet hours, reminders and a runnable headless CLI remain separately scoped.

- **Approval:** User approved this plan and narrow P0 extension via `proceed` on
  2026-11-02. Implementation and development validation remain offline.

#### Completion (2026-11-02)

- **Commit:** `ce25af0` (`feat(alerts): add Telegram and Discord connectors`).
- **Delivered:** Additive provider kinds, typed Telegram input, vault-only token/chat ID
  and webhook material, fixed provider routes, consent-filtered bounded text, provider
  response classification and durable shared provider cooldowns. Existing rules,
  destination revision consent, queue, test/replay and safe history are reused.
  EN/PT-BR guided settings include manual recipient confirmation and credential
  revocation instructions. No schema migration or existing trait/command change.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (253 Rust tests).
  Desktop check reports zero errors/warnings; all 143 frontend tests and SPA build pass.
  Changed frontend files pass Prettier; all five policy regressions pass. Tracked/new-file
  scan covers 288 text files with zero findings. Svelte invariant, paired README checks
  and whitespace checks pass. Regressions include provider envelopes/Unicode limits,
  endpoint/DNS rejection, secret redaction, consent/revision lifecycle, encrypted reopen,
  provider cooldown persistence and stale-completion isolation.
- **Environment:** Existing k-001/k-006 toolchains and GTK/WebKit sysroot; cached formatter.
  Workspace loopback fixtures and policy child-process checks ran outside the restrictive
  sandbox. No downloads, system changes, live provider requests or external deliveries.
- **Contracts/documentation:** Approved P0 extension is recorded in AGENTS.md. ADR-014,
  active notes, k-027 and paired README guides document provider routing, vault envelope
  compatibility, field consent, response bounds and cooldowns. Same-kind-only preservation
  prevents carrying secrets into another connector type; edited revisions cancel pending work.
- **Limits:** Numeric Telegram chat IDs and canonical Discord webhook URLs only; no chat
  discovery/migration, threads or attachments. One message per event can visibly truncate
  JSON text. Provider retries conservatively pause all configured destinations of that
  kind. Runtime must be active/unlocked; duplicates remain possible and acceptance does
  not prove visibility. Revocation is manual in BotFather/Discord; accepted messages cannot
  be recalled. Live provider interoperability remains unverified by offline tests.
- **Next:** [04.10] promoted to READY FOR PLANNING. Saved searches/virtual folders require
  a separately approved plan; no subsequent implementation has started. Nothing pushed.

### Task [04.10]: Saved Searches & Virtual Folders

- **Status:** DONE
- **Priority:** 4
- **Systems Involved:** `crates/storage`, desktop search IPC/sidebar.
- **Scope:** Save named queries and scopes spanning selected accounts without moving
  messages. Provide views such as unread clients, invoices and upcoming bills using
  supported structured predicates. Keep saved definitions in encrypted storage.
- **Acceptance:** Create/rename/delete views; refresh membership after sync or triage;
  preserve pagination, account isolation and canonical message identity. Validate
  full-result membership rather than filtering only the loaded frontend page.


#### Approved Implementation Plan (2026-11-02)

1. `[04.10.1]` **Typed saved-view definitions:** Add a versioned headless storage
   model with opaque ID, revision, display name, query text/search mode, account
   selection and structured predicates. Default to lexical search and all accounts;
   an explicit nonempty account selection spans only those accounts. Support optional
   read/starred/attachment states, exact local tags, parsed sender mailbox/domain,
   deterministic extraction kind and boleto due-date bounds. Predicates combine with
   AND; account selections combine with OR. Missing facts never satisfy positive
   predicates. Sender identity is unauthenticated. Provide a relative upcoming-bills
   window (today through the next 30 days, inclusive, UTC calendar dates), resolved
   once per request; date and amount facts must not imply payment status. Do not add
   arbitrary SQL, a query language, rules/actions, schedules or outbound requests.
2. `[04.10.2]` **Encrypted persistence:** Add append-only migration v5 for saved
   definitions and optimistic revisions; leave v1-v4 intact. Validate bounded names,
   text, predicate collections, date ranges and account references natively. Support
   create/update/rename/delete and deterministic listing. Never store definitions or
   mail facts in browser persistence. Removed account references must never broaden
   a selected-account view: retain the explicit selection and return a visible invalid
   view until edited, while allowing unrelated definitions to be listed and managed.
3. `[04.10.3]` **Full-result native execution:** Compile validated predicates into
   parameterized SQL eligibility before LIMIT/OFFSET and before both hybrid candidate
   windows. Reuse existing lexical ranking, exact cosine semantics, RRF behavior and
   hydration; preserve canonical account/folder/UID and deduplicate memberships. Run
   definition/revision validation, eligibility, ranking and hydration in one read
   snapshot with one clock value. Semantic/hybrid retain existing opt-in/model gates;
   save query text and mode, never embeddings. Keep HybridQuery, SearchEngine and
   existing mailbox/IPC shapes compatible through additive saved-view APIs. Existing
   performance deviations remain visible; no approximate or loaded-page filtering.
4. `[04.10.4]` **Additive desktop IPC:** Add typed list/save/delete/search-saved-view
   commands with revision checks, bounded pagination and fixed safe errors. Saved-view
   search resolves the persisted definition natively; unknown IDs, stale revisions and
   missing account references fail closed. CRUD never mutates messages or invokes
   rules/deliveries. Existing ordinary search and mailbox navigation remain compatible.
5. `[04.10.5]` **EN/PT-BR sidebar and editor:** Load svelte-dev before implementation.
   Add a Saved searches sidebar section and accessible create/edit/rename/delete
   controls, including Save current search. Offer editable starter definitions for
   unread clients (sender/tag plus unread), invoices (NF-e extraction) and upcoming
   bills (boleto with relative date window); never infer clients automatically or save
   presets without user action. Support account multi-selection and predicate controls.
   Reuse inbox virtualization, pagination and sync/triage refresh; selecting/editing a
   view resets reader/selection/scroll and invalidates late requests. Deleting the active
   view returns to All Mail. Refresh relative dates at UTC midnight and on focus/resume.
   Preserve modal dirty/busy guards and clear transient state on teardown.
6. `[04.10.6]` **Evidence and closure:** Test migration/reopen and revision conflicts,
   all/selected-account isolation and removed-account failures, parsed senders, missing
   extraction facts, UTC/date boundaries, full-result page unions/order, semantic and
   hybrid prefiltering, duplicate memberships and canonical identity. Cover sync/triage
   membership changes, stale async responses, active deletion and editor lifecycle.
   Use synthetic reserved-domain fixtures and offline adapters only. Run every DoD
   command plus SPA build, formatting and tracked-file secret/PII scans; measure
   representative large-fixture saved-view workloads without weakening existing targets.
   Record contracts/ADR and applicable knowledge cards; load readme-standard for paired
   usage documentation. Make an atomic Conventional Commit, log its hash and promote
   [04.11] to READY FOR PLANNING. Do not push or implement reminders/dashboard work.

- **Approval:** User approved the full plan via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `1e8ebca` (`feat(search): add encrypted saved searches and virtual folders`).
- **Delivered:** Versioned encrypted definitions, append-only migration v5, revision-safe
  CRUD/deletion tombstones, all/selected accounts and optional existing mailbox scopes;
  typed read/star/attachment/tag/sender/extraction/boleto date predicates before native
  lexical/semantic/hybrid ranking and pagination. Canonical identity and existing public
  search/IPC contracts remain intact. Additive saved-view IPC retains semantic consent.
  EN/PT-BR sidebar editor, Save current search, editable starter drafts and active-view
  lifecycle reuse inbox virtualization, sync/triage refresh and UTC-midnight/focus updates.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (260 Rust tests).
  Desktop check reports zero errors/warnings; all 148 frontend tests and SPA build pass.
  Changed frontend files pass Prettier; five policy guard regressions and a 299-text-file
  tracked/new-file secret/PII scan pass with zero findings. Legacy syntax, README perimeter
  and diff whitespace checks pass. Required socket/child-process fixtures ran outside the
  restrictive sandbox; all validation remained offline without live providers or downloads.
- **Performance:** Six complete-membership/identity workloads on 100k messages/200k
  memberships, 180 warm samples: p95 19.111-21.812ms blank lexical, 10.178-13.331ms
  semantic, 25.249-28.801ms hybrid. These narrow starred populations meet the applicable
  ADR-008 budgets but do not establish broad parsed-fact/native UI performance. Existing
  mailbox deviations/targets remain unchanged. Reproduction/limits: docs/performance/saved-views.md.
- **Contracts:** ADR-015, active notes, k-028 and paired usage guides record missing-account
  fail-closed behavior, same-boleto UTC date bounds, revision guards and deferred draft
  lifecycle. Definitions never enter browser persistence. Views do not move messages,
  authenticate senders, infer clients/payment status, schedule reminders or deliver alerts.
- **Next:** [04.11] promoted to READY FOR PLANNING; implementation needs its own approved
  plan. No reminder/dashboard work started. Nothing pushed.

### Task [04.11]: Due-Date Dashboard & Scheduled Reminders

- **Status:** DONE
- **Priority:** 5
- **Depends On:** [03.5], [03.6]; [03.7] for Telegram/Discord destinations.
- **Systems Involved:** Existing deterministic extractors, `crates/storage`, runtime
  scheduling, desktop dashboard and rule UI.
- **Scope:** Display extracted boleto/billing amounts and due dates. Trigger configurable
  reminders before deadlines and when matching messages remain unread for a chosen
  interval. Support local and enabled external destinations with amount thresholds.
- **Acceptance:** Deterministic clock tests cover time zones, date-only deadlines,
  suspend/resume, restarts, duplicate suppression, read-before-deadline cancellation
  and explicit dismissal. Extraction never implies a bill was paid. Missed-reminder
  behavior is visible; no always-on delivery claim when all runners are stopped.

#### Completion (2026-11-02)

- **Commit:** `25fa521` (`feat(reminders): add due-date dashboard and scheduled reminders`).
- **Validation:** `cargo check --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings. `cargo fmt --all -- --check` passed. Rust core tests in `vault`, `mail-engine`, `triage-ai`, and `storage` (including new deterministic civil date math, migration v6 idempotency, duplicate suppression, unread cancellation, and explicit dismissal) passed 100% (237+ tests). Frontend checks: `pnpm --filter desktop check` (0 errors, 0 warnings), `pnpm --filter desktop test` (26 test files, 159 tests passed), `pnpm --filter desktop build` (SPA production build passed). Zero legacy Svelte syntax (`! grep -rn "export let" apps/desktop/src/`). `git diff --check` passed with 0 errors.
- **Contracts:** Schema migration v6 added `reminder_configs`, `reminder_config_deleted_ids`, `reminder_jobs`, `due_date_dismissals`. Additive Tauri IPC commands: `cmd_list_due_date_items`, `cmd_dismiss_due_date_item`, `cmd_list_reminder_configs`, `cmd_save_reminder_config`, `cmd_delete_reminder_config`. Pure Gregorian civil date calculations using Howard Hinnant's algorithm. Non-payment disclaimer invariant enforced in both EN and PT-BR.
- **Documentation:** Synchronized `README.md`, `README.pt-br.md`, and `.agent/NOTES.md`.
- **Next:** `[03.8] Local Automatic Categories` promoted to `READY FOR PLANNING`.

#### Proposed Implementation Plan (2026-11-02)

1. `[04.11.1]` **Pure Domain Models & Civil Date Math (`crates/triage-ai` / `crates/storage`):**
   - Define typed models for `DueDateItem` (`message_id`, `account_id`, `subject`, `sender`, `received_at_epoch_ms`, `is_read`, `digitable_line`, `amount_cents`, `due_date_iso`, `is_dismissed`).
   - Define status categorization (`Overdue`, `DueToday`, `Upcoming`, `Dismissed`) relative to calendar reference dates.
   - Define typed `ScheduledReminderConfig` (`DueDate` with `days_before`, `target_hour`, `min_amount_cents`; `UnreadInterval` with `interval_hours`, `min_amount_cents`; `destination_ids`, `account_id`).
   - Pure civil date utilities with deterministic calendar arithmetic: leap years, month boundaries, timezone offsets, and target epoch calculation.
   - Deterministic clock unit tests covering time zones, date-only deadlines, positive/negative UTC offsets, and leap years.

2. `[04.11.2]` **Encrypted Persistence & Migration v6 (`crates/storage`):**
   - Add append-only migration v6 for `reminder_configs`, `reminder_config_deleted_ids`, `reminder_jobs`, and `due_date_dismissals`.
   - Unique index on `(message_id, trigger_type, due_epoch_ms)` for guaranteed duplicate suppression.
   - CRUD for `reminder_configs` with optimistic revision checking.
   - Due-date queries: `list_due_date_items` returning extracted boletos with amounts and due dates, left-joined with dismissals, ordered by due date ascending.
   - Explicit dismissal: `dismiss_due_date_item` marking pending reminder jobs as `Dismissed` and persisting dismissal.
   - Read-before-deadline cancellation: `cancel_reminders_for_read_messages` updating pending unread reminder jobs to `Canceled`.

3. `[04.11.3]` **Runtime Scheduler & Alert Dispatching (`apps/desktop/src-tauri`):**
   - Integrate reminder processing into `outbox_loop`: `service.process_reminders()`.
   - Evaluate due reminder jobs: verify unread/dismissed status; enqueue deliveries into `alert_jobs` targeting the configured `destination_ids` (reusing existing vault, retry, rate limit, and notification/webhook transports).
   - Visible missed-reminder behavior: annotate delayed reminders (> 15 min past due from sleeping/stopped runners) with a clear delayed notification note.
   - Sync & triage hooks: schedule reminder jobs upon message ingestion for matching configs; cancel unread reminders immediately upon `MarkRead` triage mutations.

4. `[04.11.4]` **Additive IPC Commands & DTOs (`apps/desktop/src-tauri`):**
   - Add `cmd_list_due_date_items(accountId?)`, `cmd_dismiss_due_date_item(messageId)`, `cmd_list_reminder_configs()`, `cmd_save_reminder_config(config)`, `cmd_delete_reminder_config(id, revision)`.
   - Register commands in `tauri::generate_handler!` in both runtime and test builder.
   - Ensure zero credential leakage and bounded safe error variants.

5. `[04.11.5]` **Due-Date Dashboard & Reminders UI (`apps/desktop`):**
   - Due-Date Dashboard (`DueDateDashboard.svelte`): accessible from a new "Due Dates" / "Contas a Vencer" Sidebar item with an indicator badge for overdue/upcoming counts.
   - Metrics summary: Overdue, Due Today, Upcoming counts & totals formatted in BRL currency (`R$ 1.234,56`).
   - Filter tabs: All, Overdue, Due Today, Upcoming, Dismissed; account filter; search by sender/subject.
   - Boleto card: formatted amount, due date with relative badge ("Vence hoje", "Venceu há X dias", "Vence em X dias"), 1-click "Copiar código" (linha digitável copy to clipboard), "Ver e-mail" (navigate to message in reader), and "Dispensar lembrete".
   - Prominent non-payment disclaimer in EN and PT-BR: "Extracted from message receipts. Does not constitute banking confirmation of payment."
   - Reminders Configuration (`RemindersModule.svelte` in Settings and accessible from Dashboard): configure due-date and unread reminder rules, amount thresholds, and alert destinations.
   - Svelte 5 Runes strictly enforced, full bilingual EN / PT-BR parity in `translations.ts`.

6. `[04.11.6]` **Comprehensive Verification & Definition of Done:**
   - Pure clock unit tests (timezone offsets, civil date math, leap years, DST transitions).
   - Storage tests: migration v6 idempotence, duplicate suppression, restart recovery, explicit dismissal, read cancellation.
   - IPC service tests and frontend component tests (`*.svelte.test.ts`).
   - Repository DoD commands: `cargo check`, `cargo test`, `cargo fmt`, `cargo clippy -D warnings`, `pnpm --filter desktop check`, `pnpm --filter desktop test`, `pnpm --filter desktop build`, Svelte 5 invariant, secret/PII scrub, and bilingual README perimeter synchronization.


### Task [03.8]: Local Automatic Categories

- **Status:** DONE
- **Priority:** 6
- **Depends On:** [03.5]
- **Systems Involved:** `crates/triage-ai`, `crates/storage`, desktop inbox.
- **Scope:** Classify people, finance, purchases, newsletters and notifications locally,
  starting with deterministic rules. Allow corrections and sender-specific overrides.
  Expose category views without changing canonical IMAP folders.
- **Acceptance:** Synthetic fixtures cover classification and override precedence;
  re-sync preserves corrections. No cloud endpoint or model download is required.
  Learned classification remains separately scoped behind explicit model consent.

#### Approved Implementation Plan (2026-11-02)

1. `[03.8.1]` **Pure deterministic classification:** Extend the existing classifier
   in `triage-ai`, preserving Security, Financial, Shopping, Newsletter and Personal
   wire values and adding Notifications. UI labels map to Security, Finance, Purchases,
   Newsletters, People and Notifications. Define automatic precedence as Security >
   Financial > Shopping > Newsletter > Notifications > Personal fallback. Use bounded
   local facts, existing extracted badges and explicit EN/PT-BR contextual signals;
   expose fixed reason codes and classifier version, never probabilistic confidence.
   Personal is a fallback, not evidence of a human or authenticated sender. Preserve
   extraction badges independently of category corrections. No model or network calls.
2. `[03.8.2]` **Encrypted correction persistence:** Add append-only migration v7
   for classifier metadata, per-message corrections and account-scoped exact-sender
   overrides with optimistic revisions. Effective precedence is explicit message
   correction > sender override > automatic result. Resetting a correction falls back
   to the next layer. Reuse the existing parsed single-mailbox normalization; malformed
   or ambiguous senders cannot create sender overrides, and display names never match.
   Sender overrides affect existing and future cached mail in that account only.
   Store all metadata in SQLCipher, with no browser persistence or IMAP writes.
3. `[03.8.3]` **Ingestion and lifecycle:** Keep automatic results separate from
   corrections so sync upserts, flag changes, body hydration and classifier upgrades
   cannot overwrite user choices. Preserve corrections across duplicate memberships,
   restart and canonical MOVE. For cache deletion/re-ingestion, use account-bound
   provider identity, then Message-ID plus timestamp; retain correction identities
   until account removal. Without a reliable identity, retain only canonical-record
   corrections and disclose that limitation rather than transfer a correction to an
   unrelated message. Backfill automatic classifications for existing cached mail in
   bounded resumable local batches without fetching bodies/attachments, running rules,
   scheduling reminders or delivering alerts. Recheck identity/revision on batch writes.
4. `[03.8.4]` **Native search and additive IPC:** Add typed category metadata,
   correction/reset and sender-override list/save/delete commands with bounded inputs,
   revision checks and fixed errors. Add category search through the existing private
   SQL eligibility/ranking machinery, preserving established search/trait/scope shapes.
   Apply effective category before pagination and both hybrid candidate windows in
   one snapshot; preserve exact cosine/tie behavior and canonical IMAP identity.
   Add an optional effective-category predicate to saved-view filters with a compatible
   default. Existing saved definitions remain valid; semantic consent gates still apply.
5. `[03.8.5]` **EN/PT-BR inbox controls:** Load svelte-dev before UI edits. Add local
   category views using existing virtualized rows, account scope and search modes.
   Show localized effective category and automatic/manual/sender provenance. Provide
   explicit separate actions for correcting a selected message, resetting it, and
   applying/removing an account-specific sender override; explain its existing/future
   scope before saving. Keep extraction/security badges visible regardless of category.
   Provide override management in settings. Refresh membership after changes and sync;
   invalidate stale requests and preserve modal guards, selection and teardown safety.
6. `[03.8.6]` **Evidence and closure:** Cover mixed signals, missing facts, precedence,
   malformed/display-name senders, account isolation, revision conflicts, reset behavior,
   migration/reopen, interrupted backfill, re-sync/MOVE and correction identity limits.
   Verify full-result category page membership, semantic/hybrid eligibility, saved-view
   compatibility, and stale UI responses using synthetic offline fixtures. Run the full
   repository DoD plus SPA build, formatting and secret/PII scans. Measure representative
   large-fixture category queries without weakening existing performance targets or
   hiding known deviations. Update active contracts/ADR and applicable knowledge cards;
   load readme-standard for paired usage documentation. Commit atomically, log the hash
   and promote [04.12] to READY FOR PLANNING. Do not push or begin new-sender review.

- **Approval:** User approved the recorded scope via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `cf81280` (`feat(categories): add local classification and durable corrections`).
- **Delivered:** Six deterministic local categories with fixed reasons/version, additive
  Notifications wire value, encrypted append-only migration v7, revision-safe message
  corrections and exact account/sender overrides. Message > sender > automatic precedence,
  reset tombstones, canonical MOVE/re-ingestion identity handling, bounded resumable
  history backfill and independent extraction badges. Additive native category search,
  saved-view category predicates, EN/PT-BR controls and settings removal management retain
  existing IPC shapes, core traits, mailbox identity and full-result pagination.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (279 Rust tests).
  Desktop check reports zero errors/warnings; all 163 frontend tests and SPA build pass.
  Changed frontend files pass Prettier. Five policy regressions and a 324-text-file
  tracked/new-file secret/PII scan pass with zero findings. Legacy syntax and diff checks
  pass. Socket/child-process fixtures ran in the authorized local execution context;
  all validation stayed offline without provider calls, downloads or deliveries.
- **Performance:** 100k messages / 200k memberships, six workloads / 180 warm samples.
  Local backfill tight-loop time is 4,721.357ms; desktop yields between <=300-row batches.
  Broad Finance semantic p95 262.892ms and hybrid p95 451.471ms miss existing budgets.
  Narrow category+starred semantic/hybrid and blank lexical workloads meet applicable
  fixture budgets. Targets remain unchanged; existing mailbox/native deviations remain.
  Evidence and limits: `docs/performance/categories.md` and aggregate CSV. No native
  first-frame, 60fps or arbitrary-workload latency claim is made.
- **Contracts:** ADR-017, active notes, k-029 and paired usage guides document precedence,
  initial-history convergence, unauthenticated sender identity, Personal fallback and
  record-only correction limits. Message-ID bracket normalization matches MIME ingestion.
  Corrections never train models, mutate IMAP folders or execute rules/reminders/alerts.
- **Next:** [04.12] promoted to READY FOR PLANNING. Its sender-history seeding and review
  semantics need their own approved plan; no implementation started. Nothing pushed.

### Task [04.12]: New-Sender Review

- **Status:** DONE
- **Priority:** 7
- **Systems Involved:** `crates/storage`, desktop inbox/settings.
- **Scope:** Offer an optional local review view for previously unseen senders, with
  accept/decline controls and per-account preferences; reuse rule actions where useful.
- **Acceptance:** Define normalized sender identity and initial-history seeding during
  planning. Persist review choices across re-sync; never auto-delete messages. Clearly
  distinguish local inbox filtering from blocking delivery at the provider, and never
  present a familiar sender address as proof of authenticated identity.

#### Proposed Implementation Plan (2026-11-02)

1. `[04.12.1]` **Sender identity and explicit local choices:** Reuse
   `triage_ai::rules::sender_address`: exactly one parsed mailbox, ASCII-lowercased
   address, account-bound exact matching, no display-name/domain matching or provider
   alias folding. Model Known (history), Pending, Accepted and Declined separately;
   Known is not user acceptance. Accept/decline applies to existing and future cached
   messages from that exact sender in that account, with reversible revision-safe
   choices. Malformed/missing/ambiguous senders remain visible with an unavailable
   identity explanation and cannot receive sender-wide decisions. Address familiarity
   never implies authentication. No automatic archive, trash, unsubscribe or rule action.
2. `[04.12.2]` **Encrypted persistence and conservative history seeding:** Add
   append-only migration v8 for per-account preferences, revisioned sender choices,
   normalized message membership and resumable seeding/frontiers. Default disabled;
   enabling seeds cached history locally in bounded batches before activating review.
   Capture per-folder UIDVALIDITY/UIDNEXT frontiers for first post-enable synchronization;
   initial/history imports, newly discovered folders and rebuild history seed Known
   rather than flood Pending. Persist frontiers before ingestion and retain them across
   interrupted batches; missing UIDNEXT treats arrivals conservatively as history until
   a usable frontier is established. New canonical messages above established frontiers
   create Pending only for unseen valid senders. History never overwrites an existing
   Pending/Accepted/Declined choice. Show initialization status and explain conservative
   seeding, including arrivals during initial setup and unobserved pre-enable mail.
3. `[04.12.3]` **Ingestion, restart and preference lifecycle:** Update sender state
   atomically with ingestion/checkpoints, preserving choices across duplicate memberships,
   re-sync, canonical MOVE, body hydration, restart and cache deletion/re-ingestion.
   Choices persist until account removal; account deletion cleans all related state.
   Disabled accounts never hide mail or create Pending; newly observed senders while
   disabled seed Known. Re-enabling seeds intervening history without erasing existing
   decisions or pending reviews. Initialization/backfill never fetches missing data,
   mutates IMAP, runs rules, schedules reminders or delivers alerts. Keep storage
   headless and existing repository/observer/IPC contracts compatible.
4. `[04.12.4]` **Native views and additive IPC:** Add bounded typed preference,
   sender-list/decision and review-search commands with native account validation,
   optimistic revision checks and fixed errors. Provide Pending and Declined review
   views. Default enabled mode is review-only; a separate explicit per-account option
   hides Pending/Declined senders from Inbox views only. Known/Accepted and unresolved
   identities remain visible. All Mail, other mailboxes, category views, saved searches
   and explicit review views retain access to those messages. Apply review membership
   and any Inbox exclusion before pagination and both hybrid candidate windows in one
   read snapshot, preserving exact ranking and semantic consent. Disabling immediately
   removes Inbox exclusions. Existing rules/reminders/alerts continue independently;
   review is not an automation or provider-delivery gate.
5. `[04.12.5]` **EN/PT-BR desktop controls:** Load svelte-dev before UI edits.
   Add New Senders navigation using existing virtualized rows, account scope and search
   modes, with Pending/Declined filters and message access. Accept/decline confirms the
   account and exact sender scope; allow changing either decision and returning to
   Pending. Add per-account enablement, initialization status, optional Inbox filtering
   and paginated decision management in Settings. Explain local visibility, independent
   automation and unauthenticated sender identity beside the relevant controls. Preserve
   extraction badges, secure rendering, modal dirty/busy guards, request invalidation,
   selection/reader refresh and teardown safety. No browser persistence of mail facts.
6. `[04.12.6]` **Verification and closure:** Cover parsing/case/account isolation,
   history/frontier races, interrupted seed/restart, unknown UIDNEXT, UIDVALIDITY reset,
   duplicate membership/MOVE, decisions surviving re-ingestion, stale writes and
   disable/re-enable. Verify native complete-result membership/pagination for lexical,
   semantic and hybrid modes, Inbox-only exclusion, accessible declined mail, and stale
   UI responses. Use synthetic reserved-domain fixtures and offline adapters. Run the
   full repository DoD plus SPA build, changed-file formatting and secret/PII scans;
   record representative large-fixture timings without weakening existing budgets or
   claiming native 60fps. Update active contracts, ADR and applicable knowledge cards;
   load readme-standard for paired usage docs. Commit atomically, log its hash and
   promote [04.13] to READY FOR PLANNING. Do not push or implement conversation grouping.

- **Approval:** User approved the recorded plan and defaults via `proceed` on 2026-11-02.

#### Completion (2026-11-02)

- **Commit:** `dd74e10` (`feat(review): add durable local sender review and inbox filtering`).
- **Delivered:** Default-off, account-specific local sender review with exact parsed
  addresses, separate Known/Pending/Accepted/Declined states and reversible revision-safe
  decisions. Append-only migration v8, conservative persistent history frontiers,
  bounded resumable initialization and atomic ingestion preserve choices through restart,
  re-sync, duplicate memberships, canonical MOVE and cache deletion/re-ingestion.
  Additive IPC/native search, EN/PT-BR Pending/Declined navigation, explicit sender/account
  confirmation and guarded per-account settings retain existing traits and DTO contracts.
- **Visibility:** Optional Inbox-only exclusion; explicit All Mail, folders, categories,
  saved searches and review retain access. Unresolved identities remain visible. Nothing
  deletes/moves mail, blocks provider delivery, authenticates a sender or gates independent
  rules/reminders/alerts. No network requests, downloads or attachment fetches were added.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass (285 Rust tests).
  Desktop check has zero errors/warnings; all 167 frontend tests and SPA build pass.
  Changed frontend files pass Prettier with existing tabs/single quotes/100-column style.
  Five policy regressions and a 340-text-file tracked/new-file scan pass with zero findings.
  Legacy Svelte syntax, README perimeter and diff whitespace checks pass. Existing
  loopback/child-process fixtures ran in the authorized local execution context; all
  validation remained offline. Existing native sysroot/toolchain from k-001/k-006 reused.
- **Performance:** Shared 100k-message/200k-membership fixture, six workloads/180 warm
  samples. Tight-loop sender backfill: 4,115.029ms; desktop yields between <=300-row
  batches. Blank lexical p95: 0.772ms global / 38.857ms account, within 40ms. Semantic
  p95: 390.544ms global / 133.568ms account; hybrid p95: 524.779ms / 197.102ms. Existing
  semantic/hybrid targets remain unmet and unchanged. Evidence/measurement limits are
  in `docs/performance/sender-review.md` and aggregate CSV; no native frame/60fps claim.
- **Contracts:** ADR-018, active notes, k-030 and paired README usage guides document
  conservative initial-window/unknown-frontier seeding, history versus acceptance,
  unauthenticated exact-sender identity, disabled/re-enable behavior and visibility.
- **Next:** [04.13] promoted to READY FOR PLANNING. Conversation identity, fallback
  grouping and multi-message action semantics require a separately approved plan.
  No conversation-grouping implementation started. Nothing pushed.

### Task [04.13]: Conversation Grouping & Thread Triage

- **Status:** DONE
- **Priority:** 8
- **Systems Involved:** MIME/header ingestion, `crates/storage`, desktop inbox/reader.
- **Scope:** Group related messages using Message-ID, In-Reply-To and References;
  define safe fallback behavior for missing/malformed headers and account boundaries.
  Offer conversation-level read/archive/tag actions with explicit affected-message scope.
- **Acceptance:** Fixtures cover duplicate IDs, malformed references, subject collisions
  and cross-folder messages. Preserve per-message IMAP identity, lazy attachments,
  pagination and secure rendering; handle partial mutation failures visibly.

#### Approved Implementation Plan (2026-11-02)

1. `[04.13.1]` **Conservative account-local identity:** Build a bounded, iterative
   header-threading implementation in the headless core using stored Message-ID,
   In-Reply-To and References. Preserve case-sensitive opaque IDs after syntactic
   normalization; reject malformed/oversized values. Never group by subject, sender,
   provider aliases or across accounts. Messages without usable links remain singletons.
   Unique valid In-Reply-To takes precedence; otherwise use the nearest unambiguous
   References ancestor. Shared missing ancestors may connect replies through account-local
   placeholder nodes. Duplicate Message-IDs are ambiguous: retain every canonical record,
   do not merge duplicates by that ID or resolve inbound links through it. Discard
   self-links, break cycles deterministically and bound reference processing. Grouping
   is navigation, never authentication or a reason to deduplicate/delete mail. Harden
   the existing public tree helper without changing its signature or losing records
   from the new canonical-record conversation API.
2. `[04.13.2]` **Encrypted membership and lifecycle:** Append migration v9 for indexed
   account-local conversation membership and opaque generation/revision metadata;
   preserve earlier schemas, message/IMAP identity and public trait contracts. Build
   existing cached history in bounded resumable local batches, with no fetches or
   automation. Keep grouping unavailable for an account until its initial index is
   complete, displaying progress and retaining ordinary message navigation. Update
   affected membership on ingestion, header changes and deletion; recompute affected
   components when missing parents arrive or duplicate ambiguity changes. MOVE and
   additional folder memberships must not duplicate a canonical record. Account removal
   clears its index. Stale conversation handles fail and refresh rather than target a
   different set after a merge/split. No durable user setting depends on a stable root ID.
3. `[04.13.3]` **Native grouped search and pagination:** Add separate typed conversation
   search/member APIs; leave current message DTOs, commands, search traits and default
   message mode intact. Provide an opt-in Conversations toggle. Apply existing query,
   mailbox/account, saved-view, category and sender-review eligibility before grouping
   and pagination, preserving semantic consent and ranking. Rank groups by their best
   eligible message with deterministic ties; return each conversation once per result
   snapshot, with matching/total cached counts and aggregate unread state. Never group
   only the already-paginated frontend rows. Use generation-checked pagination and
   refresh on invalidation. Conversation expansion explicitly labels other cached
   messages in the same account, including members outside the active filter/folder;
   ordinary Inbox hiding must not be silently bypassed in result eligibility.
4. `[04.13.4]` **Explicit snapshot actions:** Offer conversation Mark Read/Unread,
   Archive and Tag only. Preview the exact cached canonical records, account, folders,
   matching versus outside-view counts, and exclusions before confirmation. Default
   action scope to matching messages; separately select all cached conversation members.
   Bind confirmation to the native membership generation, selected IDs and action;
   reject stale previews without applying any local changes. Reuse existing mutation,
   reminder-cancellation and durable outbox behavior, with atomic local updates and
   independent remote retries. Keep the existing 500-record batch limit: oversized
   selections require narrowing, never silent truncation or partial chunk submission.
   New arrivals after preview are excluded and require a fresh preview. Surface pending
   and failed affected-message outcomes and retry status without promising atomic remote
   IMAP success. No thread unsubscribe, trash, rule replay or implicit sender action.
5. `[04.13.5]` **Inbox and reader integration:** Load `svelte-dev` before component
   edits. Add EN/PT-BR grouped virtual rows, counts, keyboard selection and a paginated
   chronological member list with per-message expansion. Fetch bodies only when opened
   through existing lazy APIs; retain DOMPurify, sandbox/CSP and attachment-on-demand
   boundaries. Preserve individual-message controls and focus. Invalidate late reads,
   pagination and action previews on view/account/query/generation changes and teardown.
   Keep mail facts and conversation state out of browser persistence. Do not change
   automatic read behavior into an implicit whole-conversation action.
6. `[04.13.6]` **Evidence and closure:** Offline synthetic fixtures cover malformed and
   missing headers, duplicate IDs appearing/disappearing, case differences, shared absent
   parents, cycles, long chains, subject collisions, cross-account/folder boundaries,
   late parents, MOVE/deletion, encrypted restart/backfill and stale generation rejection.
   Test filtering before group pagination, matching/all-member action scopes, the batch
   limit, local rollback, remote partial failures, lazy body/attachment access and UI
   invalidation. Measure grouped query/index costs on representative large fixtures;
   retain existing performance budgets and disclose unmet targets. Run repository DoD,
   SPA build, formatting and secret/PII scans offline. Record ADR-019, active contracts,
   relevant knowledge and paired README usage via `readme-standard`. Make an atomic
   Conventional Commit, log its hash and promote [03.9] to READY FOR PLANNING; do not push.

- **Approval:** User approved the complete plan via `proceed` on 2026-11-02.
  No new network-policy exception; development validation remains offline.

#### Completion (2026-11-03)

- **Commit:** `c9b3253` (`feat(conversations): add local grouping and snapshot triage`).
- **Delivered:** Account-local header grouping with conservative ID validation, ambiguous
  duplicate handling, missing-ancestor placeholders and iterative cycle-safe components;
  append-only SQLCipher migration v9, resumable header staging and account readiness.
  Additive native grouped search/member/preview/apply/status APIs preserve message DTOs,
  canonical IMAP identities, existing traits and message-mode search. Optional EN/PT-BR
  grouped virtual rows, chronological paginated members and lazy sandboxed readers expose
  matching/cached/unread counts and explicitly label outside-view messages.
- **Actions:** Read/unread/archive/tag previews default to matching cached records;
  all-cached requires separate selection. Immutable native IDs/action, five-minute expiry,
  generation validation and one-use confirmation consumption prevent scope substitution
  and stale applies, including headless callers. Local updates, unread-reminder cancellation
  and outbox enqueue commit atomically. Per-message attempts/failures expose independent
  remote retries and partial success. >500 records fail without truncation. No automatic
  whole-thread read, unsubscribe/trash, history replay or attachment download is added.
- **Validation:** Offline workspace check/test/fmt/all-target Clippy pass: 295 Rust tests.
  Desktop check reports zero errors/warnings; 171 frontend tests and SPA build pass.
  Changed frontend files pass Prettier; all five policy regressions pass. Canonical scan
  covers 358 tracked text files with zero findings; Svelte invariant/diff checks pass.
  Fixtures cover encrypted restart/backfill, malformed/duplicate/cyclic headers, long
  chains, account/folder boundaries, saved/category/review/semantic eligibility, pagination,
  stale/forged previews, >500 selections, atomic rollback and partial remote failure.
- **Environment:** A session reset removed the temporary native sysroot and validation
  artifacts. User explicitly authorized isolated Linux build-package downloads on
  2026-11-03; restored dependencies live only in ignored project scratch, with no system
  package installation. Existing loopback fixtures/policy child-process tests ran outside
  the restrictive sandbox. No live provider requests, credentials, model downloads or
  external deliveries were used. Development runtime/delivery validation remained offline.
- **Contracts/documentation:** ADR-019, active notes, k-031 and paired README usage record
  identity, lifecycle, complete-branch grouped hybrid ranking, action scope and limitations.
  No new runtime network-policy exception, crate dependency or public trait change.
- **Performance limits:** Disposable encrypted 100k-message/four-account release fixture:
  staging/publication 15,463.904ms over 337 ticks; slowest tick 297.980ms. Grouped p95
  global/account-Inbox: lexical 215.069/175.117ms, exact semantic 1029.961/378.748ms,
  hybrid 1170.017/455.225ms. These exceed unchanged existing search budgets. Final
  consolidation is account-wide under the database lock; late parents/duplicates/deletion
  may require rebuilding a dirty account. Complete eligible branch rankings and metadata
  scans scale with cached mail. Aggregate evidence/limits are in
  `docs/performance/conversations.md` and CSV; no native 60fps claim is made.
- **Next:** [03.9] promoted to READY FOR PLANNING. Digests, quiet-hour scheduling,
  missed-window behavior and urgency precedence require a separately approved plan.
  No digest/quiet-hour implementation started. Nothing pushed.

### Task [03.9]: Alert Digests & Quiet Hours

- **Status:** DONE
- **Priority:** 9
- **Depends On:** [03.6], [04.11]; [03.7] for Telegram/Discord destinations.
- **Systems Involved:** Rule scheduler/delivery queue, `crates/storage`, desktop settings.
- **Scope:** Batch routine alerts into scheduled summaries, configure quiet hours per
  destination and allow explicit urgent-rule exceptions. Bound summary payloads and
  apply the same user-selected export fields as individual alerts.
- **Acceptance:** Clock tests cover time zones, overnight windows, suspend/resume,
  restart and digest duplication. Avoid a burst of deferred notifications on resume;
  define missed-window and urgency precedence behavior during planning.

#### Approved Implementation Plan (2026-11-03)

1. `[03.9.1]` **Additive scheduling contracts:** Keep existing destinations, rule
   actions, IPC and delivery traits compatible. Add a separate revision-checked
   per-destination scheduling policy: Immediate (default), Hourly (selected minute),
   or Daily (selected hour/minute), optional daily quiet interval, and explicit
   permission for urgent-rule bypass. Scheduling uses either System Local or UTC;
   arbitrary named-zone selection is outside this task. Existing destinations retain
   immediate delivery with no quiet hours. Pure headless calculations take injected
   time/calendar resolution; the desktop supplies OS-local timezone resolution.
2. `[03.9.2]` **Time and urgency semantics:** Quiet hours use start-inclusive,
   end-exclusive civil times, including overnight windows; equal endpoints are
   rejected. Spring-forward missing slots run at the first valid instant after the
   gap; repeated fall-back slots run once, at the earlier occurrence. Durable slot
   identities prevent repeated hours/restarts/clock rollback from creating another
   summary. Recompute future unsealed slots on OS timezone changes without replaying
   closed slots. Add an explicit urgent alert action, leaving Alert(String) unchanged.
   Urgent bypass requires both that action and destination opt-in; it bypasses digest
   and quiet hours, never consent, rate limits, retry gates or expiration. When both
   ordinary and urgent actions target one destination, an eligible urgent action wins
   once, preserving rule order/stop semantics. Reminders are routine and respect the
   destination policy; urgency is never inferred from subjects, badges or amounts.
3. `[03.9.3]` **Encrypted queue and missed windows:** Add append-only migration v10
   for policies, bounded pending items, sealed summary membership and slot identities.
   Enqueue in the existing rule/reminder transaction with destination and policy
   revisions. Retain per-message/account linkage for deletion and reminder cancellation.
   At a due slot, atomically seal one summary and enqueue through existing alert_jobs;
   retries reuse its immutable payload/event ID. During quiet hours defer routine work
   until quiet end. Immediate-mode quiet deferrals are also coalesced. On resume, merge
   all missed eligible slots into at most one catch-up summary per destination; never
   replay individual missed windows. Keep the existing 24-hour event retention bound:
   older unsent items expire visibly, and retry age never resets when summarizing.
   Cap pending items at 1,000/destination with safe overflow counts, not extra deliveries.
   Process bounded batches without holding SQL locks during network I/O. Existing
   eight-attempt limits, provider cooldowns and globally serial dispatch remain.
4. `[03.9.4]` **Privacy and lifecycle:** Serialize summaries only from existing
   consent-filtered snapshots; export no new mail fields and fetch nothing. Bound each
   summary to 64 KiB and 50 detailed entries with explicit total/omitted/truncated
   counts. Preserve provider text ceilings and visible truncation; local notices remain
   generic. Account/message removal and unread-reminder cancellation remove affected
   pending items; invalidate unsent sealed summaries containing removed items, without
   resending surviving entries automatically. Policy/destination edits or disable/delete
   cancel prior pending revisions; re-enable never replays them. Serialize configuration
   with dispatch and retain existing in-flight recall limits. Clear terminal payloads
   and bound safe history; compact slot/dedup markers cannot resurrect expired work.
5. `[03.9.5]` **IPC and EN/PT-BR settings:** Add typed policy read/save and side-effect-free
   schedule preview commands. Alerts settings provides delivery cadence, quiet times,
   displayed timezone/next eligible delivery, urgent-bypass consent and safe deferred/
   summarized/expired counts. Rules expose explicit urgent actions and simulation
   explanations. Explicit test/replay remains immediate only after confirmation clearly
   discloses bypassing scheduling; it still obeys consent and transport limits and
   does not flush pending automatic work. Preserve stale-revision checks, dirty/busy
   guards, mounted inbox and input cleanup. Load svelte-dev before UI implementation.
6. `[03.9.6]` **Evidence and closure:** Offline injected-clock/calendar/adapter tests
   cover UTC/local offsets, DST gaps/repeats, overnight boundaries, timezone changes,
   rollback, suspend/restart, catch-up coalescing, concurrent sealing, retry identity,
   urgency precedence, reminders, cancellation, encrypted reopen, upgrades, consent
   field bounds and overflow. No live provider requests or dependency downloads.
   Run the repository DoD, SPA build, relevant formatting and tracked-file secret/PII
   scans. Record ADR-020, active contracts and applicable knowledge cards; load
   readme-standard for paired documentation. Make an atomic Conventional Commit,
   log its hash and promote [01.4] to READY FOR PLANNING. Do not push.

- **Approval:** User approved via `proceed` on 2026-11-03, including System Local/UTC,
  24-hour expiration, coalesced catch-up and dual-consent urgency. No additional
  network-policy exception was requested.

#### Completion (2026-11-03)

- **Commit:** `be65f15` (`feat(alerts): add consent-bound digests and quiet hours`).
- **Validation:** Workspace check, fmt and all-target Clippy (`-D warnings`) exit 0. Core
  crate tests and desktop crate tests pass (33 + 27 desktop tests). Desktop svelte-check
  reports 0 errors/0 warnings; all 174 frontend tests and the SPA build pass. Policy scan
  reports zero findings; Svelte `export let` invariant and `git diff --check` pass.
- **Environment:** Desktop linking used a disposable temp sysroot (GTK/WebKit runtime
  packages extracted with `dpkg-deb`, user-authorized for build tooling only; no system
  install). No live provider requests, deliveries or model downloads.
- **Gap:** Prettier is not installed in the local dependency tree, so the formatter check
  on changed frontend files was NOT run in this session; run it before release.
- **Contracts/documentation:** ADR-020, active notes, k-032 and paired README guides.
- **Next:** [01.4] promoted to READY FOR PLANNING; no implementation started. Nothing pushed.

### Task [01.4]: Encrypted Backup & Portable Rule Export

- **Status:** DONE
- **Priority:** 10
- **Depends On:** [03.5], [04.10] for rule and saved-view formats.
- **Systems Involved:** `crates/storage`, `crates/vault`, desktop import/export UI.
- **Scope:** Export/import portable rules and saved views without credentials. Provide
  encrypted local backup/restore for supported settings and locally cached mail data;
  version the format and define a consistent SQLite/WAL snapshot and recovery process.
- **Acceptance:** Round-trip, wrong-passphrase, corruption and version-compatibility
  checks use synthetic fixtures. Credentials, database keys and secret destination URLs
  are excluded from portable exports; restored integrations require reconnection and
  remain disabled until confirmed. Restore never silently overwrites an existing vault
  or database. Clearly state that unfetched attachment binaries are absent from backup.

#### Approved Plan (2026-11-04; user approved with `proceed`)

1. Define versioned, strictly validated portable-definition and encrypted-backup
   formats in the existing headless crates; record the recovery contract in an ADR.
   Preserve existing migrations, traits and IPC contracts with additive APIs only.
2. Export rules and saved views as bounded JSON with explicit account/destination
   placeholders. Preview imports, require explicit reference mapping, allocate fresh
   IDs/revisions, and import rules disabled in one transaction. Missing references
   must never broaden account scope; imports never evaluate historical mail.
   Disclose that user-authored names, queries and predicates can contain private text.
3. Produce a consistent SQLite/WAL read snapshot into a fresh encrypted artifact,
   copying only allowlisted supported data. Use vault-owned Argon2id key derivation
   and authenticated version metadata; keep source database keys, vault contents,
   credentials and secret integration URLs out of the artifact. Never write a
   plaintext database staging file or copy the live database file without its WAL.
   Validate the SQLCipher snapshot mechanism offline before building the UI.
4. Include cached message bodies/headers, attachment metadata, mailbox membership,
   tags/read/star/snooze state, rules, saved views, category corrections, sender-review
   choices, dismissals and supported non-secret preferences. Rebuild derived search
   and conversation indexes as needed. Exclude model files, external attachment files,
   active authorization sessions and pending mutation/rule/reminder/alert/digest work.
   Restored accounts require reconnection; rules, reminders, destinations and AI/network
   permissions remain disabled until explicitly configured/confirmed. No restore replay.
5. Restore only through a startup recovery flow before workers start, into a new
   private staging directory and fresh vault/database identity. Authenticate and
   validate versions, integrity and references before publication. Refuse existing
   target data/vaults, symlinks and path traversal; interruption leaves the original
   installation intact and incomplete recovery retryable. No in-place merge restore.
6. Add native file-selection/import-preview APIs and EN/PT-BR Settings/startup UI
   with passphrase confirmation, dirty/busy guards and transient secret cleanup.
   Explain included data, omitted unfetched attachment binaries, reconnection and
   lack of passphrase recovery. No browser persistence or network validation.
7. Verify synthetic round-trips, WAL-only committed data, concurrent-write snapshot
   consistency, wrong passphrase, tampering/truncation, unsupported versions, invalid
   references, import rollback, occupied targets and interrupted recovery. Prove no
   credentials or pending deliveries survive restoration. Run the full repository
   DoD, frontend formatting/build and policy scans; document contracts/gotchas,
   create one conventional commit, then log its hash. Do not push.

- **Approval boundary:** This plan proposes fresh-target recovery and the explicit
  supported-data perimeter above. Implementation authorized by the user. Execute only [01.4]; halt for unapproved dependencies or
  material changes to the recovery/security contract.

#### Completion (2026-11-04)

- **Commit:** `8f2fbab` (`feat(backup): add encrypted recovery and portable definitions`).
- **Delivered:** Headless authenticated streaming backups from one consistent committed
  SQLite/WAL snapshot; bounded credential-free portable rule/view JSON, full native
  preview/reference mapping and atomic disabled imports; startup-only isolated-vault
  fresh-target restore with a fresh database key, integrity/reference checks and inert
  staging recovery. Cached-only accounts reconnect by exact email without replacing
  connected accounts. EN/PT-BR controls disclose content, omissions and recovery limits.
  Imported saved views reload from native storage without browser persistence.
- **Validation:** `cargo check --workspace`, `cargo test --workspace`,
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `pnpm --filter desktop check`, `pnpm --filter desktop test` (181 tests), and
  `pnpm --filter desktop build` passed. Typecheck reports zero errors/warnings.
  Changed frontend files pass Prettier with repository tab/single-quote conventions.
  All five policy-scanner regressions pass; the full 383 tracked-text-file secret/PII/
  legacy-Svelte scan reports zero findings. Legacy-prop and diff checks pass.
- **Coverage:** Wrong passphrase, tampering/truncation/order/version rejection, WAL-only
  commits/concurrent-write consistency, queue/secret-reference exclusion, disabled
  restored automation, occupied/symlink/traversal targets, failed-stage cleanup and
  reopened fresh vault/database, account reconnect, reference rejection/import rollback,
  changed-preview/consent guards, credential cleanup, teardown and independent form guards.
- **Environment:** Offline validation used the existing toolchain/sysroot from k-001/k-006.
  Existing synthetic loopback Rust tests and Node policy subprocess checks ran outside
  the restrictive sandbox. No dependency downloads, provider calls, model downloads or
  real credentials were used. Native desktop-portal interoperability and large-backup
  responsiveness were not established; the implementation/documentation makes no claim.
- **Contracts:** ADR-021, k-033 and NOTES record format, limits, restore/reconnect and
  empty-host cached-only account semantics. Existing migrations, traits and command
  signatures remain unchanged; commands are additive. README EN/PT-BR stays synchronized.
- **Next:** No prioritized task follows [01.4]. Future backlog requires selection and
  planning; no unapproved task was started. Nothing pushed.

#### Closing Checklist — [01.4]

- [x] Rust/frontend strict typing, automated tests, formatting and Clippy.
- [x] Svelte 5 invariant, full tracked-file secret/PII scan and clean diff.
- [x] SPA build, EN/PT-BR usage documentation and supported-data disclosure.
- [x] ADR-021, active contracts and k-033 knowledge preservation.
- [x] Atomic conventional feature commit; hash recorded above.
- [x] Task closed; no prioritized successor available to promote.

---

## Other Future Backlog / Ideas (Unprioritized)

- Offline cached-mail CLI [00.3], offline CLI container [00.4] and shared sync foundation [00.5] completed; active headless sync/daemon and writable container deployment remain unprioritized
- Outbound SMTP & lightweight Markdown/Plain-text reply composer (`v0.2+`)
- Multi-folder sync (Archive/All Mail with Gmail dedup, Sent read-only): scoped into the proposed [04.1] mailbox queue; implementation pending approval
- Real Tier 2 ONNX inference via `ort`: completed in [03.10].
- Coverage-guided fuzzing for the in-house IMAP parser: completed in [02.7].
- Optional binary-quantized ANN candidate stage + exact int8 rerank (`v0.2`)
