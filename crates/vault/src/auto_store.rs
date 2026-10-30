use std::sync::Arc;

use crate::{KeyringStore, SecretStore, VaultError, VaultKey};

/// Composite secret store that automatically uses the OS Keyring when available
/// and gracefully delegates to an encrypted fallback vault in headless/non-desktop environments.
pub struct AutoSecretStore {
    keyring: KeyringStore,
    fallback: Arc<dyn SecretStore>,
}

impl AutoSecretStore {
    /// Constructs an `AutoSecretStore` using the default `triage-mail` keyring service and given fallback store.
    pub fn new(fallback: Arc<dyn SecretStore>) -> Self {
        Self {
            keyring: KeyringStore::new(),
            fallback,
        }
    }

    /// Constructs an `AutoSecretStore` with a custom keyring service name.
    pub fn with_service(service: impl Into<String>, fallback: Arc<dyn SecretStore>) -> Self {
        Self {
            keyring: KeyringStore::with_service(service),
            fallback,
        }
    }
}

impl SecretStore for AutoSecretStore {
    fn set_secret(&self, key: &VaultKey, secret: &[u8]) -> Result<(), VaultError> {
        match self.keyring.set_secret(key, secret) {
            Ok(()) => {
                // If storing in OS Keyring succeeded, clean up any old secret in fallback
                let _ = self.fallback.delete_secret(key);
                Ok(())
            }
            Err(VaultError::BackendUnavailable(_)) => self.fallback.set_secret(key, secret),
            Err(other) => Err(other),
        }
    }

    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError> {
        match self.keyring.get_secret(key) {
            Ok(secret) => Ok(secret),
            Err(VaultError::BackendUnavailable(_)) => self.fallback.get_secret(key),
            Err(VaultError::NotFound(_)) => {
                // Fallback check in case the secret was saved when keyring was previously unavailable
                self.fallback.get_secret(key)
            }
            Err(other) => Err(other),
        }
    }

    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError> {
        let keyring_result = self.keyring.delete_secret(key);
        let fallback_result = self.fallback.delete_secret(key);

        match (keyring_result, fallback_result) {
            (Ok(()), _) | (_, Ok(())) => Ok(()),
            (Err(VaultError::NotFound(msg)), Err(VaultError::NotFound(_))) => {
                Err(VaultError::NotFound(msg))
            }
            (Err(VaultError::BackendUnavailable(_)), res) => res,
            (res, _) => res,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Argon2AesGcmCipher, EncryptedFallbackStore, MasterCipher};
    use argon2::Params;

    fn fast_cipher() -> Arc<dyn MasterCipher> {
        let params = Params::new(1024, 1, 1, Some(32)).unwrap();
        Arc::new(Argon2AesGcmCipher::from_passphrase_with_params(
            b"test-password",
            params,
        ))
    }

    #[test]
    fn auto_store_roundtrip_with_fallback() {
        let fallback = Arc::new(EncryptedFallbackStore::in_memory(fast_cipher()));
        let store = AutoSecretStore::with_service("triage-mail-synthetic-test", fallback);

        let key = VaultKey::for_account("user@example.org", "password").unwrap();
        let secret = b"rfc2606_synthetic_secret";

        store
            .set_secret(&key, secret)
            .expect("set succeeds in auto store");
        let retrieved = store.get_secret(&key).expect("retrieves secret");
        assert_eq!(retrieved, secret);

        store.delete_secret(&key).expect("deletes secret");
        assert_eq!(
            store.get_secret(&key),
            Err(VaultError::NotFound(key.as_str().into()))
        );
    }
}
