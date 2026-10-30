use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{MasterCipher, SecretStore, VaultError, VaultKey};

enum StorageBackend {
    Memory,
    File(PathBuf),
}

/// Fallback encrypted secret store protected by a `MasterCipher`.
///
/// Operates either in-memory or persisted to disk with atomic file writes.
pub struct EncryptedFallbackStore {
    cipher: Arc<dyn MasterCipher>,
    backend: StorageBackend,
    cache: RwLock<HashMap<String, Vec<u8>>>,
}

impl EncryptedFallbackStore {
    /// Creates an in-memory encrypted fallback store.
    pub fn in_memory(cipher: Arc<dyn MasterCipher>) -> Self {
        Self {
            cipher,
            backend: StorageBackend::Memory,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Creates or opens an encrypted file-backed fallback store.
    pub fn file_backed(
        path: impl Into<PathBuf>,
        cipher: Arc<dyn MasterCipher>,
    ) -> Result<Self, VaultError> {
        let path = path.into();
        let mut map = HashMap::new();

        if path.exists() {
            let data = fs::read(&path).map_err(|err| {
                VaultError::Io(format!(
                    "failed reading vault file `{}`: {err}",
                    path.display()
                ))
            })?;
            if !data.is_empty() {
                map = serde_json::from_slice(&data).map_err(|err| {
                    VaultError::Serialization(format!(
                        "corrupted vault file `{}`: {err}",
                        path.display()
                    ))
                })?;
            }
        }

        Ok(Self {
            cipher,
            backend: StorageBackend::File(path),
            cache: RwLock::new(map),
        })
    }

    fn persist_if_file_backed(&self, map: &HashMap<String, Vec<u8>>) -> Result<(), VaultError> {
        if let StorageBackend::File(path) = &self.backend {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    VaultError::Io(format!(
                        "failed to create vault dir `{}`: {err}",
                        parent.display()
                    ))
                })?;
            }

            let serialized = serde_json::to_vec(map).map_err(|err| {
                VaultError::Serialization(format!("failed serializing vault: {err}"))
            })?;

            let tmp_path = format!("{}.tmp.{}", path.display(), std::process::id());
            let tmp_path = Path::new(&tmp_path);

            fs::write(tmp_path, serialized).map_err(|err| {
                VaultError::Io(format!(
                    "failed writing temp vault `{}`: {err}",
                    tmp_path.display()
                ))
            })?;

            fs::rename(tmp_path, path).map_err(|err| {
                VaultError::Io(format!(
                    "failed atomically replacing vault `{}`: {err}",
                    path.display()
                ))
            })?;
        }
        Ok(())
    }
}

impl SecretStore for EncryptedFallbackStore {
    fn set_secret(&self, key: &VaultKey, secret: &[u8]) -> Result<(), VaultError> {
        let encrypted = self.cipher.encrypt(secret)?;

        let mut cache = self
            .cache
            .write()
            .map_err(|_| VaultError::CryptoError("poisoned lock on vault store".into()))?;

        cache.insert(key.as_str().to_string(), encrypted);
        self.persist_if_file_backed(&cache)?;
        Ok(())
    }

    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError> {
        let cache = self
            .cache
            .read()
            .map_err(|_| VaultError::CryptoError("poisoned lock on vault store".into()))?;

        let encrypted = cache
            .get(key.as_str())
            .ok_or_else(|| VaultError::NotFound(key.as_str().to_string()))?;

        self.cipher.decrypt(encrypted)
    }

    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| VaultError::CryptoError("poisoned lock on vault store".into()))?;

        if cache.remove(key.as_str()).is_none() {
            return Err(VaultError::NotFound(key.as_str().to_string()));
        }

        self.persist_if_file_backed(&cache)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Argon2AesGcmCipher;
    use argon2::Params;

    fn fast_cipher() -> Arc<dyn MasterCipher> {
        let params = Params::new(1024, 1, 1, Some(32)).unwrap();
        Arc::new(Argon2AesGcmCipher::from_passphrase_with_params(
            b"test-vault-password",
            params,
        ))
    }

    #[test]
    fn in_memory_store_roundtrip_and_delete() {
        let store = EncryptedFallbackStore::in_memory(fast_cipher());
        let key = VaultKey::for_account("user@example.com", "imap").unwrap();
        let secret = b"my_app_password_999";

        assert_eq!(
            store.get_secret(&key),
            Err(VaultError::NotFound(key.as_str().into()))
        );

        store.set_secret(&key, secret).expect("set succeeds");
        assert_eq!(store.get_secret(&key).expect("get succeeds"), secret);

        store.delete_secret(&key).expect("delete succeeds");
        assert_eq!(
            store.get_secret(&key),
            Err(VaultError::NotFound(key.as_str().into()))
        );
    }

    #[test]
    fn file_backed_store_persists_across_instances() {
        let tmp_dir =
            std::env::temp_dir().join(format!("triage-mail-test-vault-{}", std::process::id()));
        let vault_path = tmp_dir.join("vault.enc");

        let cipher = fast_cipher();
        let key = VaultKey::for_account("user@example.com", "token").unwrap();
        let secret = b"oauth2_access_token_xyz";

        {
            let store1 = EncryptedFallbackStore::file_backed(&vault_path, cipher.clone())
                .expect("initializes file-backed store");
            store1.set_secret(&key, secret).expect("stores secret");
        }

        // Open a second store reading the same file
        {
            let store2 = EncryptedFallbackStore::file_backed(&vault_path, cipher)
                .expect("reopens file-backed store");
            let retrieved = store2.get_secret(&key).expect("retrieves persisted secret");
            assert_eq!(retrieved, secret);
        }

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
