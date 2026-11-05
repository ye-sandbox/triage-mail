use super::*;
use crate::ownership::OwnershipError;
use std::{collections::HashMap, fs, sync::Mutex};

const PHRASE: &str = "synthetic bootstrap fixture passphrase";

#[derive(Default)]
struct MemoryVault(Mutex<HashMap<String, Vec<u8>>>);
impl SecretStore for MemoryVault {
    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError> {
        self.0
            .lock()
            .unwrap()
            .get(key.as_str())
            .cloned()
            .ok_or_else(|| VaultError::NotFound(key.as_str().into()))
    }
    fn set_secret(&self, key: &VaultKey, bytes: &[u8]) -> Result<(), VaultError> {
        self.0
            .lock()
            .unwrap()
            .insert(key.as_str().into(), bytes.to_vec());
        Ok(())
    }
    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError> {
        self.0.lock().unwrap().remove(key.as_str());
        Ok(())
    }
}
fn key() -> VaultKey {
    VaultKey::for_account("application", "database-key").unwrap()
}
fn owner(path: &Path) -> Arc<WriterOwnership> {
    Arc::new(WriterOwnership::acquire(path).unwrap())
}
fn injected(
    path: &Path,
    ownership: Arc<WriterOwnership>,
    vault: Arc<MemoryVault>,
) -> Result<LocalInstallation, BootstrapError> {
    open_with_vault(path, None, ownership, |_, phrase, recovered| {
        assert!(phrase.is_none());
        assert!(!recovered);
        Ok(vault)
    })
}
fn assert_busy(path: &Path) {
    assert!(matches!(
        WriterOwnership::acquire(path),
        Err(OwnershipError::Busy)
    ));
}
fn recovered(path: &Path) {
    fs::create_dir(path).unwrap();
    let store = SqliteMailStore::open(
        path.join("mail.db"),
        &storage::DatabaseKey::from_bytes([31; 32]),
        Default::default(),
    )
    .unwrap();
    let vault = vault::EncryptedFallbackStore::file_backed(
        path.join("vault.enc"),
        Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
            PHRASE.as_bytes(),
        )),
    )
    .unwrap();
    vault.set_secret(&key(), &[31; 32]).unwrap();
    fs::write(path.join("recovered-local-vault"), b"recovered").unwrap();
    drop(store);
}

#[test]
fn provisioning_reopen_and_independently_retained_resources_hold_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    let ownership = owner(&path);
    let vault = Arc::new(MemoryVault::default());
    let local = injected(&path, ownership.clone(), vault.clone()).unwrap();
    assert_eq!(vault.get_secret(&key()).unwrap().len(), 32);
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o700
    );
    local.store.list_account_connections().unwrap();
    let persisted = vault.get_secret(&key()).unwrap();
    drop(local);
    let local = injected(&path, ownership.clone(), vault.clone()).unwrap();
    assert_eq!(vault.get_secret(&key()).unwrap(), persisted);
    drop(ownership);
    let store = local.store.clone();
    let secrets = local.vault.clone();
    drop(local);
    assert_busy(&path);
    drop(secrets);
    assert_busy(&path);
    store.list_account_connections().unwrap();
    drop(store);
    let local = injected(&path, owner(&path), vault).unwrap();
    let secrets = local.vault.clone();
    drop(local);
    assert_busy(&path);
    assert_eq!(secrets.get_secret(&key()).unwrap(), persisted);
    drop(secrets);
    assert!(WriterOwnership::acquire(&path).is_ok());
}

#[test]
fn missing_or_malformed_existing_keys_are_not_replaced_and_retry_keeps_guard() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    let ownership = owner(&path);
    let vault = Arc::new(MemoryVault::default());
    drop(injected(&path, ownership.clone(), vault.clone()).unwrap());
    let original = vault.get_secret(&key()).unwrap();
    let database = fs::read(path.join("mail.db")).unwrap();
    for invalid in [None, Some(vec![1; 31]), Some(vec![2; 33])] {
        vault.delete_secret(&key()).unwrap();
        if let Some(bytes) = invalid.as_ref() {
            vault.set_secret(&key(), bytes).unwrap();
        }
        assert!(matches!(
            injected(&path, ownership.clone(), vault.clone()),
            Err(BootstrapError::VaultUnavailable)
        ));
        assert_eq!(vault.get_secret(&key()).ok(), invalid);
        assert_eq!(fs::read(path.join("mail.db")).unwrap(), database);
        assert_busy(&path);
    }
    vault.set_secret(&key(), &[0; 32]).unwrap();
    assert!(matches!(
        injected(&path, ownership.clone(), vault.clone()),
        Err(BootstrapError::StorageUnavailable)
    ));
    assert_eq!(vault.get_secret(&key()).unwrap(), [0; 32]);
    assert_eq!(fs::read(path.join("mail.db")).unwrap(), database);
    vault.set_secret(&key(), &original).unwrap();
    assert!(injected(&path, ownership, vault).is_ok());
}

#[test]
fn mismatched_guard_and_symlink_installation_fail_before_selection_or_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    let other = dir.path().join("missing-parent/other");
    let ownership = owner(&path);
    assert!(matches!(
        open_with_vault(&other, None, ownership.clone(), |_, _, _| panic!(
            "selected vault"
        )),
        Err(BootstrapError::StorageUnavailable)
    ));
    assert!(!dir.path().join("missing-parent").exists());
    fs::create_dir(dir.path().join("target")).unwrap();
    std::os::unix::fs::symlink(dir.path().join("target"), &path).unwrap();
    assert!(matches!(
        open(&path, None, ownership),
        Err(BootstrapError::StorageUnavailable)
    ));
    assert_eq!(fs::read_dir(dir.path().join("target")).unwrap().count(), 0);
}

#[test]
fn canonical_parent_alias_can_open_the_owned_installation() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, dir.path().join("alias")).unwrap();
    let path = real.join("installation");
    let ownership = owner(&path);
    assert!(injected(
        &dir.path().join("alias/installation"),
        ownership,
        Arc::new(MemoryVault::default())
    )
    .is_ok());
}

#[test]
fn recovery_requires_valid_fallback_and_never_uses_keyring() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    recovered(&path);
    let ownership = owner(&path);
    let database = fs::read(path.join("mail.db")).unwrap();
    let vault = fs::read(path.join("vault.enc")).unwrap();
    assert!(matches!(
        open_with_vault(&path, None, ownership.clone(), |_, _, _| panic!(
            "selected vault"
        )),
        Err(BootstrapError::VaultUnavailable)
    ));
    for (phrase, expected) in [
        ("", BootstrapError::InvalidRequest),
        (
            "incorrect synthetic passphrase",
            BootstrapError::VaultUnavailable,
        ),
    ] {
        assert!(
            matches!(open(&path, Some(Zeroizing::new(phrase.into())), ownership.clone()),
            Err(error) if error == expected)
        );
        assert_eq!(fs::read(path.join("mail.db")).unwrap(), database);
        assert_eq!(fs::read(path.join("vault.enc")).unwrap(), vault);
        assert_busy(&path);
    }
    let local = open(&path, Some(Zeroizing::new(PHRASE.into())), ownership).unwrap();
    assert_eq!(local.vault.get_secret(&key()).unwrap(), [31; 32]);
    assert_eq!(fs::read(path.join("vault.enc")).unwrap(), vault);
}

#[test]
fn invalid_passphrase_and_backend_failure_have_fixed_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    let ownership = owner(&path);
    assert!(matches!(
        open(
            &path,
            Some(Zeroizing::new(String::new())),
            ownership.clone()
        ),
        Err(BootstrapError::InvalidRequest)
    ));
    assert!(matches!(
        open_with_vault(&path, None, ownership, |_, _, _| Err(
            BootstrapError::VaultUnavailable
        )),
        Err(BootstrapError::VaultUnavailable)
    ));
    assert!(!path.join("mail.db").exists());
    assert!(!path.join("vault.enc").exists());
    assert_eq!(
        BootstrapError::VaultUnavailable.to_string(),
        "vault unavailable"
    );
}

#[test]
fn process_helper() {
    let Some(path) = std::env::var_os("TRIAGE_BOOTSTRAP_FIXTURE") else {
        return;
    };
    let path = Path::new(&path);
    let local = open(path, Some(Zeroizing::new(PHRASE.into())), owner(path)).unwrap();
    let retain_store = std::env::var("TRIAGE_BOOTSTRAP_RESOURCE").unwrap() == "store";
    let store = retain_store.then(|| local.store.clone());
    let vault = (!retain_store).then(|| local.vault.clone());
    drop(local);
    use std::io::{Read, Write};
    println!("bootstrap-ready");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_exact(&mut [0]).unwrap();
    if let Some(store) = store {
        store.list_account_connections().unwrap();
    }
    if let Some(vault) = vault {
        assert_eq!(vault.get_secret(&key()).unwrap(), [31; 32]);
    }
}

#[test]
fn subprocess_resources_retain_lock_until_exit() {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    recovered(&path);
    for resource in ["store", "vault"] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "bootstrap::tests::process_helper", "--nocapture"])
            .env("TRIAGE_BOOTSTRAP_FIXTURE", &path)
            .env("TRIAGE_BOOTSTRAP_RESOURCE", resource)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            assert_ne!(
                output.read_line(&mut line).unwrap(),
                0,
                "bootstrap helper exited early"
            );
            if line.trim() == "bootstrap-ready" {
                break;
            }
            line.clear();
        }
        assert_busy(&path);
        child.stdin.as_mut().unwrap().write_all(b"x").unwrap();
        assert!(child.wait().unwrap().success());
        assert!(WriterOwnership::acquire(&path).is_ok());
    }
}
