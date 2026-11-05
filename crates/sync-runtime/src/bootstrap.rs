//! Local installation opening only. Hosts own service composition and worker startup.
use std::{ops::Deref, path::Path, sync::Arc};

use storage::SqliteMailStore;
use vault::{SecretStore, VaultError, VaultKey};
use zeroize::Zeroizing;

use crate::ownership::WriterOwnership;

/// Fixed diagnostics never include paths, credentials or backend error strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapError {
    InvalidRequest,
    VaultUnavailable,
    StorageUnavailable,
    Internal,
}
impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidRequest => "invalid request",
            Self::VaultUnavailable => "vault unavailable",
            Self::StorageUnavailable => "storage unavailable",
            Self::Internal => "internal operation failed",
        })
    }
}
impl std::error::Error for BootstrapError {}

/// Independently cloned resources retain ownership, even after this envelope drops.
pub struct LocalInstallation {
    pub store: Arc<OwnedStorage>,
    pub vault: Arc<dyn SecretStore>,
}

/// Borrow the repository without exposing an independently owned unguarded handle.
/// INVARIANT: Drop the repository before the guard. Deref borrows cannot outlive self.
pub struct OwnedStorage {
    inner: SqliteMailStore,
    _ownership: Arc<WriterOwnership>,
}
impl Deref for OwnedStorage {
    type Target = SqliteMailStore;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

struct OwnedSecrets {
    inner: Arc<dyn SecretStore>,
    // INVARIANT: Detached credential operations must retain process ownership.
    _ownership: Arc<WriterOwnership>,
}
impl SecretStore for OwnedSecrets {
    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError> {
        self.inner.get_secret(key)
    }
    fn set_secret(&self, key: &VaultKey, secret: &[u8]) -> Result<(), VaultError> {
        self.inner.set_secret(key, secret)
    }
    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError> {
        self.inner.delete_secret(key)
    }
}

/// Open/provision local storage using the desktop's existing unlock policy.
/// The caller retains its own guard across failed unlock attempts. No worker,
/// provider access, inference, recovery publication or orphan cleanup runs here.
pub fn open(
    path: &Path,
    passphrase: Option<Zeroizing<String>>,
    ownership: Arc<WriterOwnership>,
) -> Result<LocalInstallation, BootstrapError> {
    open_with_vault(path, passphrase, ownership, select_vault)
}

fn open_with_vault(
    path: &Path,
    passphrase: Option<Zeroizing<String>>,
    ownership: Arc<WriterOwnership>,
    select: impl FnOnce(
        &Path,
        Option<Zeroizing<String>>,
        bool,
    ) -> Result<Arc<dyn SecretStore>, BootstrapError>,
) -> Result<LocalInstallation, BootstrapError> {
    // INVARIANT: Reject mismatched ownership before permissions, vault or SQL writes.
    if !ownership.matches_installation(path) {
        return Err(BootstrapError::StorageUnavailable);
    }
    let recovered = path.join("recovered-local-vault").exists();
    if recovered && passphrase.is_none() {
        return Err(BootstrapError::VaultUnavailable);
    }
    prepare_directory(path)?;
    let vault = Arc::new(OwnedSecrets {
        inner: select(path, passphrase, recovered)?,
        _ownership: ownership.clone(),
    });
    let store = open_storage(path, vault.as_ref())?;
    Ok(LocalInstallation {
        store: Arc::new(OwnedStorage {
            inner: store,
            _ownership: ownership,
        }),
        vault,
    })
}

fn prepare_directory(path: &Path) -> Result<(), BootstrapError> {
    std::fs::create_dir_all(path).map_err(|_| BootstrapError::StorageUnavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| BootstrapError::StorageUnavailable)?;
    }
    Ok(())
}

fn select_vault(
    path: &Path,
    passphrase: Option<Zeroizing<String>>,
    recovered: bool,
) -> Result<Arc<dyn SecretStore>, BootstrapError> {
    match passphrase {
        Some(passphrase) => {
            if passphrase.is_empty() {
                return Err(BootstrapError::InvalidRequest);
            }
            let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
                passphrase.as_bytes(),
            ));
            let fallback = Arc::new(
                vault::EncryptedFallbackStore::file_backed(path.join("vault.enc"), cipher)
                    .map_err(|_| BootstrapError::VaultUnavailable)?,
            );
            // INVARIANT: Recovery must never consult/provision the OS keyring.
            if recovered {
                Ok(fallback)
            } else {
                Ok(Arc::new(vault::AutoSecretStore::new(fallback)))
            }
        }
        None => Ok(Arc::new(vault::KeyringStore::new())),
    }
}

fn open_storage(path: &Path, vault: &dyn SecretStore) -> Result<SqliteMailStore, BootstrapError> {
    let key_id = VaultKey::for_account("application", "database-key")
        .map_err(|_| BootstrapError::Internal)?;
    let db = path.join("mail.db");
    let raw = match vault.get_secret(&key_id) {
        Ok(bytes) => Zeroizing::new(bytes),
        // INVARIANT: A missing key for an existing database is never replaced.
        Err(VaultError::NotFound(_)) if !db.exists() => {
            let mut raw = Zeroizing::new(vec![0u8; 32]);
            getrandom::getrandom(&mut raw).map_err(|_| BootstrapError::Internal)?;
            vault
                .set_secret(&key_id, &raw)
                .map_err(|_| BootstrapError::VaultUnavailable)?;
            raw
        }
        _ => return Err(BootstrapError::VaultUnavailable),
    };
    let bytes = Zeroizing::new(
        <[u8; 32]>::try_from(raw.as_slice()).map_err(|_| BootstrapError::VaultUnavailable)?,
    );
    SqliteMailStore::open(
        db,
        &storage::DatabaseKey::from_bytes(*bytes),
        Default::default(),
    )
    .map_err(|_| BootstrapError::StorageUnavailable)
}

#[cfg(all(test, unix))]
mod tests;
