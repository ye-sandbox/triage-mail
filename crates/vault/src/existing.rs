//! Non-creating, bounded access to an existing fallback vault.
use std::{collections::HashMap, fs::File, io::Read, path::Path, sync::Arc};

use zeroize::Zeroizing;

use crate::{MasterCipher, VaultError, VaultKey};

const MAX_VAULT_BYTES: u64 = 16 * 1024 * 1024;

/// Existing file snapshot with no persistence or secret mutation operations.
pub struct ExistingVaultReader {
    cipher: Arc<dyn MasterCipher>,
    entries: HashMap<String, Vec<u8>>,
}

impl ExistingVaultReader {
    pub fn open(path: &Path, cipher: Arc<dyn MasterCipher>) -> Result<Self, VaultError> {
        let mut bytes = Vec::new();
        // INVARIANT: Read-only open cannot initialize a missing vault. The byte limit
        // applies to the actual read, including a concurrently growing file.
        File::open(path)
            .map_err(|_| unavailable())?
            .take(MAX_VAULT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| unavailable())?;
        if bytes.len() as u64 > MAX_VAULT_BYTES {
            return Err(unavailable());
        }
        let entries = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        Ok(Self { cipher, entries })
    }

    pub fn get_secret(&self, key: &VaultKey) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let encrypted = self.entries.get(key.as_str()).ok_or_else(unavailable)?;
        self.cipher
            .decrypt(encrypted)
            .map(Zeroizing::new)
            .map_err(|_| unavailable())
    }
}

fn unavailable() -> VaultError {
    VaultError::BackendUnavailable("existing vault unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Argon2AesGcmCipher, EncryptedFallbackStore, SecretStore};

    #[test]
    fn existing_reader_authenticates_without_creating_or_writing() {
        let path = std::env::temp_dir().join(format!("existing-vault-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let cipher = Arc::new(Argon2AesGcmCipher::from_key([27; 32]));
        assert!(ExistingVaultReader::open(&path, cipher.clone()).is_err());
        assert!(!path.exists());
        let key = VaultKey::for_account("application", "database-key").unwrap();
        let store = EncryptedFallbackStore::file_backed(&path, cipher.clone()).unwrap();
        store.set_secret(&key, &[24; 32]).unwrap();
        let before = std::fs::read(&path).unwrap();
        let reader = ExistingVaultReader::open(&path, cipher).unwrap();
        assert_eq!(&*reader.get_secret(&key).unwrap(), &[24; 32]);
        assert!(reader
            .get_secret(&VaultKey::for_account("absent", "key").unwrap())
            .is_err());
        let wrong =
            ExistingVaultReader::open(&path, Arc::new(Argon2AesGcmCipher::from_key([28; 32])))
                .unwrap();
        assert!(wrong.get_secret(&key).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::write(&path, b"not a vault").unwrap();
        assert!(
            ExistingVaultReader::open(&path, Arc::new(Argon2AesGcmCipher::from_key([27; 32])))
                .is_err()
        );
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_VAULT_BYTES + 1).unwrap();
        assert!(
            ExistingVaultReader::open(&path, Arc::new(Argon2AesGcmCipher::from_key([27; 32])))
                .is_err()
        );
        std::fs::remove_file(path).unwrap();
    }
}
