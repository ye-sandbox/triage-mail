//! Cooperative installation ownership, independent of SQLite's transaction locks.
use std::{fs::File, path::Path};

/// Fixed diagnostics deliberately omit filesystem paths and OS error details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnershipError {
    Busy,
    Unavailable,
}

impl std::fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Busy => "installation already owned",
            Self::Unavailable => "installation ownership unavailable",
        })
    }
}

impl std::error::Error for OwnershipError {}

/// Keep this handle alive for every worker and blocking operation using the installation.
///
/// INVARIANT: The empty sibling lock file is persistent. Never unlink it on release:
/// another opener could otherwise lock a new inode while the old owner still runs.
/// Locks are advisory and require cooperating processes on a local filesystem.
/// This is not protection against hostile filesystem replacement or older binaries.
pub struct WriterOwnership {
    _file: File,
    #[cfg(any(unix, windows))]
    installation: std::path::PathBuf,
}

impl WriterOwnership {
    pub(crate) fn matches_installation(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            unix::installation_path(path).is_ok_and(|path| path == self.installation)
        }
        #[cfg(windows)]
        {
            windows::installation_path(path).is_ok_and(|path| path == self.installation)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            false
        }
    }

    /// Acquire immediately or fail. Parent directories may be created, but the
    /// installation itself is neither created nor opened.
    pub fn acquire(installation: &Path) -> Result<Self, OwnershipError> {
        #[cfg(unix)]
        {
            unix::acquire(installation)
        }
        #[cfg(windows)]
        {
            windows::acquire(installation)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = installation;
            Err(OwnershipError::Unavailable)
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{
        ffi::OsString,
        fs::{self, Metadata, OpenOptions, TryLockError},
        os::unix::fs::{MetadataExt, OpenOptionsExt},
        path::Component,
    };

    pub(super) fn acquire(installation: &Path) -> Result<WriterOwnership, OwnershipError> {
        // INVARIANT: Validate before creating parents; rejected paths cannot provision them.
        validate_path(installation)?;
        std::fs::create_dir_all(installation.parent().ok_or(OwnershipError::Unavailable)?)
            .map_err(|_| OwnershipError::Unavailable)?;
        let installation = installation_path(installation)?;
        let mut lock_name = OsString::from(".triage-mail-writer-");
        lock_name.push(
            installation
                .file_name()
                .ok_or(OwnershipError::Unavailable)?,
        );
        lock_name.push(".lock");
        let lock = installation.with_file_name(lock_name);
        let file = open_lock(&lock)?;
        match file.try_lock() {
            Ok(()) => Ok(WriterOwnership {
                _file: file,
                installation,
            }),
            Err(TryLockError::WouldBlock) => Err(OwnershipError::Busy),
            Err(TryLockError::Error(_)) => Err(OwnershipError::Unavailable),
        }
    }

    fn validate_path(installation: &Path) -> Result<(), OwnershipError> {
        if !installation.is_absolute()
            || installation
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(OwnershipError::Unavailable);
        }
        installation
            .file_name()
            .ok_or(OwnershipError::Unavailable)?;
        Ok(())
    }

    pub(super) fn installation_path(
        installation: &Path,
    ) -> Result<std::path::PathBuf, OwnershipError> {
        validate_path(installation)?;
        let name = installation
            .file_name()
            .ok_or(OwnershipError::Unavailable)?;
        let parent = installation.parent().ok_or(OwnershipError::Unavailable)?;
        let parent = parent
            .canonicalize()
            .map_err(|_| OwnershipError::Unavailable)?;
        match fs::symlink_metadata(parent.join(name)) {
            Ok(meta) if !meta.is_dir() => return Err(OwnershipError::Unavailable),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(OwnershipError::Unavailable);
            }
            _ => {}
        }
        Ok(parent.join(name))
    }

    fn valid(meta: &Metadata) -> bool {
        meta.is_file() && meta.len() == 0 && meta.nlink() == 1 && meta.mode() & 0o777 == 0o600
    }

    fn open_lock(path: &Path) -> Result<File, OwnershipError> {
        let created = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path);
        let file = match created {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let meta = fs::symlink_metadata(path).map_err(|_| OwnershipError::Unavailable)?;
                if !valid(&meta) {
                    return Err(OwnershipError::Unavailable);
                }
                // INVARIANT: No truncation, permission repair or content writes. Metadata
                // checks reject ordinary symlinks/FIFOs; they do not defeat hostile races.
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path)
                    .map_err(|_| OwnershipError::Unavailable)?
            }
            Err(_) => return Err(OwnershipError::Unavailable),
        };
        let actual = file.metadata().map_err(|_| OwnershipError::Unavailable)?;
        let named = fs::symlink_metadata(path).map_err(|_| OwnershipError::Unavailable)?;
        if !valid(&actual)
            || !valid(&named)
            || actual.dev() != named.dev()
            || actual.ino() != named.ino()
        {
            return Err(OwnershipError::Unavailable);
        }
        Ok(file)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        ffi::OsString,
        fs::{self, Metadata, OpenOptions, TryLockError},
        os::windows::fs::MetadataExt,
        path::Component,
    };

    pub(super) fn acquire(installation: &Path) -> Result<WriterOwnership, OwnershipError> {
        // INVARIANT: Validate before creating parents; rejected paths cannot provision them.
        validate_path(installation)?;
        std::fs::create_dir_all(installation.parent().ok_or(OwnershipError::Unavailable)?)
            .map_err(|_| OwnershipError::Unavailable)?;
        let installation = installation_path(installation)?;
        let mut lock_name = OsString::from(".triage-mail-writer-");
        lock_name.push(
            installation
                .file_name()
                .ok_or(OwnershipError::Unavailable)?,
        );
        lock_name.push(".lock");
        let lock = installation.with_file_name(lock_name);
        let file = open_lock(&lock)?;
        match file.try_lock() {
            Ok(()) => Ok(WriterOwnership {
                _file: file,
                installation,
            }),
            Err(TryLockError::WouldBlock) => Err(OwnershipError::Busy),
            Err(TryLockError::Error(_)) => Err(OwnershipError::Unavailable),
        }
    }

    fn validate_path(installation: &Path) -> Result<(), OwnershipError> {
        if !installation.is_absolute()
            || installation
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(OwnershipError::Unavailable);
        }
        installation
            .file_name()
            .ok_or(OwnershipError::Unavailable)?;
        Ok(())
    }

    pub(super) fn installation_path(
        installation: &Path,
    ) -> Result<std::path::PathBuf, OwnershipError> {
        validate_path(installation)?;
        let name = installation
            .file_name()
            .ok_or(OwnershipError::Unavailable)?;
        let parent = installation.parent().ok_or(OwnershipError::Unavailable)?;
        let parent = parent
            .canonicalize()
            .map_err(|_| OwnershipError::Unavailable)?;
        match fs::symlink_metadata(parent.join(name)) {
            Ok(meta) if !meta.is_dir() => return Err(OwnershipError::Unavailable),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(OwnershipError::Unavailable);
            }
            _ => {}
        }
        Ok(parent.join(name))
    }

    fn valid(meta: &Metadata) -> bool {
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.is_file()
            && !meta.file_type().is_symlink()
            && (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) == 0
            && meta.len() == 0
            && meta.number_of_links() == Some(1)
    }

    fn open_lock(path: &Path) -> Result<File, OwnershipError> {
        let created = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path);
        let file = match created {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let meta = fs::symlink_metadata(path).map_err(|_| OwnershipError::Unavailable)?;
                if !valid(&meta) {
                    return Err(OwnershipError::Unavailable);
                }
                // INVARIANT: No truncation, permission repair or content writes. Metadata
                // checks reject ordinary symlinks/FIFOs; they do not defeat hostile races.
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path)
                    .map_err(|_| OwnershipError::Unavailable)?
            }
            Err(_) => return Err(OwnershipError::Unavailable),
        };
        let actual = file.metadata().map_err(|_| OwnershipError::Unavailable)?;
        let named = fs::symlink_metadata(path).map_err(|_| OwnershipError::Unavailable)?;
        let actual_vol = actual
            .volume_serial_number()
            .ok_or(OwnershipError::Unavailable)?;
        let named_vol = named
            .volume_serial_number()
            .ok_or(OwnershipError::Unavailable)?;
        let actual_idx = actual.file_index().ok_or(OwnershipError::Unavailable)?;
        let named_idx = named.file_index().ok_or(OwnershipError::Unavailable)?;
        if !valid(&actual) || !valid(&named) || actual_vol != named_vol || actual_idx != named_idx {
            return Err(OwnershipError::Unavailable);
        }
        Ok(file)
    }
}
