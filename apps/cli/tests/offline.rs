use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
    sync::Arc,
};

use vault::{SecretStore, VaultKey};

const PHRASE: &[u8] = b"synthetic local fixture phrase";

struct Fixture {
    dir: tempfile::TempDir,
    // Keep a writer open so the executable must read committed, uncheckpointed WAL.
    store: storage::SqliteMailStore,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let key = storage::DatabaseKey::from_bytes([23; 32]);
    let store =
        storage::SqliteMailStore::open(dir.path().join("mail.db"), &key, Default::default())
            .unwrap();
    let conn =
        storage::connection::open_encrypted_connection(dir.path().join("mail.db"), &key).unwrap();
    conn.execute_batch(
        "PRAGMA wal_autocheckpoint=0;
         INSERT INTO accounts VALUES ('acct-a','reader@example.org','AppPassword',0,0);
         INSERT INTO accounts VALUES ('acct-b','other@example.org','AppPassword',0,0);
         INSERT INTO messages (id,account_id,folder,uid,subject,sender,recipients,
             received_at_epoch_ms,raw_headers,text_body,created_at_epoch_ms)
         VALUES ('m-a','acct-a','INBOX',1,'boleto','sender@example.org','reader@example.org',1,'','text',0),
                ('m-b','acct-b','INBOX',1,'boleto','sender@example.org','other@example.org',2,'','other text',0);
         INSERT INTO pending_mutations(message_id,payload_json) VALUES ('m-a','{}');"
    ).unwrap();
    let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(PHRASE));
    let fallback =
        vault::EncryptedFallbackStore::file_backed(dir.path().join("vault.enc"), cipher).unwrap();
    fallback
        .set_secret(
            &VaultKey::for_account("application", "database-key").unwrap(),
            &[23; 32],
        )
        .unwrap();
    std::fs::write(dir.path().join("recovered-local-vault"), b"recovered").unwrap();
    Fixture { dir, store }
}

fn invoke(path: &Path, arguments: &[&str], phrase: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_triage-mail-cli"))
        .arg("--data-dir")
        .arg(path)
        .arg("--unlock-stdin")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Rejected paths/arguments can exit before reading stdin; only a closed pipe is
    // expected in that case. The child's exit code/output remain the assertions.
    if let Err(error) = child.stdin.take().unwrap().write_all(phrase) {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    child.wait_with_output().unwrap()
}

#[test]
fn executable_reads_encrypted_cache_without_processing_or_changing_it() {
    let fixture = fixture();
    let path = fixture.dir.path();
    let before = std::fs::read(path.join("vault.enc")).unwrap();
    let accounts = invoke(path, &["accounts"], PHRASE);
    assert!(
        accounts.status.success(),
        "{}",
        String::from_utf8_lossy(&accounts.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Vec<String>>(&accounts.stdout).unwrap(),
        ["acct-a", "acct-b"]
    );
    assert!(!String::from_utf8_lossy(&accounts.stdout).contains('@'));
    let output = invoke(
        path,
        &["search", "--query", "boleto", "--account", "acct-a"],
        PHRASE,
    );
    assert!(output.status.success());
    let rows: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["id"], "m-a");
    let page = invoke(path, &["search", "--limit", "1", "--offset", "1"], PHRASE);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&page.stdout).unwrap()[0]["id"],
        "m-a"
    );
    let body = invoke(path, &["show", "--message-id", "m-a"], PHRASE);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body.stdout).unwrap()["text_plain"],
        "text"
    );
    assert!(body.stderr.is_empty());
    assert_eq!(std::fs::read(path.join("vault.enc")).unwrap(), before);
    // Reading from the retained store also proves no database replacement occurred.
    assert!(
        !fixture
            .store
            .get_message_by_id("m-a")
            .unwrap()
            .unwrap()
            .is_read
    );
    let key = storage::DatabaseKey::from_bytes([23; 32]);
    let conn = storage::connection::open_encrypted_connection(path.join("mail.db"), &key).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pending_mutations", [], |r| r
            .get::<_, usize>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, usize>(0))
            .unwrap(),
        11
    );
}

#[test]
fn fixed_errors_never_echo_passphrases_arguments_or_mail() {
    let fixture = fixture();
    let path = fixture.dir.path();
    for (args, phrase, code) in [
        (vec!["accounts"], &b"synthetic incorrect phrase"[..], 3),
        (vec!["show", "--message-id", "absent"], PHRASE, 5),
        (
            vec!["accounts", "--secret", "synthetic private argument"],
            PHRASE,
            2,
        ),
        (vec!["search", "--limit", "501"], PHRASE, 2),
    ] {
        let output = invoke(path, &args, phrase);
        assert_eq!(output.status.code(), Some(code));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!error.contains("synthetic") && !error.contains('@') && !error.contains("boleto"));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_triage-mail-cli"))
        .arg("--data-dir")
        .arg(path)
        .arg("accounts")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3)); // recovery marker forbids keyring use
    std::fs::remove_file(path.join("vault.enc")).unwrap();
    assert!(!invoke(path, &["accounts"], PHRASE).status.success());
    assert!(!path.join("vault.enc").exists());
    let absent = path.join("missing");
    assert_eq!(
        invoke(&absent, &["accounts"], PHRASE).status.code(),
        Some(4)
    );
    assert!(!absent.exists());
}

#[test]
fn executable_refuses_schema_key_and_output_failures_without_partial_json() {
    let fixture = fixture();
    let path = fixture.dir.path();
    let key = storage::DatabaseKey::from_bytes([23; 32]);
    let conn = storage::connection::open_encrypted_connection(path.join("mail.db"), &key).unwrap();
    conn.execute(
        "UPDATE messages SET subject=?1 WHERE id='m-a'",
        ["x".repeat(4 * 1024 * 1024)],
    )
    .unwrap();
    let output = invoke(path, &["search"], PHRASE);
    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    conn.execute(
        "UPDATE messages SET subject='boleto',text_body=?1 WHERE id='m-a'",
        ["\u{1b}[31m\n\0"],
    )
    .unwrap();
    let output = invoke(path, &["show", "--message-id", "m-a"], PHRASE);
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["text_plain"],
        "\u{1b}[31m\n\0"
    );
    conn.execute("DELETE FROM schema_migrations WHERE version=11", [])
        .unwrap();
    let output = invoke(path, &["accounts"], PHRASE);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert_eq!(
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        10
    );
    let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(PHRASE));
    let fallback =
        vault::EncryptedFallbackStore::file_backed(path.join("vault.enc"), cipher).unwrap();
    let id = VaultKey::for_account("application", "database-key").unwrap();
    fallback.set_secret(&id, &[25; 32]).unwrap();
    assert_eq!(invoke(path, &["accounts"], PHRASE).status.code(), Some(4));
    fallback.set_secret(&id, &[23; 31]).unwrap();
    assert_eq!(invoke(path, &["accounts"], PHRASE).status.code(), Some(3));
    fallback.delete_secret(&id).unwrap();
    assert_eq!(invoke(path, &["accounts"], PHRASE).status.code(), Some(3));
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_inputs_and_unlocks_non_recovered_fallback_explicitly() {
    let fixture = fixture();
    let path = fixture.dir.path();
    std::fs::remove_file(path.join("recovered-local-vault")).unwrap();
    assert!(invoke(path, &["accounts"], PHRASE).status.success());
    let alias = path.join("alias");
    std::os::unix::fs::symlink(path, &alias).unwrap();
    assert_eq!(invoke(&alias, &["accounts"], PHRASE).status.code(), Some(4));
    std::fs::rename(path.join("vault.enc"), path.join("original-vault")).unwrap();
    std::os::unix::fs::symlink(path.join("original-vault"), path.join("vault.enc")).unwrap();
    assert_eq!(invoke(path, &["accounts"], PHRASE).status.code(), Some(3));
}

#[cfg(target_os = "linux")]
fn invoke_container(image: &str, path: &Path, arguments: &[&str], phrase: &[u8]) -> Output {
    use std::os::unix::fs::MetadataExt;
    let owner = std::fs::metadata(path).unwrap();
    let mut child = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--pull=never",
            "--network=none",
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--log-driver=none",
            "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=16m,mode=1777",
            "--user",
            &format!("{}:{}", owner.uid(), owner.gid()),
            "--mount",
            &format!("type=bind,src={},dst=/data,readonly", path.display()),
            "-i",
            image,
            "--data-dir",
            "/data",
            "--unlock-stdin",
        ])
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Err(error) = child.stdin.take().unwrap().write_all(phrase) {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    child.wait_with_output().unwrap()
}

// INVARIANT: This opt-in test needs an already-built local image; Docker may never
// pull one. All mounted data is synthetic and the runtime has no external network.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires local Docker and TRIAGE_CLI_TEST_IMAGE; see docs/cli.md"]
fn container_reads_readonly_encrypted_wal_and_fails_closed() {
    let image = std::env::var("TRIAGE_CLI_TEST_IMAGE").expect("local image required");
    assert_container_image(&image);
    let fixture = fixture();
    let path = fixture.dir.path();
    assert!(path.join("mail.db-wal").metadata().unwrap().len() > 0);
    assert!(path.join("mail.db-shm").exists());
    // INVARIANT: Native WAL readers may update SHM read marks. Compare durable
    // data/vault bytes, while the container mount itself enforces read-only SHM.
    let files = [
        "mail.db",
        "mail.db-wal",
        "vault.enc",
        "recovered-local-vault",
    ];
    let before: Vec<_> = files
        .iter()
        .map(|f| std::fs::read(path.join(f)).unwrap())
        .collect();
    assert_container_json(&image, path);
    assert_container_unlock_and_sidecars(&image, path);
    for (file, original) in files.iter().zip(before) {
        assert!(
            std::fs::read(path.join(file)).unwrap() == original,
            "changed fixture file: {file}"
        );
    }
    assert!(
        !fixture
            .store
            .get_message_by_id("m-a")
            .unwrap()
            .unwrap()
            .is_read
    );
    let key = storage::DatabaseKey::from_bytes([23; 32]);
    let conn = storage::connection::open_encrypted_connection(path.join("mail.db"), &key).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pending_mutations", [], |r| r
            .get::<_, usize>(0))
            .unwrap(),
        1
    );
    assert_container_refuses_missing_and_schema(&image, path);
}

#[cfg(target_os = "linux")]
fn assert_container_image(image: &str) {
    let inspect = Command::new("docker")
        .args(["image", "inspect", image])
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let metadata: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(metadata[0]["Config"]["User"], "65532:65532");
    assert_eq!(
        metadata[0]["Config"]["Entrypoint"],
        serde_json::json!(["/usr/local/bin/triage-mail-cli"])
    );
    let help = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--pull=never",
            "--network=none",
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--log-driver=none",
            image,
        ])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("accounts"));
}

#[cfg(target_os = "linux")]
fn assert_container_json(image: &str, path: &Path) {
    for args in [
        vec!["accounts"],
        vec!["search", "--query", "boleto", "--account", "acct-a"],
        vec!["show", "--message-id", "m-a"],
    ] {
        let native = invoke(path, &args, PHRASE);
        let output = invoke_container(image, path, &args, PHRASE);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, native.stdout);
        assert!(output.stderr.is_empty());
    }
}

#[cfg(target_os = "linux")]
fn assert_container_unlock_and_sidecars(image: &str, path: &Path) {
    // A frozen synthetic DB+WAL copy lacks SHM; read-only mount cannot repair it.
    let missing_shm = tempfile::tempdir().unwrap();
    for file in [
        "mail.db",
        "mail.db-wal",
        "vault.enc",
        "recovered-local-vault",
    ] {
        std::fs::copy(path.join(file), missing_shm.path().join(file)).unwrap();
    }
    let rejected = invoke_container(image, missing_shm.path(), &["accounts"], PHRASE);
    assert_eq!(rejected.status.code(), Some(4));
    assert!(rejected.stdout.is_empty());
    assert!(!missing_shm.path().join("mail.db-shm").exists());
    for phrase in [b"".as_slice(), b"synthetic wrong phrase".as_slice()] {
        let output = invoke_container(image, path, &["accounts"], phrase);
        assert_eq!(output.status.code(), Some(3));
        assert!(output.stdout.is_empty());
    }
}

#[cfg(target_os = "linux")]
fn assert_container_refuses_missing_and_schema(image: &str, path: &Path) {
    let key = storage::DatabaseKey::from_bytes([23; 32]);
    let conn = storage::connection::open_encrypted_connection(path.join("mail.db"), &key).unwrap();
    conn.execute("DELETE FROM schema_migrations WHERE version=11", [])
        .unwrap();
    assert_eq!(
        invoke_container(image, path, &["accounts"], PHRASE)
            .status
            .code(),
        Some(4)
    );
    assert_eq!(
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        10
    );
    std::fs::remove_file(path.join("vault.enc")).unwrap();
    assert_eq!(
        invoke_container(image, path, &["accounts"], PHRASE)
            .status
            .code(),
        Some(3)
    );
    assert!(!path.join("vault.enc").exists());
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        invoke_container(image, empty.path(), &["accounts"], PHRASE)
            .status
            .code(),
        Some(4)
    );
    assert_eq!(std::fs::read_dir(empty.path()).unwrap().count(), 0);
}
