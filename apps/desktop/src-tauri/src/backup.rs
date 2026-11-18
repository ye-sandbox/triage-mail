//! Local-only file I/O and fresh-target recovery. Fixed errors keep secret material out of IPC.
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use storage::portable::{PortableDefinitions, ReferenceMapping, MAX_PORTABLE_BYTES};
use vault::{SecretStore, VaultKey};
use zeroize::Zeroizing;

use crate::{
    service::{DesktopService, IpcError},
    settings::IncomingApiKey,
};

pub(crate) fn local_path(path: &Path) -> Result<(), IpcError> {
    if !path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(IpcError::InvalidRequest);
    }
    let parent = path.parent().ok_or(IpcError::InvalidRequest)?;
    if parent
        .canonicalize()
        .map_err(|_| IpcError::InvalidRequest)?
        != parent
    {
        return Err(IpcError::InvalidRequest);
    }
    Ok(())
}
pub fn private_file(path: &Path) -> Result<File, IpcError> {
    local_path(path)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| IpcError::InvalidRequest)
}
fn input(path: &Path) -> Result<File, IpcError> {
    local_path(path)?;
    let meta = std::fs::symlink_metadata(path).map_err(|_| IpcError::InvalidRequest)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 64 * 1024 * 1024 * 1024 {
        return Err(IpcError::InvalidRequest);
    }
    let file = File::open(path).map_err(|_| IpcError::InvalidRequest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = file.metadata().map_err(|_| IpcError::InvalidRequest)?;
        if meta.dev() != opened.dev() || meta.ino() != opened.ino() {
            return Err(IpcError::InvalidRequest);
        }
    }
    Ok(file)
}
pub(crate) fn open_local_input(path: &Path) -> Result<File, IpcError> {
    input(path)
}
pub fn preview(path: &Path) -> Result<PortableDefinitions, IpcError> {
    let mut bytes = Vec::new();
    input(path)?
        .take((MAX_PORTABLE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| IpcError::InvalidRequest)?;
    PortableDefinitions::parse(&bytes).map_err(|_| IpcError::InvalidRequest)
}
impl DesktopService {
    pub async fn backup_file(
        self: &Arc<Self>,
        path: PathBuf,
        passphrase: IncomingApiKey,
    ) -> Result<(), IpcError> {
        self.blocking(move |s| {
            let file = private_file(&path)?;
            let result = s
                .store
                .write_backup(BufWriter::new(file), passphrase.0.as_bytes())
                .map_err(|_| IpcError::StorageUnavailable)
                .and_then(|writer| {
                    writer
                        .get_ref()
                        .sync_all()
                        .map_err(|_| IpcError::StorageUnavailable)
                });
            if result.is_err() {
                let _ = std::fs::remove_file(&path);
            }
            result
        })
        .await
    }
    pub async fn export_file(self: &Arc<Self>, path: PathBuf) -> Result<(), IpcError> {
        self.blocking(move |s| {
            let definitions = s
                .store
                .export_definitions()
                .map_err(|_| IpcError::StorageUnavailable)?;
            let file = private_file(&path)?;
            let result = serde_json::to_writer_pretty(&file, &definitions)
                .map_err(|_| IpcError::StorageUnavailable)
                .and_then(|()| file.sync_all().map_err(|_| IpcError::StorageUnavailable));
            if result.is_err() {
                let _ = std::fs::remove_file(&path);
            }
            result
        })
        .await
    }
    pub async fn import_file(
        self: &Arc<Self>,
        path: PathBuf,
        mapping: ReferenceMapping,
        expected: PortableDefinitions,
        consent: bool,
    ) -> Result<(), IpcError> {
        if !consent {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(move |s| {
            let bundle = preview(&path)?;
            // INVARIANT: Re-read and compare the exact preview before importing; a
            // replaced file cannot silently change the user's reviewed definitions.
            if serde_json::to_vec(&bundle).ok() != serde_json::to_vec(&expected).ok() {
                return Err(IpcError::InvalidRequest);
            }
            s.store
                .import_definitions(bundle, mapping)
                .map_err(|_| IpcError::InvalidRequest)
        })
        .await
    }
}

/// Startup only: stage beside an absent target, fsync, then publish the directory.
/// Recovery uses an isolated fallback vault and never reads/writes the OS keyring.
pub fn restore(
    source: &Path,
    target: &Path,
    passphrase: Zeroizing<String>,
) -> Result<(), IpcError> {
    let ownership = crate::ownership::acquire(target)?;
    restore_owned(source, target, passphrase, ownership)
}

/// The caller retains the same ownership across recovery and subsequent bootstrap.
pub(crate) fn restore_owned(
    source: &Path,
    target: &Path,
    passphrase: Zeroizing<String>,
    _ownership: Arc<sync_runtime::ownership::WriterOwnership>,
) -> Result<(), IpcError> {
    if target.exists() || std::fs::symlink_metadata(target).is_ok() {
        return Err(IpcError::InvalidRequest);
    }
    let parent = target.parent().ok_or(IpcError::InvalidRequest)?;
    std::fs::create_dir_all(parent).map_err(|_| IpcError::StorageUnavailable)?;
    local_path(target)?;
    let stage = parent.join(format!(
        ".triage-recovery-{}",
        vault::backup::random_id().map_err(|_| IpcError::Internal)?
    ));
    std::fs::create_dir(&stage).map_err(|_| IpcError::StorageUnavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stage, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| IpcError::StorageUnavailable)?;
    }
    let result = stage_restore(source, &stage, &passphrase).and_then(|()| {
        if std::fs::symlink_metadata(target).is_ok() {
            return Err(IpcError::InvalidRequest);
        }
        // Nonempty targets cannot be replaced by directory rename. Staging never
        // touches an existing database/vault; abandoned stages are inert on startup.
        std::fs::rename(&stage, target).map_err(|_| IpcError::StorageUnavailable)?;
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| IpcError::StorageUnavailable)
    });
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}
fn stage_restore(source: &Path, stage: &Path, passphrase: &str) -> Result<(), IpcError> {
    let mut raw = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(raw.as_mut()).map_err(|_| IpcError::Internal)?;
    let store = storage::SqliteMailStore::open(
        stage.join("mail.db"),
        &storage::DatabaseKey::from_bytes(*raw),
        Default::default(),
    )
    .map_err(|_| IpcError::StorageUnavailable)?;
    store
        .read_backup(
            BufReader::new(input(source)?.take(64 * 1024 * 1024 * 1024 + 1)),
            passphrase.as_bytes(),
        )
        .map_err(|_| IpcError::InvalidRequest)?;
    drop(store);
    let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
        passphrase.as_bytes(),
    ));
    let vault = vault::EncryptedFallbackStore::file_backed(stage.join("vault.enc"), cipher)
        .map_err(|_| IpcError::VaultUnavailable)?;
    vault
        .set_secret(
            &VaultKey::for_account("application", "database-key")
                .map_err(|_| IpcError::Internal)?,
            raw.as_ref(),
        )
        .map_err(|_| IpcError::VaultUnavailable)?;
    private_file(&stage.join("recovered-local-vault"))?
        .sync_all()
        .map_err(|_| IpcError::StorageUnavailable)?;
    for name in ["mail.db", "vault.enc"] {
        File::open(stage.join(name))
            .and_then(|f| f.sync_all())
            .map_err(|_| IpcError::StorageUnavailable)?;
    }
    File::open(stage)
        .and_then(|f| f.sync_all())
        .map_err(|_| IpcError::StorageUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_io_rejects_existing_outputs_symlinks_and_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("definitions.json");
        std::fs::write(&path, b"synthetic existing file").unwrap();
        assert!(private_file(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic existing file");
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(input(&link).is_err());
            assert!(private_file(&link).is_err());
            let linked_parent = dir.path().join("linked-parent");
            std::os::unix::fs::symlink(dir.path(), &linked_parent).unwrap();
            assert!(private_file(&linked_parent.join("new-file")).is_err());
        }
        assert!(private_file(&dir.path().join("../traversal")).is_err());
    }
    #[test]
    fn restore_isolated_vault_occupied_target_and_failed_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("backup.tmb");
        let store = storage::SqliteMailStore::open_in_memory(
            &storage::DatabaseKey::from_bytes([3; 32]),
            Default::default(),
        )
        .unwrap();
        store
            .write_backup(File::create(&path).unwrap(), b"synthetic backup passphrase")
            .unwrap();
        let target = dir.path().join("restored");
        assert!(restore(
            &path,
            &target,
            Zeroizing::new("incorrect synthetic passphrase".into())
        )
        .is_err());
        assert!(!target.exists());
        restore(
            &path,
            &target,
            Zeroizing::new("synthetic backup passphrase".into()),
        )
        .unwrap();
        assert!(target.join("recovered-local-vault").is_file());
        let recovered_vault = vault::EncryptedFallbackStore::file_backed(
            target.join("vault.enc"),
            Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
                b"synthetic backup passphrase",
            )),
        )
        .unwrap();
        let raw = Zeroizing::new(
            recovered_vault
                .get_secret(&VaultKey::for_account("application", "database-key").unwrap())
                .unwrap(),
        );
        let key: [u8; 32] = raw.as_slice().try_into().unwrap();
        assert_ne!(key, [3; 32]);
        let reopened = storage::SqliteMailStore::open(
            target.join("mail.db"),
            &storage::DatabaseKey::from_bytes(key),
            Default::default(),
        )
        .unwrap();
        assert!(reopened.disconnected_accounts().unwrap().is_empty());
        drop(reopened);
        let original = std::fs::read(target.join("vault.enc")).unwrap();
        assert!(restore(
            &path,
            &target,
            Zeroizing::new("synthetic backup passphrase".into())
        )
        .is_err());
        assert_eq!(original, std::fs::read(target.join("vault.enc")).unwrap());
    }
}
