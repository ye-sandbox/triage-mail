use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use mail_engine::{
    sync::IncrementalSyncResult, AuthMethod, DiscoveredFolder, EngineError, ParsedInboundMessage,
};
use storage::{AccountRecord, DatabaseKey, FolderSyncState, SqliteMailStore};
use sync_runtime::{
    account::AccountConnectionConfig,
    daemon::{run_daemon, DaemonConfig, DaemonEvent},
    host::SyncTransport,
};

struct MockDaemonTransport {
    folders: Vec<DiscoveredFolder>,
    messages: Vec<ParsedInboundMessage>,
    sync_count: Arc<AtomicUsize>,
}

#[async_trait]
impl SyncTransport for MockDaemonTransport {
    async fn discover_folders(
        &self,
        _config: &AccountConnectionConfig,
    ) -> Result<Vec<DiscoveredFolder>, EngineError> {
        Ok(self.folders.clone())
    }

    async fn sync_incremental(
        &self,
        _config: &AccountConnectionConfig,
        checkpoint: &FolderSyncState,
    ) -> Result<IncrementalSyncResult, EngineError> {
        self.sync_count.fetch_add(1, Ordering::SeqCst);
        Ok(IncrementalSyncResult {
            folder: checkpoint.folder_name.clone(),
            uidvalidity: if checkpoint.uidvalidity == 0 {
                100
            } else {
                checkpoint.uidvalidity
            },
            uidvalidity_reset: false,
            server_uidnext: (self.messages.len() as u32) + 1,
            new_last_uid: self.messages.len() as u32,
            highest_modseq: None,
            new_messages: if checkpoint.uidvalidity == 0 {
                self.messages.clone()
            } else {
                Vec::new()
            },
            flag_deltas: Vec::new(),
        })
    }
}

fn sample_message(uid: u32, subject: &str) -> ParsedInboundMessage {
    ParsedInboundMessage {
        uid,
        modseq: None,
        provider_message_id: None,
        message_id_header: Some(format!("<msg-{uid}@example.org>")),
        in_reply_to: None,
        references: vec![],
        references_header: None,
        subject: subject.to_string(),
        sender: "sender@example.org".to_string(),
        recipients: vec!["user@example.com".to_string()],
        date_epoch_ms: Some(1_700_000_000),
        text_plain: Some(format!("Body for {subject}")),
        html_raw: None,
        raw_headers: Some("X-Synthetic: yes".into()),
        unsubscribe: None,
        attachments: vec![],
    }
}

fn setup_store(dir: &tempfile::TempDir) -> SqliteMailStore {
    let key = DatabaseKey::from_bytes([77; 32]);
    let db_path = dir.path().join("mail.db");
    SqliteMailStore::open(&db_path, &key, Default::default()).unwrap()
}

fn add_account(store: &SqliteMailStore, account_id: &str, email: &str) -> AccountConnectionConfig {
    let record = AccountRecord {
        id: account_id.to_string(),
        email: email.to_string(),
        auth_type: format!("{:?}", AuthMethod::AppPassword),
        created_at_epoch_ms: 1000,
        updated_at_epoch_ms: 1000,
    };
    let config = AccountConnectionConfig {
        account_id: account_id.to_string(),
        email_display: email.to_string(),
        imap_host: "imap.example.org".to_string(),
        imap_port: 993,
        auth_method: AuthMethod::AppPassword,
    };
    let json = serde_json::to_string(&config).unwrap();
    store.insert_account_connection(&record, &json).unwrap();
    config
}

#[tokio::test]
async fn test_daemon_bounded_iterations() {
    let dir = tempfile::tempdir().unwrap();
    let store = setup_store(&dir);
    add_account(&store, "acc_daemon", "daemon@example.org");

    let sync_counter = Arc::new(AtomicUsize::new(0));
    let transport = MockDaemonTransport {
        folders: vec![DiscoveredFolder {
            name: "INBOX".into(),
            display_name: "Inbox".into(),
            delimiter: Some("/".into()),
            attributes: Vec::new(),
            role: None,
            is_selectable: true,
        }],
        messages: vec![sample_message(1, "Welcome Daemon")],
        sync_count: sync_counter.clone(),
    };

    let config = DaemonConfig::default()
        .with_max_iterations(Some(3))
        .with_account_filter(Some("acc_daemon".into()));
    let config = DaemonConfig {
        interval_secs: 1,
        account_filter: config.account_filter,
        max_iterations: config.max_iterations,
    };

    let (_tx, rx) = tokio::sync::watch::channel(false);
    let mut events = Vec::new();

    let res = run_daemon(&store, &transport, config, rx, |evt| {
        events.push(evt);
    })
    .await;

    assert!(res.is_ok());
    assert_eq!(sync_counter.load(Ordering::SeqCst), 3);

    assert!(matches!(events.first(), Some(DaemonEvent::Started { .. })));
    assert!(matches!(
        events.last(),
        Some(DaemonEvent::Stopped {
            total_iterations: 3,
            ..
        })
    ));

    let tick_count = events
        .iter()
        .filter(|e| matches!(e, DaemonEvent::SyncTick { .. }))
        .count();
    assert_eq!(tick_count, 3);
}

#[tokio::test]
async fn test_daemon_graceful_shutdown_wakes_early() {
    let dir = tempfile::tempdir().unwrap();
    let store = setup_store(&dir);
    add_account(&store, "acc_early", "early@example.org");

    let transport = MockDaemonTransport {
        folders: vec![DiscoveredFolder {
            name: "INBOX".into(),
            display_name: "Inbox".into(),
            delimiter: Some("/".into()),
            attributes: Vec::new(),
            role: None,
            is_selectable: true,
        }],
        messages: vec![],
        sync_count: Arc::new(AtomicUsize::new(0)),
    };

    let config = DaemonConfig {
        interval_secs: 60,
        account_filter: None,
        max_iterations: None,
    };

    let (tx, rx) = tokio::sync::watch::channel(false);
    let mut events = Vec::new();

    let start = std::time::Instant::now();
    let shutdown_handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = tx.send(true);
    });

    let res = run_daemon(&store, &transport, config, rx, |evt| {
        events.push(evt);
    })
    .await;

    let _ = shutdown_handle.await;

    assert!(res.is_ok());
    assert!(start.elapsed().as_secs() < 5); // Must not wait 60 seconds

    assert!(events.iter().any(|e| matches!(
        e,
        DaemonEvent::Stopping { reason } if reason == "shutdown_signal"
    )));
    assert!(events
        .iter()
        .any(|e| matches!(e, DaemonEvent::Stopped { .. })));
}
