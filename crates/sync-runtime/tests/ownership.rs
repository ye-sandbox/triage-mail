#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::{symlink, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use sync_runtime::ownership::{OwnershipError, WriterOwnership};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "triage-ownership-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn installation(&self) -> PathBuf {
        self.0.join("installation")
    }
    fn lock(&self) -> PathBuf {
        self.0.join(".triage-mail-writer-installation.lock")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn command(path: &Path, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "process_helper", "--nocapture"])
        .env("TRIAGE_OWNERSHIP_FIXTURE", path)
        .env("TRIAGE_OWNERSHIP_MODE", mode);
    command
}

fn holding_process(path: &Path) -> Process {
    let mut child = Process(
        command(path, "hold")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut output = BufReader::new(child.0.stdout.as_mut().unwrap());
    let mut line = String::new();
    loop {
        assert_ne!(
            output.read_line(&mut line).unwrap(),
            0,
            "helper exited before ownership"
        );
        if line.trim() == "ownership-ready" {
            break;
        }
        line.clear();
    }
    child
}

#[test]
fn process_helper() {
    let Some(path) = std::env::var_os("TRIAGE_OWNERSHIP_FIXTURE") else {
        return;
    };
    let path = Path::new(&path);
    if std::env::var("TRIAGE_OWNERSHIP_MODE").unwrap() == "busy" {
        assert!(matches!(
            WriterOwnership::acquire(path),
            Err(OwnershipError::Busy)
        ));
        // A rejected contender never reaches its stand-in provisioning operation.
        return;
    }
    let _guard = WriterOwnership::acquire(path).unwrap();
    println!("ownership-ready");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_exact(&mut [0]).unwrap();
}

#[test]
fn process_contention_clean_and_abrupt_exit_reuse_the_same_empty_inode() {
    let fixture = Fixture::new();
    let path = fixture.installation();
    let mut child = holding_process(&path);
    let inode = fs::metadata(fixture.lock()).unwrap().ino();
    let output = command(&path, "busy").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!path.exists());
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Busy)
    ));
    child.0.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(child.0.wait().unwrap().success());
    let guard = WriterOwnership::acquire(&path).unwrap();
    assert_eq!(inode, fs::metadata(fixture.lock()).unwrap().ino());
    drop(guard);
    let mut child = holding_process(&path);
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let _guard = WriterOwnership::acquire(&path).unwrap();
    assert_eq!(inode, fs::metadata(fixture.lock()).unwrap().ino());
    assert!(fs::read(fixture.lock()).unwrap().is_empty());
}

#[test]
fn independent_installations_and_parent_aliases_have_consistent_identity() {
    let fixture = Fixture::new();
    let path = fixture.installation();
    let _first = WriterOwnership::acquire(&path).unwrap();
    let _second = WriterOwnership::acquire(&fixture.0.join("another")).unwrap();
    symlink(&fixture.0, fixture.0.join("alias")).unwrap();
    assert!(matches!(
        WriterOwnership::acquire(&fixture.0.join("alias/installation")),
        Err(OwnershipError::Busy)
    ));
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Busy)
    ));
    assert_eq!(fs::metadata(fixture.lock()).unwrap().mode() & 0o777, 0o600);
}

#[test]
fn invalid_targets_and_lock_files_are_refused_without_repair() {
    let fixture = Fixture::new();
    let path = fixture.installation();
    let lock = fixture.lock();
    let sentinel = fixture.0.join("sentinel");
    fs::write(&sentinel, b"private fixture sentinel").unwrap();
    symlink(&sentinel, &lock).unwrap();
    assert!(matches!(
        WriterOwnership::acquire(&path),
        Err(OwnershipError::Unavailable)
    ));
    assert_eq!(fs::read(&sentinel).unwrap(), b"private fixture sentinel");
    fs::remove_file(&lock).unwrap();
    fs::create_dir(&lock).unwrap();
    assert!(WriterOwnership::acquire(&path).is_err());
    fs::remove_dir(&lock).unwrap();
    fs::write(&lock, b"").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(WriterOwnership::acquire(&path).is_err());
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&lock, b"unexpected metadata").unwrap();
    assert!(WriterOwnership::acquire(&path).is_err());
    fs::write(&lock, b"").unwrap();
    fs::hard_link(&lock, fixture.0.join("hardlink")).unwrap();
    assert!(WriterOwnership::acquire(&path).is_err());
    fs::remove_file(fixture.0.join("hardlink")).unwrap();
    symlink(&fixture.0, &path).unwrap();
    assert!(WriterOwnership::acquire(&path).is_err());
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"occupied").unwrap();
    assert!(WriterOwnership::acquire(&path).is_err());
    assert!(WriterOwnership::acquire(Path::new("relative")).is_err());
    assert!(WriterOwnership::acquire(&fixture.0.join("../traversal")).is_err());
    assert!(WriterOwnership::acquire(Path::new("/")).is_err());
    assert_eq!(
        OwnershipError::Busy.to_string(),
        "installation already owned"
    );
    assert_eq!(
        OwnershipError::Unavailable.to_string(),
        "installation ownership unavailable"
    );
}
