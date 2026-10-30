---
id: "k-003"
title: "Keyring Error Mapping and Argon2id/AES-GCM Envelope Invariants"
date: 2026-10-29
tags:
  - "vault"
  - "crypto"
  - "keyring"
  - "argon2"
  - "aes-gcm"
severity: "low"
affected_components:
  - "crates/vault"
symptoms:
  - "Keyring store unavailable on headless/SSH/CI environments"
  - "AES-GCM 0.11 hybrid-array type complexity"
---

# [k-003] Keyring Error Mapping and Argon2id/AES-GCM Envelope Invariants

## Problem / Symptom
1. `keyring v4` on Linux utilizes D-Bus Secret Service via `zbus`. When running inside headless environments (Docker, CI runners, SSH sessions, or headless WSL instances), the D-Bus Secret Service daemon is not running, causing calls to return `NoDefaultStore`, `NoStorageAccess`, `NotSupportedByStore`, or `PlatformFailure`.
2. RustCrypto `aes-gcm 0.11.x` introduces breaking changes with `hybrid-array` requiring complex generic array size bounds, whereas `aes-gcm 0.10.3` and `argon2 0.5.3` provide stable, zero-unsafe authenticated encryption and key derivation.

---

## Approved Solution
1. In `crates/vault/src/keyring_store.rs`, map all store initialization and access failures to `VaultError::BackendUnavailable`:
```rust
fn map_keyring_error(err: &keyring::Error, key: &VaultKey) -> VaultError {
    match err {
        keyring::Error::NoEntry => VaultError::NotFound(key.as_str().to_string()),
        keyring::Error::NoDefaultStore
        | keyring::Error::PlatformFailure(_)
        | keyring::Error::NoStorageAccess(_)
        | keyring::Error::NotSupportedByStore(_) => {
            VaultError::BackendUnavailable(format!("OS credential backend error: {err}"))
        }
        _ => VaultError::CryptoError(format!("OS credential store error: {err}")),
    }
}
```
2. Use `AutoSecretStore` to transparently fall back to `EncryptedFallbackStore` (Argon2id + AES-256-GCM) when `BackendUnavailable` is encountered.
3. Use the canonical envelope format for ciphertexts:
   - `TMV1`: Passphrase-derived with Argon2id salt: `[TMV1 (4B)][Salt (16B)][Nonce (12B)][Ciphertext + Tag]`
   - `TMK1`: Direct 256-bit symmetric key: `[TMK1 (4B)][Nonce (12B)][Ciphertext + Tag]`
4. Strictly implement custom `fmt::Debug` on `AccountAuthMaterial` to redact credentials and tokens (`"[REDACTED]"`).

---

## What NEVER to Do
- ❌ **Anti-Pattern:** Leaking raw access tokens or passwords across IPC or in error / debug strings.
- ❌ **Anti-Pattern:** Failing hard in headless environments without providing an encrypted fallback mechanism.
