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
