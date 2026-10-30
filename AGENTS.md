# Agent Guidelines and Rules

You are the lead software engineer developing this project: **`triage-mail`**.

> **100% Local-First, Provider-Agnostic, Open-Source (AGPLv3) Email Aggregator & Triage Engine** built on a pure native stack (Rust Cargo Workspace + Tauri v2 + SvelteKit 5 SPA). Zero Electron, zero Python/Node backend sidecars, zero telemetry.

---

## ⚖️ Rule Precedence Hierarchy

When requirements or directives conflict, the agent MUST resolve them using the following priority:
1. **[P0] Security, Zero-Knowledge & Secrets Isolation:** NEVER expose or commit tokens, `client_secret`, test credentials, or personal email addresses in tracked files or unit tests (git history will transition from private to public under AGPLv3). User-provided OAuth credentials (`client_id`/`client_secret`) or App Passwords MUST be stored exclusively at runtime in `crates/vault` (`keyring` or Argon2id encrypted local vault). NEVER execute external network calls except to user-configured IMAP servers, explicit RFC 8058 One-Click Unsubscribe endpoints, on-demand embedding model downloads, or explicitly opted-in AI endpoints.
2. **[P1] Payload, IPC & Schema Invariants:** NEVER break established Tauri IPC contracts, SQLite/SQLCipher schemas, or Rust trait boundaries recorded in `.agent/NOTES.md`.
3. **[P2] Strict Typing & Memory Safety:** Rust crates `vault`, `mail-engine`, and `triage-ai` MUST compile with `#![forbid(unsafe_code)]`; `crates/storage` MUST compile with `#![deny(unsafe_code)]` with a single isolated `#[allow(unsafe_code)]` block strictly for `sqlite-vec` FFI registration (`sqlite3_auto_extension`). All crates MUST compile with zero `clippy` warnings (`-D warnings`). TypeScript/Svelte 5 code MUST pass `svelte-check` with zero unchecked `any` declarations and strictly use Svelte 5 Runes (`$props`, `$state`, `$derived`, `$effect`).
4. **[P3] Architectural Separation & Headless Modularity:** Core domain logic (`mail-engine`, `storage`, `vault`, `triage-ai`) MUST remain decoupled from `apps/desktop` so the Rust engine can operate headless (CLI/daemon) without GUI dependencies.
5. **[P4] Code Style & Complexity:** Functions SHOULD remain concise (typically ≤ 50 LOC) with low cognitive complexity; formatters (`cargo fmt`, `prettier`) MUST pass.

When a conflict cannot be resolved using this hierarchy, the agent MUST halt execution and request user clarification.

---

## Scope Constraints & Blast Radius

- **IN-SCOPE (MVP v0.1 — "Triage-First" Inbound Only):**
  - Multi-account inbound sync with maximum transparent access options for the user: App Passwords, OAuth2 Public PKCE (`127.0.0.1` loopback listener), user-provided BYO OAuth (`client_id`/`client_secret` stored in `vault`), standard IMAP + TLS 1.3 (`tokio-rustls`) behind an agnostic `ImapConnector` trait, and local Proton Mail Bridge.
  - Fast incremental sync (`UIDVALIDITY`, `UIDNEXT`, `CONDSTORE`/`MODSEQ`) + real-time background sync via IMAP `IDLE` + adaptive polling.
  - Unified Inbox with 60fps virtual scrolling for `100k+` messages.
  - Sub-15ms hybrid search combining configurable lexical SQLite `FTS5` (default `unicode61 remove_diacritics 2`, optional `trigram`) and semantic vector KNN (`sqlite-vec` with `int8` scalar quantization and pre-filtering) over full-database SQLCipher encryption.
  - Keyboard-driven triage (`Command Palette`): Read/Unread, Archive, Trash, Local Tags, RFC 8058 One-Click Unsubscribe (`List-Unsubscribe-Post`), and two-way IMAP flag synchronization.
  - 3-tier Triage & Extraction Pipeline: Tier 1 (Deterministic Rust regex/heuristics for 2FA/OTP, FEBRABAN Boletos with exact BRL value and due date extraction, Pix EMV/keys, NF-e/invoice metadata, Tracking codes, `List-Unsubscribe` + RFC 8058 One-Click), Tier 2 (Local CPU ONNX embeddings via `fastembed-rs`/`ort` with on-demand model caching in `$XDG_DATA_HOME/triage-mail/models`), Tier 3 (Optional local Ollama `http://127.0.0.1:11434` or BYOK Cloud LLM).
  - Safe rendering: Strict `DOMPurify` + isolated `<iframe sandbox="">` with zero script execution, remote tracking pixel blocking by default, and normalized base dark-mode CSS.
  - Lazy attachments: Sync only headers, flags, and text/HTML body; fetch binary attachments on-demand via `IMAP BODY.PEEK`.
- **OUT-OF-SCOPE (MVP v0.1):**
  - Outbound email composition/sending (SMTP), complex rich-text HTML composer, calendar (CalDAV), or contacts (CardDAV).

---

## Modular Context Triggers

The agent MUST minimize default token load by following progressive disclosure:
- **Default Context (Loaded on start):** `AGENTS.md`, `.agent/TASK.md`, `.agent/NOTES.md` (Active Contracts only).
- **Architectural Decisions (`docs/adr/decisions.md`):** MUST load when creating new crates, researching historical trade-offs, or changing system boundaries.
- **Domain Skills (`.agent/skills/<name>/SKILL.md` or global skills):** MUST load `svelte-dev` when authoring Svelte 5 components, and `readme-standard` when updating `README.md` / `README.pt-br.md`.
- **Knowledge Base (`.agent/knowledge/`):** MUST load relevant atomic cards when diagnosing obscure platform errors, SQLCipher/sqlite-vec linkage quirks, or IMAP provider gotchas.

---

## Execution Protocol

1. Read `AGENTS.md`, `.agent/TASK.md`, and `.agent/NOTES.md` before editing any files.
2. **Plan first:** Set `Status` in `.agent/TASK.md` to `PLANNING`; present plan; await approval; then set to `RUNNING`.
3. Work on exactly ONE active task at a time.
4. **Falsifiable Definition of Done (DoD):**
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

## Fail-Stop Protocol & Escalation Hierarchy (Circuit Breaker)

If an automated command (test, build, typecheck, lint) fails **2 consecutive times** with the same root cause:
1. The agent MUST STOP execution immediately.
2. The agent MUST NOT attempt unapproved speculative refactorings.
3. The agent MUST escalate to the user with a structured diagnostic block:
   ```yaml
   failure_stage: "test | typecheck | lint | build"
   error_signature: "exact error message"
   consecutive_failures: 2
   root_cause_analysis: "technical description"
   attempted_fixes:
     - "fix 1 description"
     - "fix 2 description"
   pending_decision: "question or proposed options for user"
   ```

---

## Task Numbering (`[XX.Y]`)

Format: `[Epic].[Sequence]` with two-digit epics. Subtasks: `[XX.Y.Z]`. Exactly **one** task active in `RUNNING` status. IDs are immutable within a release cycle. After Git tag: archive to `ARCHIVE.md`, restart at `[00.1]`/`[01.1]`, and update active task ID.

| Prefix | Phase | Focus |
| :---: | :--- | :--- |
| **`00.x`** | Bootstrap & Workspace | Cargo Workspace (`crates/*`), SvelteKit 5 + Tauri v2 scaffold, test/lint harnesses |
| **`01.x`** | Vault & Encrypted Storage | OS Keyring (`vault`), Full-DB SQLCipher + WAL + Configurable FTS5 + `sqlite-vec` (`storage`) |
| **`02.x`** | Mail Engine (Inbound) | Agnostic `ImapConnector`, Multi-Auth (App Passwords / OAuth2 PKCE / BYO-OAuth / Proton Bridge), MIME parser, `UIDVALIDITY`/`MODSEQ` & IDLE manager (`mail-engine`) |
| **`03.x`** | Triage & AI Pipeline | Tier 1 Regex (2FA, Pix, FEBRABAN Boleto value/due-date, NF-e, Tracking, RFC 8058 One-Click Unsubscribe), Tier 2 Local ONNX Embeddings (on-demand cache), Tier 3 LLM (`triage-ai`) |
| **`04.x`** | Tauri IPC & Desktop UI | Svelte 5 Virtualized Unified Inbox, Command Palette, Sandboxed HTML Viewer with base dark-mode CSS (`apps/desktop`) |
| **`90.x`** | Optimization | Lean binary footprint, memory optimization, query latency (<15ms) |
| **`99.x`** | Hardening & Release | Public transition audit, secret scrubbing verification, AGPLv3 compliance, release tag |

---

## Post-Release Hygiene (Trigger: Git tag on any phase)

When releasing `vX.Y.Z`:
1. **Archive:** Move completed log from `TASK.md` to `ARCHIVE.md` under `## [vX.Y.Z] - YYYY-MM-DD`.
2. **Consolidate:** Promote definitive architectural decisions to `docs/adr/decisions.md`; prune ephemeral scratch notes in `.agent/NOTES.md` preserving only active contracts.
3. **Perimeter:** Sync `.env.example`, `README.md`, and `README.pt-br.md` (following `readme-standard`) to the release tag.
4. **Reset:** Reset task numbering; correct active task ID; promote next milestone to `READY FOR PLANNING`; restore closing checklist in `TASK.md`.

---

## Stack & Repository Topology

- **OS / Shell:** Linux (`bash`).
- **License:** GNU AGPLv3 (`LICENSE`).
- **Architecture:** Modular Cargo Workspace (headless-capable Rust core crates) + Tauri v2 Desktop App (SvelteKit 5 SPA frontend).
- **Modules:**
  - `crates/vault`: Rust (2021/2024 edition), `keyring` (OS Secret Service / Keychain / Credential Manager), `argon2` (Argon2id), `aes-gcm` / `chacha20poly1305`.
  - `crates/storage`: Rust, `rusqlite` (bundled SQLCipher + FTS5) + `sqlite-vec` for KNN vector search, WAL mode migrations.
  - `crates/mail-engine`: Rust, `tokio`, `async-imap`, `tokio-rustls` (TLS 1.3), `oauth2` (loopback PKCE), `mail-parser` (RFC 5322 / MIME).
  - `crates/triage-ai`: Rust, deterministic extractors (Regex for OTP/2FA, Boletos/Pix, Tracking, `List-Unsubscribe`), local CPU embeddings (`fastembed` / `ort`), optional LLM client (`reqwest` for local Ollama or BYOK).
  - `apps/desktop`: SvelteKit 5 (`@sveltejs/adapter-static`, Runes, TypeScript strict, Tailwind CSS, `bits-ui`, `DOMPurify`) + `src-tauri` (Tauri v2 IPC commands & channels).

---

## Validation Commands

- **Rust Workspace:**
  - Check compilation: `cargo check --workspace` (Exit code MUST be 0)
  - Run tests: `cargo test --workspace` (Exit code MUST be 0)
  - Format check: `cargo fmt --all -- --check` (Exit code MUST be 0)
  - Linter: `cargo clippy --workspace --all-targets -- -D warnings` (Exit code MUST be 0)
- **Frontend (`apps/desktop`):**
  - Install dependencies: `pnpm install`
  - Typecheck: `pnpm --filter desktop check` (Exit code MUST be 0)
  - Run unit tests: `pnpm --filter desktop test` (Exit code MUST be 0)
  - Build SPA: `pnpm --filter desktop build` (Exit code MUST be 0)

---

## Skills

| Skill | Trigger |
| :--- | :--- |
| `svelte-dev` | Authoring or modifying Svelte 5 components, Runes state (`.svelte`, `.svelte.ts`), or UI primitives in `apps/desktop/src/**` |
| `readme-standard` | Updating `README.md` or `README.pt-br.md` |

---

## Golden Rules

- **MUST NOT** commit any secret, `client_secret`, test token, or personal email address in tracked files or unit tests (use RFC 2606 reserved domains like `@example.com` / `@example.org` in all tests and fixtures).
- **MUST NOT** download binary email attachments automatically during background sync; sync only headers, flags, and text/HTML body (`Lazy Attachments` via on-demand `IMAP BODY.PEEK`).
- **MUST NOT** render unsanitized email HTML or allow script/remote image execution by default; always sanitize with `DOMPurify` and isolate inside `<iframe sandbox="">` with CSP blocking remote tracking pixels unless explicitly unblocked by the user.
- **MUST NOT** introduce telemetry, analytics, or unsolicited outbound network requests.
- **MUST NOT** couple `crates/mail-engine`, `crates/storage`, `crates/vault`, or `crates/triage-ai` to Tauri-specific crates; keep core crates pure Rust so they can run headless in CLI/Docker.
- **MUST NOT** use legacy Svelte 4 syntax (`export let`, `$:`, `<slot />`, `on:click`); strictly enforce Svelte 5 Runes (`$props()`, `$state()`, `$derived()`, `$effect()`, `{#snippet}`).
- **MUST** couple future sustainability/extensibility features (e.g., E2EE Cloud Sync of rules/settings, Supporter Pack webhooks) via clean Rust `Trait` boundaries in core crates without imposing artificial account limits on the base local app.
- **MUST** document non-obvious domain logic constraints, ordering dependencies, protocol quirks, and safety invariants as in-code comments using explicit invariant prefixes (`// INVARIANT: <rationale>`).
- **MUST** write all agent-facing directives, skills, contracts, and task logs (`AGENTS.md`, `.agent/*`) in concise Technical English.

---

## Code Quality & Contrast Pairs

```rust
// BAD: Coupling core crate to Tauri runtime or syncing heavy attachments eagerly
pub async fn sync_folder(app_handle: tauri::AppHandle, session: &mut ImapSession) {
    let messages = session.fetch("1:*", "RFC822").await.unwrap(); // Fetches entire raw MIME including 25MB attachments!
    app_handle.emit("sync", messages.len()).unwrap();
}

// GOOD: Pure Rust trait/channel boundary and Lazy Attachment fetch (headers + flags + body structure only)
// INVARIANT: Never fetch full RFC822 in bulk sync; fetch ENVELOPE, FLAGS, UID, and text/html body parts only to keep RAM and SQLite lean
pub async fn sync_folder<S: SyncEventSink>(sink: &S, session: &mut ImapSession) -> Result<SyncSummary, EngineError> {
    let summaries = session.fetch_lightweight("1:*").await?;
    sink.on_batch_synced(&summaries).await?;
    Ok(SyncSummary { count: summaries.len() })
}
```

```svelte
<!-- BAD: Unsanitized HTML injection and legacy Svelte 4 syntax -->
<script lang="ts">
  export let htmlBody: string;
</script>
<div>{@html htmlBody}</div>

<!-- GOOD: Svelte 5 Runes + DOMPurify + sandboxed iframe with zero script execution and blocked remote images -->
<script lang="ts">
  import { sanitizeEmailHtml } from '$lib/security/sanitize';
  type Props = { rawHtml: string; allowRemoteImages?: boolean };
  let { rawHtml, allowRemoteImages = false }: Props = $props();
  const safeSrcDoc = $derived(sanitizeEmailHtml(rawHtml, { allowRemoteImages }));
</script>
<iframe
  title="Email Content"
  sandbox=""
  referrerpolicy="no-referrer"
  srcdoc={safeSrcDoc}
  class="w-full h-full border-0 bg-background"
></iframe>
```

---

## Git Conventions

- **Atomic Commits:** Each commit MUST represent a single logical change.
- **Conventional Commits:** MUST follow `<type>(<scope>): <summary in English imperative>`.
- **Branch Strategy:** Trunk-based on `main` or feature branches `feat|fix/<name>`.
- **Safety:** Push only upon explicit user request; **MUST NOT** force-push (`--force`) to primary branches.
