//! Desktop composition over ownership-bound local installation bootstrap.
#[cfg(any(feature = "native-runtime", test))]
use std::path::Path;
use std::sync::Arc;

use sync_runtime::ownership::WriterOwnership;

#[cfg(any(feature = "native-runtime", test))]
use crate::remote::NativeRemoteMail;
#[cfg(any(feature = "native-runtime", test))]
use crate::DesktopService;
use crate::IpcError;
#[cfg(any(feature = "native-runtime", test))]
use zeroize::Zeroizing;

#[cfg(all(test, any(unix, windows)))]
mod tests;

pub(crate) fn acquire(path: &std::path::Path) -> Result<Arc<WriterOwnership>, IpcError> {
    WriterOwnership::acquire(path)
        .map(Arc::new)
        .map_err(|_| IpcError::StorageUnavailable)
}

#[cfg(any(feature = "native-runtime", test))]
pub(crate) fn bootstrap(
    path: &Path,
    passphrase: Option<Zeroizing<String>>,
    ownership: Arc<sync_runtime::ownership::WriterOwnership>,
) -> Result<Arc<DesktopService>, IpcError> {
    let local =
        sync_runtime::bootstrap::open(path, passphrase, ownership.clone()).map_err(|error| {
            match error {
                sync_runtime::bootstrap::BootstrapError::InvalidRequest => IpcError::InvalidRequest,
                sync_runtime::bootstrap::BootstrapError::VaultUnavailable => {
                    IpcError::VaultUnavailable
                }
                sync_runtime::bootstrap::BootstrapError::StorageUnavailable => {
                    IpcError::StorageUnavailable
                }
                sync_runtime::bootstrap::BootstrapError::Internal => IpcError::Internal,
            }
        })?;
    let vault = local.vault;
    let remote = Arc::new(NativeRemoteMail::new(vault.clone()));
    let provider = crate::model_download::model_config()
        .ok()
        .map(triage_ai::onnx::OnnxEmbeddingProvider::new)
        .map(Arc::new);
    let mut service = DesktopService::new_learned(local.store, vault, remote, provider)?;
    Arc::get_mut(&mut service)
        .ok_or(IpcError::Internal)?
        ._ownership = Some(ownership);
    Ok(service)
}
