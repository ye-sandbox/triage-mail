use async_trait::async_trait;
use mail_engine::{
    sync::IncrementalSyncResult, AuthMethod, DiscoveredFolder, EngineError, ParsedInboundMessage,
};
use storage::{AccountRecord, DatabaseKey, FolderSyncState, SqliteMailStore};
use sync_runtime::{
    account::AccountConnectionConfig, host::SyncTransport, sync_accounts, HeadlessSyncError,
};

struct MockTransport {
    folders: Vec<DiscoveredFolder>,
    messages: Vec<ParsedInboundMessage>,
}

#[async_trait]
impl SyncTransport for MockTransport {
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

fn setup_store(dir: &tempfile::TempDir) -> (SqliteMailStore, DatabaseKey) {
    let key = DatabaseKey::from_bytes([42; 32]);
    let db_path = dir.path().join("mail.db");
    let store = SqliteMailStore::open(&db_path, &key, Default::default()).unwrap();
    (store, key)
}

fn add_account(
    store: &SqliteMailStore,
    account_id: &str,
    email: &str,
    host: &str,
    port: u16,
    auth_method: AuthMethod,
) -> AccountConnectionConfig {
    let record = AccountRecord {
        id: account_id.to_string(),
        email: email.to_string(),
        auth_type: format!("{auth_method:?}"),
        created_at_epoch_ms: 1000,
        updated_at_epoch_ms: 1000,
    };
    let config = AccountConnectionConfig {
        account_id: account_id.to_string(),
        email_display: email.to_string(),
        imap_host: host.to_string(),
        imap_port: port,
        auth_method,
    };
    let json = serde_json::to_string(&config).unwrap();
    store.insert_account_connection(&record, &json).unwrap();
    config
}

#[tokio::test]
async fn test_sync_accounts_single_and_multi_account() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = setup_store(&dir);

    let _config_a = add_account(
        &store,
        "acct-1",
        "user1@example.com",
        "imap.example.com",
        993,
        AuthMethod::AppPassword,
    );
    let _config_b = add_account(
        &store,
        "acct-2",
        "user2@example.com",
        "imap.example.com",
        993,
        AuthMethod::AppPassword,
    );

    let transport = MockTransport {
        folders: vec![DiscoveredFolder {
            name: "INBOX".to_string(),
            display_name: "Inbox".to_string(),
            delimiter: Some("/".to_string()),
            attributes: Vec::new(),
            role: None,
            is_selectable: true,
        }],
        messages: vec![
            sample_message(1, "Invoice #123"),
            sample_message(2, "Boleto bancário"),
        ],
    };

    // 1. Sync specific account
    let summary = sync_accounts(&store, &transport, Some("acct-1"))
        .await
        .unwrap();
    assert_eq!(summary.accounts.len(), 1);
    assert_eq!(summary.accounts[0].account_id, "acct-1");
    assert_eq!(summary.accounts[0].synced_messages, 2);
    assert!(summary.accounts[0].error.is_none());
    assert_eq!(summary.total_synced_messages, 2);

    // Verify messages saved in store
    let msg1_id = store
        .find_message_id("acct-1", "INBOX", 1)
        .unwrap()
        .expect("msg 1 found");
    let msg1 = store
        .get_message_by_id(&msg1_id)
        .unwrap()
        .expect("msg 1 record found");
    assert_eq!(msg1.subject, "Invoice #123");

    let msg2_id = store
        .find_message_id("acct-1", "INBOX", 2)
        .unwrap()
        .expect("msg 2 found");
    let msg2 = store
        .get_message_by_id(&msg2_id)
        .unwrap()
        .expect("msg 2 record found");
    assert_eq!(msg2.subject, "Boleto bancário");

    // 2. Sync all accounts (acct-1 already has messages; acct-2 should ingest 2)
    let summary_all = sync_accounts(&store, &transport, None).await.unwrap();
    assert_eq!(summary_all.accounts.len(), 2);
    // acct-1 has already synced, so incremental query returns 0 new
    assert_eq!(summary_all.accounts[0].synced_messages, 0);
    // acct-2 gets 2 new messages
    assert_eq!(summary_all.accounts[1].synced_messages, 2);
    assert_eq!(summary_all.total_synced_messages, 2);
}

#[tokio::test]
async fn test_sync_accounts_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = setup_store(&dir);

    let transport = MockTransport {
        folders: Vec::new(),
        messages: Vec::new(),
    };

    let err = sync_accounts(&store, &transport, Some("non-existent"))
        .await
        .unwrap_err();
    match err {
        HeadlessSyncError::AccountNotFound(id) => assert_eq!(id, "non-existent"),
        other => panic!("expected AccountNotFound, got: {other:?}"),
    }
}

#[test]
fn test_account_connection_config_tls_policy() {
    // 1. Remote host with standard AppPassword TLS passes
    let valid_remote = AccountConnectionConfig {
        account_id: "acct-remote".to_string(),
        email_display: "user@example.com".to_string(),
        imap_host: "imap.example.com".to_string(),
        imap_port: 993,
        auth_method: AuthMethod::AppPassword,
    };
    let (endpoint, key) = valid_remote.into_endpoint_and_vault_key().unwrap();
    assert_eq!(endpoint.host, "imap.example.com");
    assert!(!endpoint.allow_loopback_self_signed_tls);
    assert_eq!(key.as_str(), "triage-mail:acct-remote:imap-auth");

    // 2. Proton Bridge on loopback allows self-signed TLS
    let proton_loopback = AccountConnectionConfig {
        account_id: "acct-proton".to_string(),
        email_display: "user@example.com".to_string(),
        imap_host: "127.0.0.1".to_string(),
        imap_port: 1143,
        auth_method: AuthMethod::ProtonBridgeLocal,
    };
    let (endpoint_proton, _) = proton_loopback.into_endpoint_and_vault_key().unwrap();
    assert!(endpoint_proton.allow_loopback_self_signed_tls);

    // 3. Proton Bridge on non-loopback host fails TLS policy
    let proton_remote = AccountConnectionConfig {
        account_id: "acct-insecure".to_string(),
        email_display: "user@example.com".to_string(),
        imap_host: "remote-bridge.example.com".to_string(),
        imap_port: 1143,
        auth_method: AuthMethod::ProtonBridgeLocal,
    };
    assert!(proton_remote.into_endpoint_and_vault_key().is_err());
}
