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

#[cfg(unix)]
#[test]
fn cached_cli_reads_committed_wal_while_another_process_owns_the_installation() {
    use std::os::unix::fs::OpenOptionsExt;

    let fixture = fixture();
    let path = fixture.dir.path();
    let mut name = std::ffi::OsString::from(".triage-mail-writer-");
    name.push(path.file_name().unwrap());
    name.push(".lock");
    let lock_path = path.parent().unwrap().join(name);
    // INVARIANT: Match the sibling ownership protocol without introducing a
    // runtime dependency into the offline reader. The native guard has separate
    // subprocess coverage; this checks the actual CLI against a held OS lock.
    let ownership = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&lock_path)
        .unwrap();
    ownership.try_lock().unwrap();
    let vault = std::fs::read(path.join("vault.enc")).unwrap();
    let output = invoke(path, &["search", "--query", "boleto"], PHRASE);
    assert!(output.status.success(), "{output:?}");
    let rows: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(vault, std::fs::read(path.join("vault.enc")).unwrap());
    assert!(std::fs::read(&lock_path).unwrap().is_empty());
    assert_eq!(fixture.store.pending_mutations().unwrap().len(), 1);
    drop(ownership);
    // Test-only teardown, after every possible owner has exited.
    std::fs::remove_file(lock_path).unwrap();
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

#[cfg(unix)]
#[test]
fn sync_refuses_with_exit_code_7_when_installation_is_already_owned() {
    use std::os::unix::fs::OpenOptionsExt;

    let fixture = fixture();
    let path = fixture.dir.path();
    let mut name = std::ffi::OsString::from(".triage-mail-writer-");
    name.push(path.file_name().unwrap());
    name.push(".lock");
    let lock_path = path.parent().unwrap().join(name);
    let ownership = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&lock_path)
        .unwrap();
    ownership.try_lock().unwrap();

    let output = invoke(path, &["sync"], PHRASE);
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("installation already owned by another writer"));

    drop(ownership);
    let _ = std::fs::remove_file(lock_path);
}

#[test]
fn sync_refuses_unknown_account_with_exit_code_5() {
    let fixture = fixture();
    let path = fixture.dir.path();

    let output = invoke(path, &["sync", "--account", "non-existent-account"], PHRASE);
    assert_eq!(output.status.code(), Some(5), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cached message or account not found"));
}

#[test]
fn sync_help_text_displays_synopsis_and_exit_codes() {
    let output = Command::new(env!("CARGO_BIN_EXE_triage-mail-cli"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sync: [--account OPAQUE_ID]"));
    assert!(stdout.contains("daemon: [--interval-secs 1..86400 (default 60)]"));
    assert!(stdout.contains("7 lock contention"));
    assert!(stdout.contains("8 remote error"));
}

#[cfg(unix)]
#[test]
fn daemon_refuses_with_exit_code_7_when_installation_is_already_owned() {
    use std::os::unix::fs::OpenOptionsExt;

    let fixture = fixture();
    let path = fixture.dir.path();
    let mut name = std::ffi::OsString::from(".triage-mail-writer-");
    name.push(path.file_name().unwrap());
    name.push(".lock");
    let lock_path = path.parent().unwrap().join(name);
    let ownership = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&lock_path)
        .unwrap();
    ownership.try_lock().unwrap();

    let output = invoke(path, &["daemon", "--interval-secs", "60"], PHRASE);
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("installation already owned by another writer"));

    drop(ownership);
    let _ = std::fs::remove_file(lock_path);
}

#[test]
fn daemon_refuses_unknown_account_with_exit_code_5() {
    let fixture = fixture();
    let path = fixture.dir.path();

    let output = invoke(
        path,
        &[
            "daemon",
            "--account",
            "non-existent-account",
            "--max-iterations",
            "1",
        ],
        PHRASE,
    );
    assert_eq!(output.status.code(), Some(5), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cached message or account not found"));
}

#[test]
fn daemon_refuses_invalid_interval_with_exit_code_2() {
    let fixture = fixture();
    let path = fixture.dir.path();

    let output = invoke(path, &["daemon", "--interval-secs", "0"], PHRASE);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn daemon_runs_with_max_iterations_and_emits_ndjson() {
    let fixture = fixture();
    let path = fixture.dir.path();

    let output = invoke(
        path,
        &["daemon", "--interval-secs", "1", "--max-iterations", "2"],
        PHRASE,
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 4,
        "expected at least 4 NDJSON lines, got {lines:?}"
    );

    assert!(lines[0].contains(r#""event":"started""#));
    assert!(lines[1].contains(r#""event":"sync_tick""#));
    assert!(lines[1].contains(r#""iteration":1"#));
    assert!(lines[2].contains(r#""event":"sync_tick""#));
    assert!(lines[2].contains(r#""iteration":2"#));
    assert!(lines[3].contains(r#""event":"stopping""#));
    assert!(lines[4].contains(r#""event":"stopped""#));
}

#[cfg(target_os = "linux")]
const MOCK_IMAP_SCRIPT: &str = r#"
import ssl, socket, sys, threading

ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain("/certs/cert.pem", "/certs/key.pem")

server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.bind(("127.0.0.1", 1143))
server.listen(5)
print("READY", flush=True)

def handle(raw):
    try:
        conn = ctx.wrap_socket(raw, server_side=True)
        conn.sendall(b"* OK [CAPABILITY IMAP4rev1 IDLE MOVE] IMAP ready\r\n")
        while True:
            line = b""
            while not line.endswith(b"\r\n"):
                b = conn.recv(1)
                if not b:
                    return
                line += b
            cmd = line.decode("ascii", "ignore")
            parts = cmd.strip().split()
            if not parts:
                continue
            tag = parts[0]
            action = " ".join(parts[1:]).upper()
            if "CAPABILITY" in action:
                conn.sendall(f"* CAPABILITY IMAP4rev1 IDLE MOVE\r\n{tag} OK CAPABILITY completed\r\n".encode())
            elif "LOGIN" in action:
                conn.sendall(f"{tag} OK LOGIN completed\r\n".encode())
            elif "LIST" in action:
                conn.sendall(f"* LIST (\\HasNoChildren) \"/\" \"INBOX\"\r\n{tag} OK LIST completed\r\n".encode())
            elif "SELECT" in action:
                conn.sendall(f"* 1 EXISTS\r\n* OK [UIDVALIDITY 100] UIDs valid\r\n* OK [UIDNEXT 2]\r\n{tag} OK [READ-WRITE] SELECT completed\r\n".encode())
            elif "SEARCH" in action:
                conn.sendall(f"* SEARCH 1\r\n{tag} OK UID SEARCH completed\r\n".encode())
            elif "FETCH" in action and "BODYSTRUCTURE" in action:
                hdr = "From: billing@example.org\r\nTo: user@example.com\r\nSubject: Synthetic Inbound Boleto\r\nMessage-ID: <invoice-101@example.org>\r\nDate: Mon, 01 Jan 2026 00:00:00 +0000\r\n\r\n"
                bs = "(\"TEXT\" \"PLAIN\" (\"CHARSET\" \"UTF-8\") NIL NIL \"7BIT\" 14 1)"
                conn.sendall(f"* 1 FETCH (UID 1 FLAGS (\\Seen) BODYSTRUCTURE {bs} BODY[HEADER] {{{len(hdr)}}}\r\n{hdr})\r\n{tag} OK UID FETCH completed\r\n".encode())
            elif "FETCH" in action and ("BODY" in action or "PEEK" in action):
                body = "Boleto payload"
                conn.sendall(f"* 1 FETCH (UID 1 BODY[1] {{{len(body)}}}\r\n{body})\r\n{tag} OK UID FETCH completed\r\n".encode())
            elif "FETCH" in action and "FLAGS" in action:
                conn.sendall(f"* 1 FETCH (UID 1 FLAGS (\\Seen))\r\n{tag} OK UID FETCH completed\r\n".encode())
            elif "LOGOUT" in action:
                conn.sendall(f"* BYE Logging out\r\n{tag} OK LOGOUT completed\r\n".encode())
                break
            else:
                conn.sendall(f"{tag} OK done\r\n".encode())
    except Exception:
        pass

while True:
    try:
        r, _ = server.accept()
        threading.Thread(target=handle, args=(r,)).start()
    except Exception:
        break
"#;

#[cfg(target_os = "linux")]
struct MockImapServer {
    container_name: String,
    _cert_dir: tempfile::TempDir,
    child: std::process::Child,
    port: u16,
}

#[cfg(target_os = "linux")]
impl MockImapServer {
    fn start() -> Self {
        use std::io::BufRead;
        let cert_dir = tempfile::tempdir().unwrap();
        let key_path = cert_dir.path().join("key.pem");
        let cert_path = cert_dir.path().join("cert.pem");
        let openssl = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                key_path.to_str().unwrap(),
                "-out",
                cert_path.to_str().unwrap(),
                "-days",
                "1",
                "-subj",
                "/CN=127.0.0.1",
            ])
            .output()
            .expect("openssl required");
        assert!(openssl.status.success(), "openssl cert generation failed");

        let container_name = format!("mock-imap-{}", std::process::id());
        let mut child = Command::new("docker")
            .args([
                "run",
                "--rm",
                &format!("--name={container_name}"),
                "--pull=never",
                "-v",
                &format!("{}:/certs:ro", cert_dir.path().display()),
                "python:3-alpine",
                "python3",
                "-c",
                MOCK_IMAP_SCRIPT,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("docker python:3-alpine container required for opt-in mock IMAP test");

        let stdout = child.stdout.as_mut().expect("piped stdout");
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("read READY from mock IMAP");
        assert!(line.contains("READY"), "mock IMAP failed to start: {line}");

        Self {
            container_name,
            _cert_dir: cert_dir,
            child,
            port: 1143,
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for MockImapServer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["stop", "-t", "1", &self.container_name])
            .status();
        let _ = self.child.wait();
    }
}

#[cfg(target_os = "linux")]
struct WritableFixture {
    parent: tempfile::TempDir,
    app_dir: std::path::PathBuf,
    #[allow(dead_code)]
    store: storage::SqliteMailStore,
    key: storage::DatabaseKey,
}

#[cfg(target_os = "linux")]
fn writable_fixture(port: u16) -> WritableFixture {
    let parent = tempfile::tempdir().unwrap();
    let app_dir = parent.path().join("app");
    std::fs::create_dir_all(&app_dir).unwrap();
    let key = storage::DatabaseKey::from_bytes([23; 32]);
    let store =
        storage::SqliteMailStore::open(app_dir.join("mail.db"), &key, Default::default()).unwrap();
    let conn =
        storage::connection::open_encrypted_connection(app_dir.join("mail.db"), &key).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA wal_autocheckpoint=0;
         INSERT INTO accounts VALUES ('acct-test','user@example.com','ProtonBridgeLocal',0,0);
         INSERT INTO account_connections VALUES ('acct-test','{{\"account_id\":\"acct-test\",\"email_display\":\"user@example.com\",\"imap_host\":\"127.0.0.1\",\"imap_port\":{port},\"auth_method\":\"ProtonBridgeLocal\"}}');"
    ))
    .unwrap();
    let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(PHRASE));
    let fallback =
        vault::EncryptedFallbackStore::file_backed(app_dir.join("vault.enc"), cipher).unwrap();
    fallback
        .set_secret(
            &VaultKey::for_account("application", "database-key").unwrap(),
            &[23; 32],
        )
        .unwrap();
    let auth = vault::AccountAuthMaterial::ProtonBridgeLocal {
        bridge_password: "bridge-secret".to_string(),
    };
    fallback
        .set_secret(
            &VaultKey::for_account("acct-test", "imap-auth").unwrap(),
            &auth.to_bytes().unwrap(),
        )
        .unwrap();
    WritableFixture {
        parent,
        app_dir,
        store,
        key,
    }
}

#[cfg(target_os = "linux")]
fn invoke_writable_container(
    image: &str,
    parent_path: &Path,
    app_rel: &str,
    network: &str,
    arguments: &[&str],
    phrase: &[u8],
) -> Output {
    use std::os::unix::fs::MetadataExt;
    let owner = std::fs::metadata(parent_path).unwrap();
    let mut child = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--pull=never",
            &format!("--network={network}"),
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--log-driver=none",
            "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=16m,mode=1777",
            "--user",
            &format!("{}:{}", owner.uid(), owner.gid()),
            "--mount",
            &format!("type=bind,src={},dst=/data", parent_path.display()),
            "-i",
            image,
            "--data-dir",
            &format!("/data/{app_rel}"),
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

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires local Docker and TRIAGE_CLI_TEST_IMAGE; see docs/cli.md"]
fn container_writable_refuses_when_host_owns_lock() {
    use std::os::unix::fs::OpenOptionsExt;

    let image = std::env::var("TRIAGE_CLI_TEST_IMAGE").expect("local image required");
    let fixture = writable_fixture(0);
    let lock_path = fixture.parent.path().join(".triage-mail-writer-app.lock");
    let ownership = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&lock_path)
        .unwrap();
    ownership.try_lock().unwrap();

    let output = invoke_writable_container(
        &image,
        fixture.parent.path(),
        "app",
        "none",
        &["sync"],
        PHRASE,
    );
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("installation already owned by another writer"));

    let daemon_output = invoke_writable_container(
        &image,
        fixture.parent.path(),
        "app",
        "none",
        &["daemon", "--interval-secs", "60"],
        PHRASE,
    );
    assert_eq!(daemon_output.status.code(), Some(7), "{daemon_output:?}");
    let daemon_stderr = String::from_utf8_lossy(&daemon_output.stderr);
    assert!(daemon_stderr.contains("installation already owned by another writer"));

    drop(ownership);
    assert!(std::fs::read(&lock_path).unwrap().is_empty());
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires local Docker and TRIAGE_CLI_TEST_IMAGE; see docs/cli.md"]
fn container_daemon_runs_and_drains_on_sigterm() {
    use std::io::{BufRead, Read};
    use std::os::unix::fs::MetadataExt;

    let image = std::env::var("TRIAGE_CLI_TEST_IMAGE").expect("local image required");
    let fixture = writable_fixture(0);
    let owner = std::fs::metadata(fixture.parent.path()).unwrap();
    let container_name = format!("triage-cli-test-daemon-{}", std::process::id());

    let mut child = Command::new("docker")
        .args([
            "run",
            "--rm",
            &format!("--name={container_name}"),
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
            &format!(
                "type=bind,src={},dst=/data",
                fixture.parent.path().display()
            ),
            "-i",
            &image,
            "--data-dir",
            "/data/app",
            "--unlock-stdin",
            "daemon",
            "--interval-secs",
            "60",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.take().unwrap().write_all(PHRASE).unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).unwrap();
    assert!(
        first_line.contains(r#""event":"started""#),
        "expected started event, got: {first_line}"
    );

    let lock_path = fixture.parent.path().join(".triage-mail-writer-app.lock");
    assert!(lock_path.exists());
    let host_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    assert!(matches!(
        host_lock.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));

    let stop = Command::new("docker")
        .args(["stop", "-t", "10", &container_name])
        .status()
        .unwrap();
    assert!(stop.success(), "docker stop failed");

    let mut remaining = String::new();
    reader.read_to_string(&mut remaining).unwrap();
    let full_stdout = format!("{first_line}{remaining}");
    assert!(full_stdout.contains(r#""event":"stopping""#));
    assert!(full_stdout.contains(r#""event":"stopped""#));

    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "child container did not exit with code 0: {status:?}"
    );
    assert!(
        host_lock.try_lock().is_ok(),
        "lock was not released on daemon shutdown"
    );
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires local Docker and TRIAGE_CLI_TEST_IMAGE; see docs/cli.md"]
fn container_writable_sync_commits_and_releases_lock() {
    let image = std::env::var("TRIAGE_CLI_TEST_IMAGE").expect("local image required");
    let mock = MockImapServer::start();
    let fixture = writable_fixture(mock.port);

    let output = invoke_writable_container(
        &image,
        fixture.parent.path(),
        "app",
        &format!("container:{}", mock.container_name),
        &["sync", "--account", "acct-test"],
        PHRASE,
    );
    assert!(
        output.status.success(),
        "sync failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["total_synced_messages"], 1);
    assert_eq!(summary["accounts"][0]["account_id"], "acct-test");
    assert_eq!(summary["accounts"][0]["synced_messages"], 1);
    assert!(summary["accounts"][0]["error"].is_null());

    let conn = storage::connection::open_encrypted_connection(
        fixture.app_dir.join("mail.db"),
        &fixture.key,
    )
    .unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE account_id='acct-test'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let subject: String = conn
        .query_row(
            "SELECT subject FROM messages WHERE account_id='acct-test'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(subject, "Synthetic Inbound Boleto");

    let lock_path = fixture.parent.path().join(".triage-mail-writer-app.lock");
    assert!(lock_path.exists());
    let test_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    assert!(
        test_lock.try_lock().is_ok(),
        "lock was not released by sync"
    );
}
