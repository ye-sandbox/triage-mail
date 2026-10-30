use crate::{SecretStore, VaultError, VaultKey};

/// SecretStore implementation using the OS credential store (Keyring / Secret Service / Keychain).
pub struct KeyringStore {
    service: String,
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringStore {
    /// Creates a new `KeyringStore` with the default service name `triage-mail`.
    pub fn new() -> Self {
        Self::with_service("triage-mail")
    }

    /// Creates a new `KeyringStore` with a custom service name.
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &VaultKey) -> Result<keyring::Entry, VaultError> {
        keyring::Entry::new(&self.service, key.as_str()).map_err(|err| map_keyring_error(&err, key))
    }
}

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

impl SecretStore for KeyringStore {
    fn set_secret(&self, key: &VaultKey, secret: &[u8]) -> Result<(), VaultError> {
        let entry = self.entry(key)?;
        entry
            .set_secret(secret)
            .map_err(|err| map_keyring_error(&err, key))
    }

    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError> {
        let entry = self.entry(key)?;
        entry
            .get_secret()
            .map_err(|err| map_keyring_error(&err, key))
    }

    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError> {
        let entry = self.entry(key)?;
        entry
            .delete_credential()
            .map_err(|err| map_keyring_error(&err, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_missing_entry_to_not_found() {
        let store = KeyringStore::with_service("triage-mail-test");
        let key = VaultKey::for_account("acct-nonexistent", "auth").unwrap();
        let result = store.get_secret(&key);

        // In test environments without D-Bus secret service, it will either return
        // NotFound or BackendUnavailable. Both are acceptable valid states.
        match result {
            Err(VaultError::NotFound(_)) | Err(VaultError::BackendUnavailable(_)) => {}
            other => panic!("expected NotFound or BackendUnavailable, got {other:?}"),
        }
    }
}
