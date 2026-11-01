# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
