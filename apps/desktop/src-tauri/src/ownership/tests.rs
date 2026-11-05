use super::*;
use std::{fs, process::Command};
use sync_runtime::ownership::OwnershipError;

const PHRASE: &str = "synthetic ownership fixture passphrase";

fn recovered(path: &Path) {
    fs::create_dir(path).unwrap();
    let store = storage::SqliteMailStore::open(
        path.join("mail.db"),
        &storage::DatabaseKey::from_bytes([29; 32]),
        Default::default(),
    )
    .unwrap();
    let fallback = vault::EncryptedFallbackStore::file_backed(
        path.join("vault.enc"),
        Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
            PHRASE.as_bytes(),
        )),
    )
    .unwrap();
    fallback
        .set_secret(
            &VaultKey::for_account("application", "database-key").unwrap(),
            &[29; 32],
        )
        .unwrap();
    fs::write(path.join("recovered-local-vault"), b"recovered").unwrap();
    drop(store);
}

fn contender(path: &Path, mode: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ownership::tests::process_helper", "--nocapture"])
        .env("TRIAGE_DESKTOP_OWNER_FIXTURE", path)
        .env("TRIAGE_DESKTOP_OWNER_MODE", mode)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn process_helper() {
    let Some(path) = std::env::var_os("TRIAGE_DESKTOP_OWNER_FIXTURE") else {
        return;
    };
    let path = Path::new(&path);
    let result = if std::env::var("TRIAGE_DESKTOP_OWNER_MODE").unwrap() == "restore" {
        crate::backup::restore(
            &path.with_extension("backup"),
            path,
            Zeroizing::new(PHRASE.into()),
        )
    } else {
        acquire(path).and_then(|guard| bootstrap(path, None, guard).map(|_| ()))
    };
    assert_eq!(result, Err(IpcError::StorageUnavailable));
}

#[test]
fn contenders_cannot_initialize_or_restore_an_owned_installation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    let ownership = acquire(&path).unwrap();
    for mode in ["initialize", "restore"] {
        contender(&path, mode);
    }
    assert!(!path.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1); // Only the empty lock.
    recovered(&path);
    let database = fs::read(path.join("mail.db")).unwrap();
    let vault = fs::read(path.join("vault.enc")).unwrap();
    for mode in ["initialize", "restore"] {
        contender(&path, mode);
    }
    assert_eq!(database, fs::read(path.join("mail.db")).unwrap());
    assert_eq!(vault, fs::read(path.join("vault.enc")).unwrap());
    drop(ownership);
    assert!(acquire(&path).is_ok());
}

#[test]
fn failed_unlock_retry_and_cloned_credentials_preserve_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    recovered(&path);
    let ownership = acquire(&path).unwrap();
    assert!(matches!(
        bootstrap(&path, None, ownership.clone()),
        Err(IpcError::VaultUnavailable)
    ));
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Busy)
    ));
    assert!(bootstrap(
        &path,
        Some(Zeroizing::new("incorrect fixture passphrase".into())),
        ownership.clone()
    )
    .is_err());
    let service = bootstrap(
        &path,
        Some(Zeroizing::new(PHRASE.into())),
        ownership.clone(),
    )
    .unwrap();
    drop(ownership);
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Busy)
    ));
    let detached_credentials = service.vault.clone();
    drop(service);
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Busy)
    ));
    let key = VaultKey::for_account("application", "database-key").unwrap();
    assert_eq!(detached_credentials.get_secret(&key).unwrap(), [29; 32]);
    drop(detached_credentials);
    assert!(acquire(&path).is_ok());
}

#[tokio::test]
async fn canceled_blocking_caller_cannot_release_ownership_while_storage_work_runs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    recovered(&path);
    let service = bootstrap(
        &path,
        Some(Zeroizing::new(PHRASE.into())),
        acquire(&path).unwrap(),
    )
    .unwrap();
    let (started, running) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let (finished, done) = tokio::sync::oneshot::channel();
    let caller = tokio::spawn(async move {
        service
            .blocking(move |service| {
                started.send(()).unwrap();
                wait.recv().unwrap();
                drop(service);
                finished.send(()).unwrap();
                Ok(())
            })
            .await
    });
    running.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Busy)
    ));
    release.send(()).unwrap();
    done.await.unwrap();
    assert!(acquire(&path).is_ok());
}

#[test]
fn restore_and_bootstrap_share_ownership_without_a_handoff_gap() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("installation");
    let source = dir.path().join("fixture.tmb");
    let store = storage::SqliteMailStore::open_in_memory(
        &storage::DatabaseKey::from_bytes([30; 32]),
        Default::default(),
    )
    .unwrap();
    store
        .write_backup(fs::File::create(&source).unwrap(), PHRASE.as_bytes())
        .unwrap();
    let ownership = acquire(&path).unwrap();
    crate::backup::restore_owned(
        &source,
        &path,
        Zeroizing::new(PHRASE.into()),
        ownership.clone(),
    )
    .unwrap();
    contender(&path, "initialize");
    let service = bootstrap(&path, Some(Zeroizing::new(PHRASE.into())), ownership).unwrap();
    contender(&path, "restore");
    drop(service);
    assert!(acquire(&path).is_ok());
}
