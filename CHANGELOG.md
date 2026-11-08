# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2026-11-08

### Fixed
- Windows installation writer ownership (`WriterOwnership`): resolved fail-closed `StorageUnavailable` defect on Windows application launch/upgrade by implementing safe Windows non-blocking exclusive file locking via `File::try_lock()`, volume identity verification via `volume_serial_number()` and `file_index()`, and metadata validation rejecting symlinks and reparse points (`crates/sync-runtime/src/ownership.rs`).

## [0.1.4] - 2026-11-08

### Added
- Headless background daemon (`triage-mail daemon`) and one-shot sync CLI (`triage-mail sync`) with periodic sync scheduler, signal drain on `SIGTERM`, and exclusive writer ownership (`WriterOwnership`).
- Writable and read-only container deployments (`packaging/cli`) with non-root security boundaries, sibling lockfile conventions, and outbound TLS verification.
- Local ONNX embeddings via `fastembed-rs` and `ort` with on-demand model caching, background vector indexing, and progress reporting.
- Consent-bound alerts and webhook dispatchers supporting generic JSON POST endpoints, Telegram Bot (`sendMessage`), and Discord webhooks with quiet hours schedules and digests.
- Local triage rules engine with condition-action evaluations, simulation dry-runs, automatic and manual classification categories, sender reviews, and conversation grouping.
- Encrypted saved searches, virtual folders, and encrypted backup recovery with portable JSON export/import.
- Coverage-guided fuzzing harnesses for IMAP `BODYSTRUCTURE` and `FETCH` response parsing, hardening literal parsing against malformed tokens.

## [0.1.3] - 2026-11-02

### Added
- Application Title Bar with themed desktop chrome, native window management (drag, minimize, maximize, restore, close), and persistent remaining-height layout.
- Unified Settings Modal with persistent mounted inbox shell, unsaved form guards, and modular panes (General, Accounts, Search, AI, Privacy) in English and Brazilian Portuguese.
- Account Mailboxes Navigation with expandable account folders, folder catalog discovery, same-account deduplication, and append-only Migration v2.
- Two-way Starred flag synchronization and local Snoozed mailbox triage with deadline filtering.
- Account-scoped lexical (FTS5) and semantic vector search with bounded query execution over SQLCipher.
- Private local account avatars with client-side thumbnail normalization (128x128), EXIF metadata scrubbing, and encrypted storage.

## [0.1.2] - 2026-11-01

### Added
- Step-by-step credential configuration playbooks and field checklists in `AccountHelpGuide.svelte` for all four sign-in modes (App Password for Gmail/iCloud/Yahoo/Fastmail, OAuth Public PKCE in Microsoft Entra/Azure, OAuth BYO Credentials in Google Cloud Console, and local Proton Mail Bridge) in both English (`en`) and Brazilian Portuguese (`pt-BR`).
- One-click `Gmail OAuth` preset in `presets.ts` and `AccountForm.svelte` to auto-populate `imap.gmail.com:993`, `OAuth2ByoCredentials`, Google OAuth 2.0 authorization/token endpoints, and the `https://mail.google.com/` scope.

## [0.1.1] - 2026-11-01

### Added
- Interactive in-app onboarding help guide (`AccountHelpGuide.svelte`) explaining App Password, OAuth Public PKCE, OAuth BYO Credentials, and Proton Mail Bridge with prerequisites, compatible services, and configuration tips.
- One-click provider presets for Gmail, Outlook, iCloud, Yahoo, Fastmail, and Proton to auto-fill host, port, and recommended auth mode.
- Minimalist Apple-style segmented pill language selector (`EN` | `PT`) with translucent backdrop blur, instant active response, and full accessibility support.
- Reactive Svelte 5 rune-based i18n store (`en` / `pt-BR`) with automatic browser language detection and `localStorage` persistence.
- Comprehensive Account Setup & Authentication Guide in both `README.md` and `README.pt-br.md` maintaining 1:1 bilingual parity.

## [0.1.0] - 2026-11-01

### Added
- Pure native desktop email engine built on Tauri v2, Rust Cargo workspace, and SvelteKit 5 SPA.
- Agnostic IMAP protocol engine (`mail-engine`) with two-phase lazy body fetch via RFC 3501 `BODYSTRUCTURE`, IDLE manager, and SASL XOAUTH2 token refresh.
- Multi-mode authentication: App Passwords, OAuth2 Public PKCE (`127.0.0.1` loopback callback), user-supplied BYO OAuth credentials, and Proton Mail Bridge local TLS loopback pinning.
- Single-file SQLCipher AES-256 storage (`storage`) with WAL mode, configurable FTS5 tokenizer, and `sqlite-vec` account-partitioned vector indexing.
- Parallel read pool executing lexical and semantic search concurrently via dual SQLite WAL connections (ADR-008).
- Tier 1 deterministic triage extractors for 2FA/OTP, Pix, FEBRABAN Boletos with exact BRL value and due date, NF-e metadata, Tracking numbers, and RFC 8058 One-Click Unsubscribe.
- Experimental Tier 2 feature-hash semantic projection search and optional Tier 3 LLM client for local Ollama and BYOK endpoints.
- Virtualized Unified Inbox with keyboard-driven Command Palette and DOMPurify-isolated sandboxed iframe message reader.
