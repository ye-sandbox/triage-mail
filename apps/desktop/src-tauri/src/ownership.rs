//! Keep ownership attached to detached credential work as well as the service.
#[cfg(any(feature = "native-runtime", test))]
use std::path::Path;
use std::sync::Arc;

use sync_runtime::ownership::WriterOwnership;
#[cfg(any(feature = "native-runtime", test))]
use vault::{SecretStore, VaultError, VaultKey};

#[cfg(any(feature = "native-runtime", test))]
use crate::remote::NativeRemoteMail;
#[cfg(any(feature = "native-runtime", test))]
use crate::DesktopService;
use crate::IpcError;
#[cfg(any(feature = "native-runtime", test))]
use zeroize::Zeroizing;

#[cfg(all(test, unix))]
mod tests;

pub(crate) fn acquire(path: &std::path::Path) -> Result<Arc<WriterOwnership>, IpcError> {
    WriterOwnership::acquire(path)
        .map(Arc::new)
        .map_err(|_| IpcError::StorageUnavailable)
}

#[cfg(any(feature = "native-runtime", test))]
fn owned_secrets(
    inner: Arc<dyn SecretStore>,
    ownership: Arc<WriterOwnership>,
) -> Arc<dyn SecretStore> {
    Arc::new(OwnedSecrets {
        inner,
        _ownership: ownership,
    })
}

#[cfg(any(feature = "native-runtime", test))]
struct OwnedSecrets {
    inner: Arc<dyn SecretStore>,
    // INVARIANT: NativeRemoteMail can detach blocking credential work during
    // cancellation. Its cloned vault must retain ownership beyond service teardown.
    _ownership: Arc<WriterOwnership>,
}

#[cfg(any(feature = "native-runtime", test))]
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

#[cfg(any(feature = "native-runtime", test))]
pub(crate) fn bootstrap(
    path: &Path,
    passphrase: Option<Zeroizing<String>>,
    ownership: Arc<sync_runtime::ownership::WriterOwnership>,
) -> Result<Arc<DesktopService>, IpcError> {
    if path.join("recovered-local-vault").exists() && passphrase.is_none() {
        return Err(IpcError::VaultUnavailable);
    }
    std::fs::create_dir_all(path).map_err(|_| IpcError::StorageUnavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| IpcError::StorageUnavailable)?;
    }
    let vault: Arc<dyn SecretStore> = match passphrase {
        Some(passphrase) => {
            if passphrase.is_empty() {
                return Err(IpcError::InvalidRequest);
            }
            let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
                passphrase.as_bytes(),
            ));
            let fallback =
                vault::EncryptedFallbackStore::file_backed(path.join("vault.enc"), cipher)
                    .map_err(|_| IpcError::VaultUnavailable)?;
            if path.join("recovered-local-vault").exists() {
                Arc::new(fallback)
            } else {
                Arc::new(vault::AutoSecretStore::new(Arc::new(fallback)))
            }
        }
        None => Arc::new(vault::KeyringStore::new()),
    };
    let vault = crate::ownership::owned_secrets(vault, ownership.clone());
    let key_id =
        VaultKey::for_account("application", "database-key").map_err(|_| IpcError::Internal)?;
    let db = path.join("mail.db");
    let raw = match vault.get_secret(&key_id) {
        Ok(bytes) => Zeroizing::new(bytes),
        Err(vault::VaultError::NotFound(_)) if !db.exists() => {
            let mut raw = Zeroizing::new(vec![0u8; 32]);
            getrandom::getrandom(&mut raw).map_err(|_| IpcError::Internal)?;
            vault
                .set_secret(&key_id, &raw)
                .map_err(|_| IpcError::VaultUnavailable)?;
            raw
        }
        _ => return Err(IpcError::VaultUnavailable),
    };
    let bytes: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| IpcError::VaultUnavailable)?;
    let store = storage::SqliteMailStore::open(
        db,
        &storage::DatabaseKey::from_bytes(bytes),
        Default::default(),
    )
    .map_err(|_| IpcError::StorageUnavailable)?;
    let remote = Arc::new(NativeRemoteMail::new(vault.clone()));
    let provider = crate::model_download::model_config()
        .ok()
        .map(triage_ai::onnx::OnnxEmbeddingProvider::new)
        .map(Arc::new);
    let mut service = DesktopService::new_learned(Arc::new(store), vault, remote, provider)?;
    Arc::get_mut(&mut service)
        .ok_or(IpcError::Internal)?
        ._ownership = Some(ownership);
    Ok(service)
}
