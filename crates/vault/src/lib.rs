#![forbid(unsafe_code)]

//! OS Keyring integration & Argon2id/AES-GCM fallback vault contracts.
//!
//! INVARIANT: Never serialize or return raw secrets/tokens across IPC to the frontend UI.
//! All credentials (App Passwords, OAuth2 tokens, user-supplied BYO OAuth `client_id`/`client_secret`)
//! reside strictly inside the secret store and are referenced by opaque identifiers.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors returned by secret storage and master cipher operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VaultError {
    #[error("secret not found for key `{0}`")]
    NotFound(String),
    #[error("keyring backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("cryptographic operation failed: {0}")]
    CryptoError(String),
    #[error("invalid secret key identifier")]
    InvalidKey,
}

/// Opaque identifier referencing a secret stored in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VaultKey(String);

impl VaultKey {
    /// Creates a validated `VaultKey` for a given account and secret kind.
    pub fn for_account(account_id: &str, kind: &str) -> Result<Self, VaultError> {
        let trimmed_account = account_id.trim();
        let trimmed_kind = kind.trim();
        if trimmed_account.is_empty() || trimmed_kind.is_empty() {
            return Err(VaultError::InvalidKey);
        }
        Ok(Self(format!(
            "triage-mail:{trimmed_account}:{trimmed_kind}"
        )))
    }

    /// Returns the canonical key namespace string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Transparent multi-mode authentication material stored in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountAuthMaterial {
    /// Standard IMAP password or provider-specific App Password.
    AppPassword { secret: String },
    /// Public OAuth2 PKCE tokens (`client_id` only, no secret required).
    OAuth2PublicPkce {
        client_id: String,
        access_token: String,
        refresh_token: Option<String>,
    },
    /// User-supplied Bring-Your-Own OAuth2 credentials (`client_id` + `client_secret`).
    OAuth2ByoCredentials {
        client_id: String,
        client_secret: String,
        access_token: String,
        refresh_token: Option<String>,
    },
    /// Local Proton Mail Bridge loopback password (`127.0.0.1`).
    ProtonBridgeLocal { bridge_password: String },
}

/// Agnostic secret storage contract implemented by OS Keyring and Argon2id fallback vault.
pub trait SecretStore: Send + Sync {
    /// Stores raw secret bytes under the specified `VaultKey`.
    fn set_secret(&self, key: &VaultKey, secret: &[u8]) -> Result<(), VaultError>;

    /// Retrieves raw secret bytes for the specified `VaultKey`.
    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError>;

    /// Deletes a secret from the store.
    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError>;
}

/// Master cipher contract for local Argon2id + AEAD (`aes-gcm` / `chacha20poly1305`) fallback encryption.
pub trait MasterCipher: Send + Sync {
    /// Encrypts plaintext bytes using the derived master key.
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, VaultError>;

    /// Decrypts ciphertext bytes using the derived master key.
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, VaultError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_canonical_vault_key_and_rejects_empty_segments() {
        let key = VaultKey::for_account("acct-01", "imap-auth").expect("valid vault key");
        assert_eq!(key.as_str(), "triage-mail:acct-01:imap-auth");

        assert_eq!(
            VaultKey::for_account("   ", "imap-auth"),
            Err(VaultError::InvalidKey)
        );
        assert_eq!(
            VaultKey::for_account("acct-01", ""),
            Err(VaultError::InvalidKey)
        );
    }
}
