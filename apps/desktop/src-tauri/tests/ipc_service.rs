use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use mail_engine::sync::IncrementalSyncResult;
use mail_engine::{AuthMethod, SyncEventSink, UnsubscribeDescriptor};
use storage::{
    AccountRecord, AttachmentRecord, DatabaseKey, FolderSyncState, MailRepository, MessageRecord,
    SearchEngine, SearchMode, SqliteMailStore,
};
use triage_ai::{EmbeddingProvider, TriageError};
use triage_mail_desktop_lib::remote::{MutationOutcome, RemoteMail, RemoteMutation};
use triage_mail_desktop_lib::service::IncomingCredentials;
use triage_mail_desktop_lib::*;
use vault::{AccountAuthMaterial, SecretStore, VaultKey};

#[derive(Default)]
struct FakeRemote {
    attachments: AtomicUsize,
    mutations: Mutex<Vec<RemoteMutation>>,
    unsubscribes: AtomicUsize,
    fail: bool,
    fail_message: Option<String>,
    sync_result: Mutex<Option<IncrementalSyncResult>>,
    test_connection_error: Mutex<Option<IpcError>>,
    quota: Mutex<Option<mail_engine::AccountStorageQuota>>,
}

#[async_trait]
impl RemoteMail for FakeRemote {
    async fn fetch_quota(
        &self,
        _: &AccountConfigDto,
    ) -> Result<Option<mail_engine::AccountStorageQuota>, IpcError> {
        Ok(*self.quota.lock().unwrap())
    }
    async fn fetch_attachment(
        &self,
        _: &AccountConfigDto,
        _: &str,
        _: u32,
        _: &str,
        _: u32,
    ) -> Result<Vec<u8>, IpcError> {
        self.attachments.fetch_add(1, Ordering::SeqCst);
        Ok(vec![1, 2, 3])
    }
    async fn mutate(
        &self,
        _: &AccountConfigDto,
        mutation: &RemoteMutation,
    ) -> Result<MutationOutcome, IpcError> {
        self.mutations.lock().unwrap().push(mutation.clone());
        if self.fail || self.fail_message.as_ref() == Some(&mutation.message_id) {
            return Err(IpcError::RemoteUnavailable);
        }
        match mutation.action {
            TriageActionDto::Archive => Ok(MutationOutcome::Moved {
                folder: "Archive".into(),
                uid: Some(77),
                uidvalidity: Some(17),
            }),
            TriageActionDto::Trash => Ok(MutationOutcome::Moved {
                folder: "Trash".into(),
                uid: Some(88),
                uidvalidity: Some(27),
            }),
            _ => Ok(MutationOutcome::FlagsUpdated),
        }
    }
    async fn unsubscribe(&self, _: &UnsubscribeDescriptor) -> Result<(), IpcError> {
        self.unsubscribes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn sync(
        &self,
        _: &AccountConfigDto,
        _: &FolderSyncState,
        sink: &dyn SyncEventSink,
    ) -> Result<IncrementalSyncResult, IpcError> {
        let result = self
            .sync_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(IpcError::Unsupported)?;
        sink.on_sync_progress(&mail_engine::SyncProgressEvent {
            account_id: "acct-1".into(),
            folder: result.folder.clone(),
            synced_count: result.new_messages.len(),
            uidvalidity: result.uidvalidity,
            highest_modseq: result.highest_modseq,
        })
        .map_err(|_| IpcError::Internal)?;
        Ok(result)
    }
    async fn wait_for_change(
        &self,
        _: &AccountConfigDto,
        _: &FolderSyncState,
    ) -> Result<bool, IpcError> {
        Ok(false)
    }
    async fn discover_folders(
        &self,
        _: &AccountConfigDto,
    ) -> Result<Vec<mail_engine::DiscoveredFolder>, IpcError> {
        Ok(vec![mail_engine::DiscoveredFolder {
            name: "INBOX".into(),
            display_name: "INBOX".into(),
            delimiter: Some("/".into()),
            attributes: vec!["\\HasNoChildren".into()],
            role: Some(mail_engine::SpecialFolderRole::Inbox),
            is_selectable: true,
        }])
    }
    async fn test_connection(
        &self,
        _: &AccountConfigDto,
        _: &IncomingCredentials,
    ) -> Result<(), IpcError> {
        match *self.test_connection_error.lock().unwrap() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}
struct FakeEmbedding;
impl EmbeddingProvider for FakeEmbedding {
    fn embed_int8(&self, _: &str) -> Result<Vec<i8>, TriageError> {
        Ok(vec![3; 384])
    }
}

fn config(id: &str) -> AccountConfigDto {
    AccountConfigDto {
        account_id: id.into(),
        email_display: "reader@example.com".into(),
        imap_host: "imap.example.com".into(),
        imap_port: 993,
        auth_method: AuthMethod::AppPassword,
    }
}
fn credentials() -> IncomingCredentials {
    IncomingCredentials(AccountAuthMaterial::AppPassword {
        secret: format!("synthetic-{}", 42),
    })
}
fn vault() -> Arc<dyn SecretStore> {
    Arc::new(vault::EncryptedFallbackStore::in_memory(Arc::new(
        vault::Argon2AesGcmCipher::from_key([19; 32]),
    )))
}
fn store() -> Arc<SqliteMailStore> {
    Arc::new(
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([23; 32]), Default::default())
            .unwrap(),
    )
}
fn setup<R: RemoteMail + 'static>(
    remote: Arc<R>,
) -> (
    Arc<DesktopService>,
    Arc<SqliteMailStore>,
    Arc<dyn SecretStore>,
) {
    let store = store();
    let vault = vault();
    (
        DesktopService::new(
            store.clone(),
            vault.clone(),
            remote,
            Some(Arc::new(FakeEmbedding)),
        ),
        store,
        vault,
    )
}
fn record() -> MessageRecord {
    MessageRecord { id:"m-1".into(),account_id:"acct-1".into(),folder:"INBOX".into(),uid:1,message_id_header:Some("<message@example.org>".into()),in_reply_to:None,references_header:None,subject:"Invoice update".into(),sender:"billing@example.org".into(),recipients:"reader@example.com".into(),received_at_epoch_ms:1,is_read:false,has_attachments:true,category_badge:Some("Financial".into()),triage_badges:vec![triage_ai::TriageBadge::UnsubscribeAvailable { supports_one_click: true }],raw_headers:"From: billing@example.org\r\nSubject: Invoice update\r\nList-Unsubscribe: <https://example.org/unsubscribe>\r\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n".into(),text_body:Some("Invoice for order".into()),html_body:Some("<p>Invoice</p>".into()),list_unsubscribe:Some("<https://example.org/unsubscribe>".into()),list_unsubscribe_post:Some("List-Unsubscribe=One-Click".into()),one_click_unsubscribe_supported:true,created_at_epoch_ms:1,attachments:vec![AttachmentRecord { id:"a-1".into(),message_id:"m-1".into(),part_id:"2".into(),filename:"invoice.pdf".into(),content_type:"application/pdf".into(),size_bytes:3,content_id:None,is_inline:false }],tags:vec![],is_starred:false,snooze_until_epoch_ms:None,provider_message_id:None }
}
async fn seed(service: &Arc<DesktopService>, store: &SqliteMailStore) {
    service
        .add_account(config("acct-1"), credentials())
        .await
        .unwrap();
    store.insert_message(&record()).unwrap();
    store
        .upsert_folder_sync_state(&FolderSyncState {
            account_id: "acct-1".into(),
            folder_name: "INBOX".into(),
            uidvalidity: 7,
            uidnext: 2,
            highest_modseq: Some(8),
        })
        .unwrap();
}
fn query(mode: SearchMode) -> SearchQueryDto {
    SearchQueryDto {
        query: "Invoice".into(),
        mode,
        limit: 20,
        offset: 0,
        scope: None,
        recipient: None,
    }
}
fn mutation(action: TriageActionDto) -> TriageMutationDto {
    TriageMutationDto {
        message_ids: vec!["m-1".into()],
        action,
    }
}

#[tokio::test]
async fn scoped_search_preserves_global_requests_and_rejects_unknown_accounts() {
    let (service, store, _) = setup(Arc::new(FakeRemote::default()));
    seed(&service, &store).await;
    let unscoped = service
        .search_messages(query(SearchMode::Lexical))
        .await
        .unwrap();
    let mut request = query(SearchMode::Lexical);
    request.scope = Some(MailboxScopeDto::GlobalAllMail);
    assert_eq!(
        service.search_messages(request.clone()).await.unwrap(),
        unscoped
    );
    request.scope = Some(MailboxScopeDto::AccountView {
        account_id: "unknown".into(),
        view: MailboxViewDto::Starred,
    });
    assert_eq!(
        service.search_messages(request.clone()).await,
        Err(IpcError::InvalidRequest)
    );
    request.scope = Some(MailboxScopeDto::AccountView {
        account_id: "acct-1".into(),
        view: MailboxViewDto::Starred,
    });
    assert!(service
        .search_messages(request.clone())
        .await
        .unwrap()
        .is_empty());
    service
        .triage_action(mutation(TriageActionDto::Star))
        .await
        .unwrap();
    assert_eq!(service.search_messages(request).await.unwrap()[0].id, "m-1");
}

#[tokio::test]
async fn accounts_store_credentials_in_vault_and_never_echo_them() {
    let (service, store, vault) = setup(Arc::new(FakeRemote::default()));
    assert_eq!(
        service
            .add_account(config("acct-1"), credentials())
            .await
            .unwrap(),
        "acct-1"
    );
    let wire = serde_json::to_string(&service.list_accounts().await.unwrap()).unwrap();
    assert!(!wire.contains("synthetic-42"));
    assert!(!wire.contains("secret"));
    let key = VaultKey::for_account("acct-1", "imap-auth").unwrap();
    assert!(matches!(
        AccountAuthMaterial::from_bytes(&vault.get_secret(&key).unwrap()).unwrap(),
        AccountAuthMaterial::AppPassword { .. }
    ));
    assert!(!store
        .list_account_connections()
        .unwrap()
        .join("")
        .contains("synthetic-42"));
    assert_eq!(
        service.add_account(config("acct-1"), credentials()).await,
        Err(IpcError::AccountExists)
    );
    // Different ID, duplicate email: metadata transaction fails and vault write is compensated.
    assert_eq!(
        service.add_account(config("acct-2"), credentials()).await,
        Err(IpcError::StorageUnavailable)
    );
    assert!(matches!(
        vault.get_secret(&VaultKey::for_account("acct-2", "imap-auth").unwrap()),
        Err(vault::VaultError::NotFound(_))
    ));
}

#[tokio::test]
async fn validates_all_auth_variants_and_rejects_mode_mismatch_and_remote_proton() {
    let (service, _, _) = setup(Arc::new(FakeRemote::default()));
    for (i, method, material) in [
        (
            1,
            AuthMethod::OAuth2PublicPkce,
            AccountAuthMaterial::OAuth2PublicPkce {
                client_id: "synthetic-public".into(),
                access_token: "synthetic-access".into(),
                refresh_token: None,
                token_endpoint: None,
            },
        ),
        (
            2,
            AuthMethod::OAuth2ByoCredentials,
            AccountAuthMaterial::OAuth2ByoCredentials {
                client_id: "synthetic-byo".into(),
                client_secret: "synthetic-secret".into(),
                access_token: "synthetic-access".into(),
                refresh_token: None,
                token_endpoint: None,
            },
        ),
        (
            3,
            AuthMethod::ProtonBridgeLocal,
            AccountAuthMaterial::ProtonBridgeLocal {
                bridge_password: "synthetic-bridge".into(),
            },
        ),
    ] {
        let mut c = config(&format!("acct-{i}"));
        c.auth_method = method;
        c.email_display = format!("reader{i}@example.com");
        if method == AuthMethod::ProtonBridgeLocal {
            c.imap_host = "127.0.0.1".into();
        }
        service
            .add_account(c, IncomingCredentials(material))
            .await
            .unwrap();
    }
    assert_eq!(
        service
            .add_account(
                config("bad"),
                IncomingCredentials(AccountAuthMaterial::ProtonBridgeLocal {
                    bridge_password: "synthetic".into()
                })
            )
            .await,
        Err(IpcError::InvalidRequest)
    );
    let mut c = config("remote-proton");
    c.auth_method = AuthMethod::ProtonBridgeLocal;
    assert_eq!(
        service
            .add_account(
                c,
                IncomingCredentials(AccountAuthMaterial::ProtonBridgeLocal {
                    bridge_password: "synthetic".into()
                })
            )
            .await,
        Err(IpcError::InvalidRequest)
    );
    let mut c = config("opaque@example.org");
    c.email_display = "another@example.org".into();
    assert_eq!(
        service.add_account(c, credentials()).await,
        Err(IpcError::InvalidRequest)
    );
}

#[tokio::test]
async fn lexical_semantic_hybrid_and_details_map_existing_storage_contracts() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    store
        .insert_message_embedding("m-1", &vec![3; 384])
        .unwrap();
    // INVARIANT: Semantic and Hybrid search fail closed when experimental_semantic_search is disabled.
    assert_eq!(
        service.search_messages(query(SearchMode::Semantic)).await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service.search_messages(query(SearchMode::Hybrid)).await,
        Err(IpcError::InvalidRequest)
    );
    service
        .set_experimental_semantic_search(true)
        .await
        .unwrap();
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let rows = service.search_messages(query(mode)).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "m-1");
        assert_eq!(
            rows[0].triage_badges,
            vec![triage_ai::TriageBadge::UnsubscribeAvailable {
                supports_one_click: true
            }]
        );
    }
    let body = service.get_message_body("m-1".into()).await.unwrap();
    assert!(body.headers.contains("List-Unsubscribe"));
    assert_eq!(body.html_raw.as_deref(), Some("<p>Invoice</p>"));
    assert_eq!(body.triage_badges, body.triage_result.badges);
    assert_eq!(body.attachments_meta[0].part_id, "2");
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
    let mut invalid = query(SearchMode::Lexical);
    invalid.limit = 501;
    assert_eq!(
        service.search_messages(invalid).await,
        Err(IpcError::InvalidRequest)
    );
}

#[tokio::test]
async fn triage_is_atomic_durable_and_preserves_original_server_identity() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    store
        .insert_message_embedding("m-1", &vec![3; 384])
        .unwrap();
    let invalid = TriageMutationDto {
        message_ids: vec!["m-1".into(), "missing".into()],
        action: TriageActionDto::MarkRead,
    };
    assert_eq!(
        service.triage_action(invalid).await,
        Err(IpcError::NotFound)
    );
    assert!(!store.get_message_by_id("m-1").unwrap().unwrap().is_read);
    assert!(store.pending_mutations().unwrap().is_empty());
    service
        .triage_action(mutation(TriageActionDto::MarkRead))
        .await
        .unwrap();
    assert!(store.get_message_by_id("m-1").unwrap().unwrap().is_read);
    assert_eq!(store.pending_mutations().unwrap().len(), 1);
    let checkpoint = store
        .get_folder_sync_state("acct-1", "INBOX")
        .unwrap()
        .unwrap();
    store
        .commit_sync_batch(
            &[],
            &[storage::MessageFlagsDelta {
                uid: 1,
                is_read: false,
                is_starred: false,
            }],
            &checkpoint,
            false,
        )
        .unwrap();
    assert!(store.get_message_by_id("m-1").unwrap().unwrap().is_read);
    triage_mail_desktop_lib::worker::process_outbox_once(&service)
        .await
        .unwrap();
    assert!(store.pending_mutations().unwrap().is_empty());
    assert_eq!(remote.mutations.lock().unwrap()[0].uidvalidity, 7);
    service
        .triage_action(mutation(TriageActionDto::Tag("Finance".into())))
        .await
        .unwrap();
    assert_eq!(
        store.get_message_by_id("m-1").unwrap().unwrap().tags,
        vec!["Finance"]
    );
    service
        .triage_action(mutation(TriageActionDto::Archive))
        .await
        .unwrap();
    service
        .set_experimental_semantic_search(true)
        .await
        .unwrap();
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let rows = service.search_messages(query(mode)).await.unwrap();
        assert_eq!(rows[0].folder, "INBOX");
        assert_eq!(rows[0].uid, 1);
        assert_eq!(rows[0].folder_override.as_deref(), Some("Archive"));
        assert_eq!(rows[0].tags, vec!["Finance"]);
        let wire = serde_json::to_value(&rows[0]).unwrap();
        assert_eq!(wire["folder_override"], "Archive");
        assert_eq!(wire["tags"][0], "Finance");
        let mut legacy = wire;
        legacy.as_object_mut().unwrap().remove("tags");
        legacy.as_object_mut().unwrap().remove("folder_override");
        let decoded: storage::MessageRowSummary = serde_json::from_value(legacy).unwrap();
        assert!(decoded.tags.is_empty());
        assert_eq!(decoded.folder_override, None);
    }
    assert_eq!(
        store.get_message_by_id("m-1").unwrap().unwrap().folder,
        "INBOX"
    );
}

#[tokio::test]
async fn failed_remote_action_stays_queued_and_reports_a_safe_event() {
    let remote = Arc::new(FakeRemote {
        fail: true,
        ..Default::default()
    });
    let (service, store, _) = setup(remote);
    seed(&service, &store).await;
    let mut events = service.events.subscribe();
    service
        .triage_action(mutation(TriageActionDto::MarkRead))
        .await
        .unwrap();
    assert_eq!(
        events.recv().await.unwrap().state,
        SyncState::MutationPending
    );
    triage_mail_desktop_lib::worker::process_outbox_once(&service)
        .await
        .unwrap();
    assert_eq!(store.pending_mutations().unwrap().len(), 1);
    assert_eq!(
        events.recv().await.unwrap().state,
        SyncState::MutationFailed
    );
}

#[tokio::test]
async fn attachments_require_known_metadata_and_reject_imap_injection() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    let request = AttachmentRequestDto {
        account_id: "acct-1".into(),
        folder: "INBOX".into(),
        uid: 1,
        part_id: "2".into(),
    };
    assert_eq!(
        service
            .fetch_attachment(request.clone())
            .await
            .unwrap()
            .bytes,
        vec![1, 2, 3]
    );
    let mut bad = request.clone();
    bad.part_id = "2]\r\nA LOGOUT".into();
    assert_eq!(
        service.fetch_attachment(bad).await,
        Err(IpcError::InvalidRequest)
    );
    let mut bad = request.clone();
    bad.folder = "INBOX\r\nLOGOUT".into();
    assert_eq!(
        service.fetch_attachment(bad).await,
        Err(IpcError::InvalidRequest)
    );
    let mut bad = request;
    bad.part_id = "3".into();
    assert_eq!(service.fetch_attachment(bad).await, Err(IpcError::NotFound));
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unsubscribe_uses_only_stored_validated_headers_after_explicit_action() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    service.get_message_body("m-1".into()).await.unwrap();
    assert_eq!(remote.unsubscribes.load(Ordering::SeqCst), 0);
    service
        .triage_action(mutation(TriageActionDto::OneClickUnsubscribe))
        .await
        .unwrap();
    assert_eq!(remote.unsubscribes.load(Ordering::SeqCst), 1);
}

#[test]
fn payload_shapes_and_credential_debug_are_stable() {
    assert_eq!(
        serde_json::to_value(query(SearchMode::Hybrid)).unwrap(),
        serde_json::json!({"query":"Invoice","mode":"Hybrid","limit":20,"offset":0})
    );
    assert_eq!(
        serde_json::to_value(mutation(TriageActionDto::Tag("x".into()))).unwrap(),
        serde_json::json!({"message_ids":["m-1"],"action":{"Tag":"x"}})
    );
    assert!(!format!("{:?}", credentials()).contains("synthetic"));
    assert_eq!(
        serde_json::to_value(IpcError::RemoteUnavailable).unwrap(),
        "RemoteUnavailable"
    );
}

#[test]
fn additive_migration_and_outbox_survive_reopening_an_encrypted_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mail.db");
    let key = DatabaseKey::from_bytes([29; 32]);
    {
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        s.insert_account_connection(
            &AccountRecord {
                id: "acct-1".into(),
                email: "reader@example.com".into(),
                auth_type: "AppPassword".into(),
                created_at_epoch_ms: 0,
                updated_at_epoch_ms: 0,
            },
            &serde_json::to_string(&config("acct-1")).unwrap(),
        )
        .unwrap();
        s.insert_message(&record()).unwrap();
        s.apply_local_mutations(&[(
            "m-1".into(),
            storage::desktop::LocalMutation::Read(true),
            Some("safe-mutation".into()),
        )])
        .unwrap();
    }
    let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    assert_eq!(s.pending_mutations().unwrap().len(), 1);
    assert_eq!(s.list_account_connections().unwrap().len(), 1);
    let raw = std::fs::read(path).unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("safe-mutation"));
}

#[test]
fn invalid_credentials_are_redacted_even_in_deserialization_errors() {
    let error = serde_json::from_str::<IncomingCredentials>("\"synthetic-private-value\"")
        .unwrap_err()
        .to_string();
    assert!(!error.contains("synthetic-private-value"));
    assert!(error.contains("invalid credential material"));
}

fn inbound(uid: u32) -> mail_engine::ParsedInboundMessage {
    use mail_engine::MimeExtractor;
    let mut message=mail_engine::StalwartMimeParser.parse_lightweight(b"From: security@example.org\r\nSubject: Verification code\r\n\r\nYour verification code: 123456").unwrap();
    message.uid = uid;
    message.modseq = Some(9);
    message
}
fn sync_result(messages: Vec<mail_engine::ParsedInboundMessage>) -> IncrementalSyncResult {
    IncrementalSyncResult {
        folder: "INBOX".into(),
        uidvalidity: 7,
        uidvalidity_reset: false,
        server_uidnext: 3,
        new_last_uid: 2,
        highest_modseq: Some(9),
        new_messages: messages,
        flag_deltas: vec![mail_engine::protocol::UidFlags {
            uid: 2,
            flags: vec!["\\Seen".into()],
        }],
    }
}

#[tokio::test]
async fn ingestion_persists_flags_checkpoint_and_embeddings_before_streaming_alerts() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    *remote.sync_result.lock().unwrap() = Some(sync_result(vec![inbound(2)]));
    let mut events = service.events.subscribe();
    assert_eq!(
        triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1"))
            .await
            .unwrap(),
        1
    );
    let event = events.recv().await.unwrap();
    assert_eq!(event.new_messages_count, 1);
    assert!(!event.triage_alerts.is_empty());
    let id = store
        .find_message_id("acct-1", "INBOX", 2)
        .unwrap()
        .unwrap();
    assert!(store.get_message_by_id(&id).unwrap().unwrap().is_read);
    assert_eq!(
        store
            .get_folder_sync_state("acct-1", "INBOX")
            .unwrap()
            .unwrap()
            .uidnext,
        3
    );
    service
        .set_experimental_semantic_search(true)
        .await
        .unwrap();
    assert!(!service
        .search_messages(query(SearchMode::Semantic))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failed_ingestion_emits_no_success_and_uidvalidity_reset_invalidates_stale_data() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    let mut invalid = inbound(2);
    invalid.attachments.push(mail_engine::AttachmentMeta {
        part_id: "2".into(),
        filename: "large.bin".into(),
        mime_type: "application/octet-stream".into(),
        size_bytes: u64::MAX,
        content_id: None,
        is_inline: false,
    });
    *remote.sync_result.lock().unwrap() = Some(sync_result(vec![invalid]));
    let mut events = service.events.subscribe();
    assert_eq!(
        triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1")).await,
        Err(IpcError::InvalidRequest)
    );
    assert!(events.try_recv().is_err());
    assert_eq!(
        store
            .get_folder_sync_state("acct-1", "INBOX")
            .unwrap()
            .unwrap()
            .uidnext,
        2
    );
    service
        .triage_action(mutation(TriageActionDto::MarkRead))
        .await
        .unwrap();
    let mut reset = sync_result(vec![]);
    reset.uidvalidity_reset = true;
    reset.uidvalidity = 10;
    reset.new_last_uid = 0;
    reset.flag_deltas.clear();
    *remote.sync_result.lock().unwrap() = Some(reset);
    triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();
    assert!(store.get_message_by_id("m-1").unwrap().is_none());
    assert!(store.pending_mutations().unwrap().is_empty());
    let checkpoint = store
        .get_folder_sync_state("acct-1", "INBOX")
        .unwrap()
        .unwrap();
    assert_eq!(checkpoint.uidvalidity, 10);
    assert_eq!(checkpoint.uidnext, 1);
}

#[cfg(feature = "native-runtime")]
#[tokio::test]
async fn typed_tauri_channel_carries_the_contracted_event_shape() {
    use triage_ai::{DefaultDeterministicTriage, DeterministicTriage, TriageEventHook};
    use triage_mail_desktop_lib::events::{BatchEvents, EventBus};
    let bus = Arc::new(EventBus::default());
    let mut receiver = bus.subscribe();
    let outputs = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let captured = outputs.clone();
    let channel = tauri::ipc::Channel::<SyncProgressEvent>::new(move |body| {
        captured.lock().unwrap().push(body.deserialize().unwrap());
        Ok(())
    });
    let batch = BatchEvents::new("acct-1".into(), bus);
    batch
        .on_sync_progress(&mail_engine::SyncProgressEvent {
            account_id: "acct-1".into(),
            folder: "INBOX".into(),
            synced_count: 1,
            uidvalidity: 7,
            highest_modseq: Some(9),
        })
        .unwrap();
    let triage = DefaultDeterministicTriage.analyze(
        "Verification code",
        "security@example.org",
        "Code: 123456",
        false,
    );
    batch.on_message_triaged("m-1", &triage).unwrap();
    assert!(receiver.try_recv().is_err());
    batch.committed();
    channel.send(receiver.recv().await.unwrap()).unwrap();
    let values = outputs.lock().unwrap();
    let event = &values[0];
    assert_eq!(event["account_id"], "acct-1");
    assert_eq!(event["new_messages_count"], 1);
    assert!(event.get("state").is_some());
    assert!(event.get("triage_alerts").is_some());
    assert_eq!(event["embedding_errors_count"], 0);
    assert!(event.get("synced_count").is_none());
    assert_eq!(event.as_object().unwrap().len(), 5);
}

#[tokio::test]
async fn move_acknowledgement_rebases_later_actions_and_preserves_tags_and_attachments() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    let mut stale = record();
    stale.id = "stale-destination".into();
    stale.folder = "Archive".into();
    stale.uid = 77;
    stale.attachments.clear();
    store.insert_message(&stale).unwrap();
    store
        .upsert_folder_sync_state(&FolderSyncState {
            account_id: "acct-1".into(),
            folder_name: "Archive".into(),
            uidvalidity: 12,
            uidnext: 78,
            highest_modseq: Some(20),
        })
        .unwrap();
    service
        .triage_action(mutation(TriageActionDto::Archive))
        .await
        .unwrap();
    service
        .triage_action(mutation(TriageActionDto::MarkUnread))
        .await
        .unwrap();
    service
        .triage_action(mutation(TriageActionDto::Tag("Keep".into())))
        .await
        .unwrap();
    triage_mail_desktop_lib::worker::process_outbox_once(&service)
        .await
        .unwrap();
    {
        let requests = remote.mutations.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].folder, "INBOX");
        assert_eq!(requests[1].folder, "Archive");
        assert_eq!(requests[1].uid, 77);
        assert_eq!(requests[1].uidvalidity, 17);
    }

    assert!(store.pending_mutations().unwrap().is_empty());
    let moved = store.get_message_by_id("m-1").unwrap().unwrap();
    assert_eq!(moved.folder, "Archive");
    assert_eq!(moved.uid, 77);
    assert_eq!(moved.tags, vec!["Keep"]);
    let summaries = service
        .search_messages(query(SearchMode::Lexical))
        .await
        .unwrap();
    assert_eq!(summaries[0].folder, "Archive");
    assert_eq!(summaries[0].uid, 77);
    assert_eq!(summaries[0].folder_override, None);
    assert_eq!(summaries[0].tags, vec!["Keep"]);
    let mut refreshed = moved.clone();
    refreshed.id = "server-generated-destination-id".into();
    refreshed.attachments[0].message_id = refreshed.id.clone();
    refreshed.attachments[0].id = "refreshed-attachment".into();
    let state = FolderSyncState {
        account_id: "acct-1".into(),
        folder_name: "Archive".into(),
        uidvalidity: 17,
        uidnext: 78,
        highest_modseq: Some(99),
    };
    let stored = store
        .commit_sync_batch(
            &[refreshed],
            &[storage::MessageFlagsDelta {
                uid: 77,
                is_read: false,
                is_starred: false,
            }],
            &state,
            false,
        )
        .unwrap();
    assert_eq!(stored[0].id, "m-1");
    assert_eq!(
        store.get_message_by_id("m-1").unwrap().unwrap().attachments[0].message_id,
        "m-1"
    );
    service
        .triage_action(mutation(TriageActionDto::Trash))
        .await
        .unwrap();
    triage_mail_desktop_lib::worker::process_outbox_once(&service)
        .await
        .unwrap();
    assert_eq!(
        store.get_message_by_id("m-1").unwrap().unwrap().folder,
        "Trash"
    );
}

#[tokio::test]
async fn credential_controls_are_rejected_before_any_vault_write() {
    let (service, _, vault) = setup(Arc::new(FakeRemote::default()));
    let result = service
        .add_account(
            config("acct-1"),
            IncomingCredentials(AccountAuthMaterial::AppPassword {
                secret: "synthetic\r\nLOGOUT".into(),
            }),
        )
        .await;
    assert_eq!(result, Err(IpcError::InvalidRequest));
    assert!(matches!(
        vault.get_secret(&VaultKey::for_account("acct-1", "imap-auth").unwrap()),
        Err(vault::VaultError::NotFound(_))
    ));
}

#[tokio::test]
async fn blank_lexical_inbox_reads_persisted_mail_without_an_embedding_provider() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, vault) = setup(remote.clone());
    seed(&service, &store).await;
    let service = DesktopService::new(store, vault, remote, None);
    let rows = service
        .search_messages(SearchQueryDto {
            query: " \t".into(),
            mode: SearchMode::Lexical,
            limit: 200,
            offset: 0,
            scope: None,
            recipient: None,
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "m-1");
}

fn mock_parsed_message(uid: u32) -> mail_engine::ParsedInboundMessage {
    mail_engine::ParsedInboundMessage {
        uid,
        modseq: Some(10),
        provider_message_id: None,
        message_id_header: Some(format!("<msg-{}@example.org>", uid)),
        in_reply_to: None,
        references: vec![],
        references_header: None,
        subject: format!("Subject {}", uid),
        sender: "sender@example.org".into(),
        recipients: vec!["recipient@example.org".into()],
        date_epoch_ms: Some(1000),
        text_plain: Some(format!("Text body {}", uid)),
        html_raw: None,
        raw_headers: Some("From: sender@example.org\r\n".into()),
        unsubscribe: None,
        attachments: vec![],
    }
}

#[tokio::test]
async fn test_worker_sync_account_once_batches_and_commits_sequentially() {
    struct BatchedRemote {
        calls: AtomicUsize,
    }
    #[async_trait]
    impl RemoteMail for BatchedRemote {
        async fn fetch_attachment(
            &self,
            _: &AccountConfigDto,
            _: &str,
            _: u32,
            _: &str,
            _: u32,
        ) -> Result<Vec<u8>, IpcError> {
            Ok(vec![])
        }
        async fn mutate(
            &self,
            _: &AccountConfigDto,
            _: &RemoteMutation,
        ) -> Result<MutationOutcome, IpcError> {
            Ok(MutationOutcome::FlagsUpdated)
        }
        async fn unsubscribe(&self, _: &UnsubscribeDescriptor) -> Result<(), IpcError> {
            Ok(())
        }
        async fn sync(
            &self,
            _: &AccountConfigDto,
            checkpoint: &FolderSyncState,
            sink: &dyn SyncEventSink,
        ) -> Result<IncrementalSyncResult, IpcError> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                let messages: Vec<_> = (1..=300).map(mock_parsed_message).collect();
                sink.on_sync_progress(&mail_engine::SyncProgressEvent {
                    account_id: "acct-1".into(),
                    folder: "INBOX".into(),
                    synced_count: 300,
                    uidvalidity: 100,
                    highest_modseq: Some(10),
                })
                .map_err(|_| IpcError::Internal)?;
                Ok(IncrementalSyncResult {
                    folder: "INBOX".into(),
                    uidvalidity: 100,
                    uidvalidity_reset: false,
                    server_uidnext: 3,
                    new_last_uid: 300,
                    highest_modseq: Some(10),
                    new_messages: messages,
                    flag_deltas: vec![],
                })
            } else if call == 1 {
                assert_eq!(checkpoint.uidnext, 301);
                let messages: Vec<_> = (301..=350).map(mock_parsed_message).collect();
                sink.on_sync_progress(&mail_engine::SyncProgressEvent {
                    account_id: "acct-1".into(),
                    folder: "INBOX".into(),
                    synced_count: 50,
                    uidvalidity: 100,
                    highest_modseq: Some(20),
                })
                .map_err(|_| IpcError::Internal)?;
                Ok(IncrementalSyncResult {
                    folder: "INBOX".into(),
                    uidvalidity: 100,
                    uidvalidity_reset: false,
                    server_uidnext: 3,
                    new_last_uid: 350,
                    highest_modseq: Some(20),
                    new_messages: messages,
                    flag_deltas: vec![],
                })
            } else {
                panic!("unexpected extra sync call");
            }
        }
        async fn wait_for_change(
            &self,
            _: &AccountConfigDto,
            _: &FolderSyncState,
        ) -> Result<bool, IpcError> {
            Ok(false)
        }
        async fn discover_folders(
            &self,
            _: &AccountConfigDto,
        ) -> Result<Vec<mail_engine::DiscoveredFolder>, IpcError> {
            Ok(vec![mail_engine::DiscoveredFolder {
                name: "INBOX".into(),
                display_name: "INBOX".into(),
                delimiter: Some("/".into()),
                attributes: vec!["\\HasNoChildren".into()],
                role: Some(mail_engine::SpecialFolderRole::Inbox),
                is_selectable: true,
            }])
        }
    }

    let remote = Arc::new(BatchedRemote {
        calls: AtomicUsize::new(0),
    });
    let (service, store, _) = setup(remote.clone());
    service
        .add_account(config("acct-1"), credentials())
        .await
        .unwrap();

    let total = triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();

    assert_eq!(total, 350);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 2);

    let state = store
        .get_folder_sync_state("acct-1", "INBOX")
        .unwrap()
        .unwrap();
    assert_eq!(state.uidnext, 351);
    assert_eq!(state.highest_modseq, Some(20));
}

#[tokio::test]
async fn test_worker_sync_account_once_retains_committed_batches_on_interruption() {
    struct FailingBatchRemote {
        calls: AtomicUsize,
    }
    #[async_trait]
    impl RemoteMail for FailingBatchRemote {
        async fn fetch_attachment(
            &self,
            _: &AccountConfigDto,
            _: &str,
            _: u32,
            _: &str,
            _: u32,
        ) -> Result<Vec<u8>, IpcError> {
            Ok(vec![])
        }
        async fn mutate(
            &self,
            _: &AccountConfigDto,
            _: &RemoteMutation,
        ) -> Result<MutationOutcome, IpcError> {
            Ok(MutationOutcome::FlagsUpdated)
        }
        async fn unsubscribe(&self, _: &UnsubscribeDescriptor) -> Result<(), IpcError> {
            Ok(())
        }
        async fn sync(
            &self,
            _: &AccountConfigDto,
            _: &FolderSyncState,
            sink: &dyn SyncEventSink,
        ) -> Result<IncrementalSyncResult, IpcError> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                let messages: Vec<_> = (1..=300).map(mock_parsed_message).collect();
                sink.on_sync_progress(&mail_engine::SyncProgressEvent {
                    account_id: "acct-1".into(),
                    folder: "INBOX".into(),
                    synced_count: 300,
                    uidvalidity: 100,
                    highest_modseq: Some(10),
                })
                .map_err(|_| IpcError::Internal)?;
                Ok(IncrementalSyncResult {
                    folder: "INBOX".into(),
                    uidvalidity: 100,
                    uidvalidity_reset: false,
                    server_uidnext: 3,
                    new_last_uid: 300,
                    highest_modseq: Some(10),
                    new_messages: messages,
                    flag_deltas: vec![],
                })
            } else {
                Err(IpcError::RemoteUnavailable)
            }
        }
        async fn wait_for_change(
            &self,
            _: &AccountConfigDto,
            _: &FolderSyncState,
        ) -> Result<bool, IpcError> {
            Ok(false)
        }
        async fn discover_folders(
            &self,
            _: &AccountConfigDto,
        ) -> Result<Vec<mail_engine::DiscoveredFolder>, IpcError> {
            Ok(vec![mail_engine::DiscoveredFolder {
                name: "INBOX".into(),
                display_name: "INBOX".into(),
                delimiter: Some("/".into()),
                attributes: vec!["\\HasNoChildren".into()],
                role: Some(mail_engine::SpecialFolderRole::Inbox),
                is_selectable: true,
            }])
        }
    }

    let remote = Arc::new(FailingBatchRemote {
        calls: AtomicUsize::new(0),
    });
    let (service, store, _) = setup(remote.clone());
    service
        .add_account(config("acct-1"), credentials())
        .await
        .unwrap();

    let err = triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1")).await;
    assert_eq!(err, Err(IpcError::RemoteUnavailable));

    // Batch 1 (300 messages) is already committed in SQLCipher and durable!
    let state = store
        .get_folder_sync_state("acct-1", "INBOX")
        .unwrap()
        .unwrap();
    assert_eq!(state.uidnext, 301);
}

struct FailingEmbedding;
impl EmbeddingProvider for FailingEmbedding {
    fn embed_int8(&self, _: &str) -> Result<Vec<i8>, TriageError> {
        Err(TriageError::Embedding("simulated failure".into()))
    }
}

#[tokio::test]
async fn sync_worker_tracks_embedding_failure_telemetry() {
    let remote = Arc::new(FakeRemote::default());
    let store = store();
    let vault = vault();
    let service = DesktopService::new(
        store.clone(),
        vault.clone(),
        remote.clone(),
        Some(Arc::new(FailingEmbedding)),
    );
    seed(&service, &store).await;
    *remote.sync_result.lock().unwrap() = Some(sync_result(vec![inbound(2)]));

    let mut rx = service.events.subscribe();
    triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();

    let mut error_count = 0;
    while let Ok(event) = rx.try_recv() {
        if event.embedding_errors_count > 0 {
            error_count += event.embedding_errors_count;
        }
    }
    assert_eq!(error_count, 1);
}

#[tokio::test]
async fn test_desktop_service_discovers_and_lists_folders() {
    struct FolderDiscoveryRemote;
    #[async_trait]
    impl RemoteMail for FolderDiscoveryRemote {
        async fn fetch_attachment(
            &self,
            _: &AccountConfigDto,
            _: &str,
            _: u32,
            _: &str,
            _: u32,
        ) -> Result<Vec<u8>, IpcError> {
            Ok(vec![])
        }
        async fn mutate(
            &self,
            _: &AccountConfigDto,
            _: &RemoteMutation,
        ) -> Result<MutationOutcome, IpcError> {
            Ok(MutationOutcome::FlagsUpdated)
        }
        async fn unsubscribe(&self, _: &UnsubscribeDescriptor) -> Result<(), IpcError> {
            Ok(())
        }
        async fn sync(
            &self,
            _: &AccountConfigDto,
            checkpoint: &FolderSyncState,
            _: &dyn SyncEventSink,
        ) -> Result<IncrementalSyncResult, IpcError> {
            Ok(IncrementalSyncResult {
                folder: checkpoint.folder_name.clone(),
                uidvalidity: 100,
                uidvalidity_reset: false,
                server_uidnext: 3,
                new_last_uid: 0,
                highest_modseq: Some(1),
                new_messages: vec![],
                flag_deltas: vec![],
            })
        }
        async fn wait_for_change(
            &self,
            _: &AccountConfigDto,
            _: &FolderSyncState,
        ) -> Result<bool, IpcError> {
            Ok(false)
        }
        async fn discover_folders(
            &self,
            _: &AccountConfigDto,
        ) -> Result<Vec<mail_engine::DiscoveredFolder>, IpcError> {
            Ok(vec![
                mail_engine::DiscoveredFolder {
                    name: "INBOX".into(),
                    display_name: "INBOX".into(),
                    delimiter: Some("/".into()),
                    attributes: vec!["\\HasNoChildren".into()],
                    role: Some(mail_engine::SpecialFolderRole::Inbox),
                    is_selectable: true,
                },
                mail_engine::DiscoveredFolder {
                    name: "[Gmail]/Sent Mail".into(),
                    display_name: "Sent Mail".into(),
                    delimiter: Some("/".into()),
                    attributes: vec!["\\HasNoChildren".into(), "\\Sent".into()],
                    role: Some(mail_engine::SpecialFolderRole::Sent),
                    is_selectable: true,
                },
                mail_engine::DiscoveredFolder {
                    name: "[Gmail]".into(),
                    display_name: "[Gmail]".into(),
                    delimiter: Some("/".into()),
                    attributes: vec!["\\Noselect".into(), "\\HasChildren".into()],
                    role: None,
                    is_selectable: false,
                },
            ])
        }
    }

    let remote = Arc::new(FolderDiscoveryRemote);
    let (service, _store, _) = setup(remote);
    service
        .add_account(config("acct-1"), credentials())
        .await
        .unwrap();

    let count = triage_mail_desktop_lib::worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();
    assert_eq!(count, 0);

    let folders = service.list_folders("acct-1".into()).await.unwrap();
    assert_eq!(folders.len(), 3);
    assert_eq!(folders[0].folder_name, "INBOX");
    assert_eq!(folders[0].role, "inbox");
    assert_eq!(folders[1].folder_name, "[Gmail]/Sent Mail");
    assert_eq!(folders[1].role, "sent");
    assert_eq!(folders[2].folder_name, "[Gmail]");
    assert!(!folders[2].is_selectable);
}

#[tokio::test]
async fn test_starred_and_unstarred_triage_actions_and_outbox() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;

    assert!(!store.get_message_by_id("m-1").unwrap().unwrap().is_starred);

    // 1. Optimistic Star mutation
    service
        .triage_action(mutation(TriageActionDto::Star))
        .await
        .unwrap();
    assert!(store.get_message_by_id("m-1").unwrap().unwrap().is_starred);
    assert_eq!(store.pending_mutations().unwrap().len(), 1);

    // 2. Incoming background flag sync with is_starred=false must NOT override pending star
    let checkpoint = store
        .get_folder_sync_state("acct-1", "INBOX")
        .unwrap()
        .unwrap();
    store
        .commit_sync_batch(
            &[],
            &[storage::MessageFlagsDelta {
                uid: 1,
                is_read: true,
                is_starred: false,
            }],
            &checkpoint,
            false,
        )
        .unwrap();
    assert!(store.get_message_by_id("m-1").unwrap().unwrap().is_starred);

    // 3. Process outbox sends Star to remote
    triage_mail_desktop_lib::worker::process_outbox_once(&service)
        .await
        .unwrap();
    assert!(store.pending_mutations().unwrap().is_empty());
    assert_eq!(
        remote.mutations.lock().unwrap()[0].action,
        TriageActionDto::Star
    );

    // 4. After outbox completes, remote flag updates reconcile cleanly
    store
        .commit_sync_batch(
            &[],
            &[storage::MessageFlagsDelta {
                uid: 1,
                is_read: true,
                is_starred: false,
            }],
            &checkpoint,
            false,
        )
        .unwrap();
    assert!(!store.get_message_by_id("m-1").unwrap().unwrap().is_starred);

    // 5. Unstar mutation
    service
        .triage_action(mutation(TriageActionDto::Unstar))
        .await
        .unwrap();
    assert!(!store.get_message_by_id("m-1").unwrap().unwrap().is_starred);
    assert_eq!(store.pending_mutations().unwrap().len(), 1);
    triage_mail_desktop_lib::worker::process_outbox_once(&service)
        .await
        .unwrap();
    assert!(store.pending_mutations().unwrap().is_empty());
    assert_eq!(
        remote.mutations.lock().unwrap()[1].action,
        TriageActionDto::Unstar
    );
}

#[tokio::test]
async fn test_snooze_unsnooze_and_expiration_reconciliation() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote);
    seed(&service, &store).await;

    let now = storage::now_epoch_ms();

    // 1. Reject past deadline
    let past_res = service
        .triage_action(mutation(TriageActionDto::Snooze {
            until_epoch_ms: now - 1000,
        }))
        .await;
    assert_eq!(past_res.unwrap_err(), IpcError::InvalidRequest);

    // 2. Reject excessively far deadline (> 1 year)
    let far_res = service
        .triage_action(mutation(TriageActionDto::Snooze {
            until_epoch_ms: now + 400 * 24 * 3600 * 1000,
        }))
        .await;
    assert_eq!(far_res.unwrap_err(), IpcError::InvalidRequest);

    // 3. Valid future deadline
    let deadline = now + 3_600_000;
    service
        .triage_action(mutation(TriageActionDto::Snooze {
            until_epoch_ms: deadline,
        }))
        .await
        .unwrap();
    let current = store.get_message_by_id("m-1").unwrap().unwrap();
    assert_eq!(current.snooze_until_epoch_ms, Some(deadline));
    // INVARIANT: Snooze is local metadata and never queues remote mutations
    assert!(store.pending_mutations().unwrap().is_empty());

    // 4. Detail DTO and row summary expose snooze and star fields
    let detail = service.get_message_body("m-1".into()).await.unwrap();
    assert_eq!(detail.snooze_until_epoch_ms, Some(deadline));
    assert!(!detail.is_starred);

    let search_res = service
        .search_messages(dto::SearchQueryDto {
            query: String::new(),
            mode: storage::SearchMode::Lexical,
            limit: 10,
            offset: 0,
            scope: None,
            recipient: None,
        })
        .await
        .unwrap();
    assert_eq!(search_res[0].snooze_until_epoch_ms, Some(deadline));
    assert!(!search_res[0].is_starred);

    // 5. Reconcile before deadline does not clear
    let expired = store.reconcile_expired_snoozes(now + 1000).unwrap();
    assert!(expired.is_empty());
    assert_eq!(
        store
            .get_message_by_id("m-1")
            .unwrap()
            .unwrap()
            .snooze_until_epoch_ms,
        Some(deadline)
    );

    // 6. Reconcile at/after deadline clears snooze
    let expired = store.reconcile_expired_snoozes(deadline + 1).unwrap();
    assert_eq!(expired, vec!["m-1"]);
    assert_eq!(
        store
            .get_message_by_id("m-1")
            .unwrap()
            .unwrap()
            .snooze_until_epoch_ms,
        None
    );

    // 7. Snooze then Unsnooze
    service
        .triage_action(mutation(TriageActionDto::Snooze {
            until_epoch_ms: deadline,
        }))
        .await
        .unwrap();
    assert_eq!(
        store
            .get_message_by_id("m-1")
            .unwrap()
            .unwrap()
            .snooze_until_epoch_ms,
        Some(deadline)
    );
    service
        .triage_action(mutation(TriageActionDto::Unsnooze))
        .await
        .unwrap();
    assert_eq!(
        store
            .get_message_by_id("m-1")
            .unwrap()
            .unwrap()
            .snooze_until_epoch_ms,
        None
    );

    // 8. Snooze then Archive immediately clears snooze
    service
        .triage_action(mutation(TriageActionDto::Snooze {
            until_epoch_ms: deadline,
        }))
        .await
        .unwrap();
    assert_eq!(
        store
            .get_message_by_id("m-1")
            .unwrap()
            .unwrap()
            .snooze_until_epoch_ms,
        Some(deadline)
    );
    service
        .triage_action(mutation(TriageActionDto::Archive))
        .await
        .unwrap();
    assert_eq!(
        store
            .get_message_by_id("m-1")
            .unwrap()
            .unwrap()
            .snooze_until_epoch_ms,
        None
    );
}

#[tokio::test]
async fn rules_observe_new_arrivals_without_replaying_history_or_duplicate_sync() {
    use triage_ai::rules::{Condition, Rule, RuleAction};
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    service
        .add_account(config("acct-1"), credentials())
        .await
        .unwrap();
    let r = Rule {
        version: 1,
        id: "rule".into(),
        revision: 0,
        name: "Synthetic arrivals".into(),
        enabled: true,
        position: 0,
        all: vec![Condition::Account("acct-1".into())],
        any: vec![],
        except: vec![],
        actions: vec![RuleAction::Star, RuleAction::Tag("new".into())],
        stop_processing: false,
    };
    service.save_rule(r).await.unwrap();
    let mut initial = sync_result(vec![mock_parsed_message(1)]);
    initial.server_uidnext = 2;
    initial.new_last_uid = 1;
    *remote.sync_result.lock().unwrap() = Some(initial);
    worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();
    let old = store
        .find_message_id("acct-1", "INBOX", 1)
        .unwrap()
        .unwrap();
    assert!(!store.get_message_by_id(&old).unwrap().unwrap().is_starred);
    let mut arrival = sync_result(vec![mock_parsed_message(2)]);
    arrival.server_uidnext = 3;
    *remote.sync_result.lock().unwrap() = Some(arrival);
    worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();
    let id = store
        .find_message_id("acct-1", "INBOX", 2)
        .unwrap()
        .unwrap();
    let m = store.get_message_by_id(&id).unwrap().unwrap();
    assert!(m.is_starred);
    assert_eq!(m.tags, ["new"]);
    let outbox = store.pending_mutations().unwrap();
    assert_eq!(outbox.len(), 1);
    let mutation: RemoteMutation = serde_json::from_str(&outbox[0].1).unwrap();
    assert_eq!(mutation.account_id, "acct-1");
    assert_eq!(mutation.folder, "INBOX");
    assert_eq!(mutation.uid, 2);
    assert_eq!(mutation.uidvalidity, 7);
    assert_eq!(mutation.action, TriageActionDto::Star);
    let draft = service
        .rule_draft(id.clone(), "draft".into())
        .await
        .unwrap();
    assert!(!draft.enabled);
    assert_eq!(draft.revision, 0);
    assert!(draft
        .all
        .contains(&Condition::Sender("sender@example.org".into())));
    let simulated = service
        .simulate_rules(vec![id.clone()], None)
        .await
        .unwrap();
    assert_eq!(simulated[0].1.actions.len(), 2);
    assert_eq!(store.pending_mutations().unwrap().len(), 1);
    worker::sync_account_once(&service, &config("acct-1"))
        .await
        .unwrap();
    service.process_rules().await.unwrap();
    assert_eq!(store.pending_mutations().unwrap().len(), 1);
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
    assert_eq!(remote.unsubscribes.load(Ordering::SeqCst), 0);
    assert!(remote.mutations.lock().unwrap().is_empty());
}

#[tokio::test]
async fn saved_view_search_retains_semantic_consent_and_never_delivers_or_fetches() {
    use storage::saved_views::{SavedView, ViewFilters};
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    let view = service
        .save_saved_view(SavedView {
            version: 1,
            id: "saved".into(),
            revision: 0,
            name: "Invoices".into(),
            query: "Invoice".into(),
            mode: SearchMode::Semantic,
            accounts: vec!["acct-1".into()],
            scope: None,
            filters: ViewFilters::default(),
        })
        .await
        .unwrap();
    assert_eq!(
        service
            .search_saved_view(view.id.clone(), view.revision, 200, 0)
            .await
            .unwrap_err(),
        IpcError::InvalidRequest
    );
    service
        .set_experimental_semantic_search(true)
        .await
        .unwrap();
    assert!(service
        .search_saved_view(view.id.clone(), view.revision, 200, 0)
        .await
        .is_ok());
    let mut updated = view.clone();
    updated.mode = SearchMode::Lexical;
    let updated = service.save_saved_view(updated).await.unwrap();
    assert_eq!(
        service
            .search_saved_view(view.id, view.revision, 200, 0)
            .await
            .unwrap_err(),
        IpcError::InvalidRequest
    );
    let rows = service
        .search_saved_view(updated.id, updated.revision, 200, 0)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "m-1");
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
    assert_eq!(remote.unsubscribes.load(Ordering::SeqCst), 0);
    assert!(remote.mutations.lock().unwrap().is_empty());
}

#[tokio::test]
async fn conversation_snapshots_reject_stale_apply_and_report_remote_failure_without_fetches() {
    use storage::conversations::{ConversationAction, ConversationQuery, ConversationSource};
    let remote = Arc::new(FakeRemote {
        fail: true,
        ..Default::default()
    });
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;
    service.backfill_conversations().await.unwrap();
    let mut context = ConversationQuery {
        source: ConversationSource::Messages,
        scope: None,
        generation: None,
    };
    let page = service
        .search_conversations(query(SearchMode::Lexical), context.clone())
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 1);
    context.generation = Some(page.generation);
    let id = page.rows[0].conversation_id.clone();
    let preview = service
        .preview_conversation(
            query(SearchMode::Lexical),
            context.clone(),
            id.clone(),
            false,
            ConversationAction::MarkRead,
        )
        .await
        .unwrap();
    let mut changed = record();
    changed.subject = "Changed invoice".into();
    store.insert_message(&changed).unwrap();
    assert!(service.apply_conversation(preview.token).await.is_err());
    assert!(!store.get_message_by_id("m-1").unwrap().unwrap().is_read);
    context.generation = None;
    let page = service
        .search_conversations(query(SearchMode::Lexical), context.clone())
        .await
        .unwrap();
    context.generation = Some(page.generation);
    let preview = service
        .preview_conversation(
            query(SearchMode::Lexical),
            context,
            id,
            true,
            ConversationAction::MarkRead,
        )
        .await
        .unwrap();
    let token = preview.token.clone();
    assert_eq!(
        service.apply_conversation(token.clone()).await.unwrap(),
        vec!["m-1"]
    );
    assert!(service.apply_conversation(token).await.is_err());
    assert!(store.get_message_by_id("m-1").unwrap().unwrap().is_read);
    worker::process_outbox_once(&service).await.unwrap();
    let status = service
        .conversation_status(vec!["m-1".into()])
        .await
        .unwrap();
    assert!(status[0].pending && status[0].failed);
    assert_eq!(status[0].attempts, 1);
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
    assert_eq!(remote.unsubscribes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn conversation_local_commit_survives_partial_remote_completion() {
    use storage::conversations::{ConversationAction, ConversationQuery, ConversationSource};
    let remote = Arc::new(FakeRemote {
        fail_message: Some("m-2".into()),
        ..Default::default()
    });
    let (service, store, _) = setup(remote);
    seed(&service, &store).await;
    let mut reply = record();
    reply.id = "m-2".into();
    reply.uid = 2;
    reply.received_at_epoch_ms = 2;
    reply.message_id_header = Some("reply@example.org".into());
    reply.in_reply_to = Some("message@example.org".into());
    reply.attachments.clear();
    store.insert_message(&reply).unwrap();
    service.backfill_conversations().await.unwrap();
    let mut context = ConversationQuery {
        source: ConversationSource::Messages,
        scope: None,
        generation: None,
    };
    let page = service
        .search_conversations(query(SearchMode::Lexical), context.clone())
        .await
        .unwrap();
    assert_eq!(page.rows[0].total_count, 2);
    context.generation = Some(page.generation);
    let preview = service
        .preview_conversation(
            query(SearchMode::Lexical),
            context,
            page.rows[0].conversation_id.clone(),
            true,
            ConversationAction::MarkRead,
        )
        .await
        .unwrap();
    service.apply_conversation(preview.token).await.unwrap();
    for id in ["m-1", "m-2"] {
        assert!(store.get_message_by_id(id).unwrap().unwrap().is_read);
    }
    worker::process_outbox_once(&service).await.unwrap();
    let statuses = service
        .conversation_status(vec!["m-1".into(), "m-2".into()])
        .await
        .unwrap();
    assert!(!statuses[0].pending);
    assert!(statuses[1].pending && statuses[1].failed);
}

#[tokio::test]
async fn recovered_account_reconnection_keeps_identity_cached_mail_and_credentials_isolated() {
    let (source, original, _) = setup(Arc::new(FakeRemote::default()));
    seed(&source, &original).await;
    let passphrase = b"synthetic backup passphrase";
    let bytes = original.write_backup(Vec::new(), passphrase).unwrap();
    let (recovered, target, secrets) = setup(Arc::new(FakeRemote::default()));
    target.read_backup(bytes.as_slice(), passphrase).unwrap();
    let accounts = recovered.list_accounts().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_id, "acct-1");
    assert!(accounts[0].imap_host.is_empty());
    assert!(secrets
        .get_secret(&VaultKey::for_account("acct-1", "imap-auth").unwrap())
        .is_err());
    assert_eq!(
        recovered
            .search_messages(query(SearchMode::Lexical))
            .await
            .unwrap()[0]
            .id,
        "m-1"
    );
    let mut new_config = config("");
    new_config.email_display = accounts[0].email_display.clone();
    assert_eq!(
        recovered
            .add_account(new_config, credentials())
            .await
            .unwrap(),
        "acct-1"
    );
    assert_eq!(recovered.list_accounts().await.unwrap().len(), 1);
    assert!(!recovered.list_accounts().await.unwrap()[0]
        .imap_host
        .is_empty());
    assert!(target.disconnected_accounts().unwrap().is_empty());
    assert!(secrets
        .get_secret(&VaultKey::for_account("acct-1", "imap-auth").unwrap())
        .is_ok());
    assert_eq!(
        recovered.add_account(config("acct-1"), credentials()).await,
        Err(IpcError::AccountExists)
    );
}

#[tokio::test]
async fn portable_import_rejects_changed_preview_and_missing_consent() {
    use std::collections::BTreeMap;
    use storage::portable::ReferenceMapping;
    use triage_ai::rules::{Condition, Rule, RuleAction};
    let (service, store, _) = setup(Arc::new(FakeRemote::default()));
    seed(&service, &store).await;
    store
        .save_rule(Rule {
            version: 1,
            id: "rule".into(),
            revision: 0,
            name: "Synthetic original".into(),
            enabled: false,
            position: 0,
            all: vec![Condition::Account("acct-1".into())],
            any: vec![],
            except: vec![],
            actions: vec![RuleAction::Star],
            stop_processing: false,
        })
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("definitions.json");
    service.export_file(path.clone()).await.unwrap();
    let expected = backup::preview(&path).unwrap();
    let mut changed = expected.clone();
    changed.rules[0].name = "Synthetic replacement".into();
    std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(
        service
            .import_file(
                path.clone(),
                ReferenceMapping {
                    accounts: BTreeMap::from([("account-1".into(), "acct-1".into())]),
                    destinations: BTreeMap::new()
                },
                expected,
                true
            )
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service
            .import_file(path, ReferenceMapping::default(), changed, false)
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(store.list_rules().unwrap().len(), 1);
}

/// Observes the real SQLCipher adapter at the last host hook, before progress flush.
struct SyncHookProbe {
    store: Arc<SqliteMailStore>,
    events: Mutex<Option<tokio::sync::broadcast::Receiver<SyncProgressEvent>>>,
    calls: AtomicUsize,
}
impl EmbeddingProvider for SyncHookProbe {
    fn embed_int8(&self, _: &str) -> Result<Vec<i8>, TriageError> {
        use storage::sender_review::ReviewState;
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let uid = if call == 0 { 1 } else { 2 };
        let id = self
            .store
            .find_message_id("acct-1", "INBOX", uid)
            .unwrap()
            .unwrap();
        let checkpoint = self
            .store
            .get_folder_sync_state("acct-1", "INBOX")
            .unwrap()
            .unwrap();
        assert_eq!(checkpoint.uidnext, uid + 1);
        let review = self.store.review_metadata(&id).unwrap().unwrap();
        assert_eq!(
            review.state,
            if uid == 1 {
                ReviewState::Known
            } else {
                ReviewState::Pending
            }
        );
        if call == 1 {
            // Rules have already drained after commit, before injected indexing.
            let message = self.store.get_message_by_id(&id).unwrap().unwrap();
            assert!(message.is_starred);
            assert_eq!(message.tags, ["arrival"]);
            assert_eq!(self.store.pending_mutations().unwrap().len(), 1);
        }
        let mut events = self.events.lock().unwrap();
        let events = events.as_mut().unwrap();
        if call == 1 {
            // Rule changes retain their independent post-commit invalidation event.
            let changed = events.try_recv().unwrap();
            assert_eq!(changed.state, SyncState::ResyncRequired);
            assert_eq!(changed.new_messages_count, 0);
        }
        assert!(events.try_recv().is_err());
        Ok(vec![3; 384])
    }
}

#[tokio::test]
async fn shared_sync_adapter_preserves_encrypted_frontiers_hooks_and_duplicate_identity() {
    use storage::sender_review::ReviewState;
    use triage_ai::rules::{Condition, Rule, RuleAction};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mail.db");
    let key = DatabaseKey::from_bytes([23; 32]);
    let store = Arc::new(SqliteMailStore::open(&path, &key, Default::default()).unwrap());
    let remote = Arc::new(FakeRemote::default());
    let probe = Arc::new(SyncHookProbe {
        store: store.clone(),
        events: Mutex::new(None),
        calls: AtomicUsize::new(0),
    });
    let service = DesktopService::new(store.clone(), vault(), remote.clone(), Some(probe.clone()));
    service
        .add_account(config("acct-1"), credentials())
        .await
        .unwrap();
    let mut preferences = store.review_preferences("acct-1").unwrap();
    preferences.enabled = true;
    store.save_review_preferences(preferences).unwrap();
    store.backfill_sender_review().unwrap();
    assert!(!store.review_preferences("acct-1").unwrap().initializing);
    service
        .save_rule(Rule {
            version: 1,
            id: "sync-rule".into(),
            revision: 0,
            name: "Synthetic arrival".into(),
            enabled: true,
            position: 0,
            all: vec![Condition::Account("acct-1".into())],
            any: vec![],
            except: vec![],
            actions: vec![RuleAction::Star, RuleAction::Tag("arrival".into())],
            stop_processing: false,
        })
        .await
        .unwrap();
    *probe.events.lock().unwrap() = Some(service.events.subscribe());

    for uid in [1, 2, 2] {
        let mut message = mock_parsed_message(uid);
        message.sender = if uid == 1 {
            "history@example.org"
        } else {
            "arrival@example.org"
        }
        .into();
        let mut result = sync_result(vec![message]);
        result.new_last_uid = uid;
        result.server_uidnext = uid + 1;
        *remote.sync_result.lock().unwrap() = Some(result);
        assert_eq!(
            worker::sync_account_once(&service, &config("acct-1"))
                .await
                .unwrap(),
            1
        );
        let event = probe
            .events
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .try_recv()
            .unwrap();
        assert_eq!(event.state, SyncState::Idle);
        assert_eq!(event.new_messages_count, 1);
    }
    let id = store
        .find_message_id("acct-1", "INBOX", 2)
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .search(&storage::HybridQuery {
                query_text: String::new(),
                query_embedding_int8: None,
                account_filter: None,
                mode: SearchMode::Lexical,
                limit: 100,
                offset: 0,
            })
            .unwrap()
            .len(),
        2
    );
    assert_eq!(store.pending_mutations().unwrap().len(), 1);
    assert_eq!(probe.calls.load(Ordering::SeqCst), 3);
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
    assert!(remote.mutations.lock().unwrap().is_empty());
    assert_eq!(remote.unsubscribes.load(Ordering::SeqCst), 0);
    drop(service);
    drop(probe);
    drop(store);
    let reopened = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    assert_eq!(
        reopened.find_message_id("acct-1", "INBOX", 2).unwrap(),
        Some(id.clone())
    );
    assert_eq!(
        reopened.review_metadata(&id).unwrap().unwrap().state,
        ReviewState::Pending
    );
    assert_eq!(reopened.pending_mutations().unwrap().len(), 1);
    assert_ne!(&std::fs::read(&path).unwrap()[..16], b"SQLite format 3\0");
}

#[tokio::test]
async fn local_archive_import_streams_mbox_and_eml_with_local_only_triage_and_worker_skip() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    let mut events_rx = service.events.subscribe();

    let dir = tempfile::tempdir().unwrap();
    let mbox_path = dir.path().join("takeout.mbox");
    let eml_path = dir.path().join("single.eml");
    let bad_path = dir.path().join("not_an_mbox.mbox");

    let mbox_data = concat!(
        "From sender1@example.org Mon Jan 01 10:00:00 2024\r\n",
        "From: \"Billing\" <billing@example.org>\r\n",
        "To: reader@example.com\r\n",
        "Subject: Monthly Receipt\r\n",
        "Date: Mon, 01 Jan 2024 10:00:00 +0000\r\n",
        "Message-ID: <mbox-1@example.org>\r\n",
        "X-Gmail-Labels: Inbox,Unread\r\n",
        "MIME-Version: 1.0\r\n",
        "Content-Type: multipart/mixed; boundary=\"B1\"\r\n",
        "\r\n",
        "--B1\r\n",
        "Content-Type: text/plain; charset=utf-8\r\n",
        "\r\n",
        "Receipt body.\r\n",
        "--B1\r\n",
        "Content-Type: application/pdf; name=\"receipt.pdf\"\r\n",
        "Content-Disposition: attachment; filename=\"receipt.pdf\"\r\n",
        "\r\n",
        "%PDF-1.4\r\n",
        "--B1--\r\n",
        "From sender2@example.org Tue Jan 02 10:00:00 2024\r\n",
        "From: news@example.org\r\n",
        "To: reader@example.com\r\n",
        "Subject: Starred Digest\r\n",
        "Date: Tue, 02 Jan 2024 10:00:00 +0000\r\n",
        "Message-ID: <mbox-2@example.org>\r\n",
        "X-Gmail-Labels: Inbox,Starred\r\n",
        "\r\n",
        "Digest body.\r\n",
    );
    std::fs::write(&mbox_path, mbox_data).unwrap();
    std::fs::write(
        &eml_path,
        concat!(
            "From: solo@example.org\r\n",
            "To: reader@example.com\r\n",
            "Subject: Standalone Note\r\n",
            "Date: Wed, 03 Jan 2024 10:00:00 +0000\r\n",
            "Message-ID: <eml-3@example.org>\r\n",
            "\r\n",
            "Standalone body.\r\n",
        ),
    )
    .unwrap();
    std::fs::write(&bad_path, b"Subject: Missing separator\r\n\r\nBody\r\n").unwrap();

    // Malformed archive fails before creating an account.
    assert_eq!(
        service
            .import_local_archive(
                bad_path,
                "archive@example.com".into(),
                mail_engine::LocalArchiveFormat::Mbox,
                None,
            )
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert!(service.list_accounts().await.unwrap().is_empty());

    // Stream .mbox into a new LocalArchive account.
    let progress = Arc::new(Mutex::new(Vec::new()));
    let progress_cb = progress.clone();
    let mbox_summary = service
        .import_local_archive(
            mbox_path,
            "archive@example.com".into(),
            mail_engine::LocalArchiveFormat::Mbox,
            Some(Arc::new(move |ev| {
                progress_cb.lock().unwrap().push(ev);
            })),
        )
        .await
        .unwrap();
    assert_eq!(mbox_summary.imported_count, 2);
    assert_eq!(mbox_summary.batches_committed, 1);
    assert_eq!(mbox_summary.last_uid, 2);

    // WorkerSupervisor skips LocalArchive accounts without opening any network connection.
    let accounts = service.list_accounts().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].auth_method, AuthMethod::LocalArchive);
    assert_eq!(accounts[0].imap_host, "");
    assert_eq!(accounts[0].imap_port, 0);
    let supervisor = worker::WorkerSupervisor::new(service.clone());
    supervisor.start_account(accounts[0].clone()).unwrap();
    supervisor.shutdown();

    // Second import to the same email_display reuses account_id and assigns monotonic UID 3.
    let eml_summary = service
        .import_local_archive(
            eml_path,
            "archive@example.com".into(),
            mail_engine::LocalArchiveFormat::Eml,
            None,
        )
        .await
        .unwrap();
    assert_eq!(eml_summary.account_id, mbox_summary.account_id);
    assert_eq!(eml_summary.imported_count, 1);
    assert_eq!(eml_summary.last_uid, 3);
    assert_eq!(service.list_accounts().await.unwrap().len(), 1);

    // Search returns all 3 imported messages across both imports.
    let rows = service
        .search_messages(SearchQueryDto {
            query: String::new(),
            mode: SearchMode::Lexical,
            limit: 50,
            offset: 0,
            scope: Some(MailboxScopeDto::AccountView {
                account_id: mbox_summary.account_id.clone(),
                view: MailboxViewDto::Inbox,
            }),
            recipient: None,
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);

    // Offline attachment fetch fails deterministically with Unsupported without touching remote.
    let m1_id = store
        .find_message_id(&mbox_summary.account_id, "INBOX", 1)
        .unwrap()
        .unwrap();
    let detail = service.get_message_body(m1_id.clone()).await.unwrap();
    assert_eq!(detail.attachments_meta.len(), 1);
    assert_eq!(
        service
            .fetch_attachment(AttachmentRequestDto {
                account_id: mbox_summary.account_id.clone(),
                folder: "INBOX".into(),
                uid: 1,
                part_id: detail.attachments_meta[0].part_id.clone(),
            })
            .await,
        Err(IpcError::Unsupported)
    );
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);

    // Local triage succeeds locally without queuing IMAP pending_mutations.
    service
        .triage_action(TriageMutationDto {
            message_ids: vec![m1_id.clone()],
            action: TriageActionDto::Archive,
        })
        .await
        .unwrap();
    assert!(store.pending_mutations().unwrap().is_empty());
    let summary = store
        .search(&storage::HybridQuery {
            query_text: String::new(),
            query_embedding_int8: None,
            account_filter: Some(mbox_summary.account_id.clone()),
            mode: storage::SearchMode::Lexical,
            limit: 50,
            offset: 0,
        })
        .unwrap();
    let m1_summary = summary.iter().find(|row| row.id == m1_id).unwrap();
    assert_eq!(m1_summary.folder_override.as_deref(), Some("Archive"));

    // Event bus & callback observed progress.
    assert!(progress
        .lock()
        .unwrap()
        .iter()
        .any(|e| e.state == SyncState::Syncing && e.new_messages_count == 2));
    assert_eq!(
        progress.lock().unwrap().last().unwrap().state,
        SyncState::Idle
    );
    assert!(events_rx.try_recv().is_ok());

    // Cannot import an archive using an email address owned by a live IMAP account.
    service
        .add_account(config("acct-imap"), credentials())
        .await
        .unwrap();
    let valid_eml = dir.path().join("conflict.eml");
    std::fs::copy(dir.path().join("single.eml"), &valid_eml).unwrap();
    assert_eq!(
        service
            .import_local_archive(
                valid_eml,
                "reader@example.com".into(),
                mail_engine::LocalArchiveFormat::Eml,
                None,
            )
            .await,
        Err(IpcError::AccountExists)
    );
}

#[tokio::test]
async fn test_account_connection_returns_diagnostics_without_persisting_anything() {
    let remote = Arc::new(FakeRemote::default());
    let (service, _store, vault) = setup(remote.clone());
    let mut cfg = config("");

    let ok = service
        .test_account_connection(cfg.clone(), credentials())
        .await
        .unwrap();
    assert_eq!(
        ok,
        ConnectionTestResultDto {
            ok: true,
            diagnostic: None
        }
    );

    *remote.test_connection_error.lock().unwrap() = Some(IpcError::ImapDisabledHint);
    let disabled = service
        .test_account_connection(cfg.clone(), credentials())
        .await
        .unwrap();
    assert_eq!(
        disabled,
        ConnectionTestResultDto {
            ok: false,
            diagnostic: Some(ConnectionDiagnosticCode::ImapDisabledHint),
        }
    );

    *remote.test_connection_error.lock().unwrap() = Some(IpcError::AuthenticationRejected);
    let rejected = service
        .test_account_connection(cfg.clone(), credentials())
        .await
        .unwrap();
    assert_eq!(
        rejected,
        ConnectionTestResultDto {
            ok: false,
            diagnostic: Some(ConnectionDiagnosticCode::AuthenticationRejected),
        }
    );

    *remote.test_connection_error.lock().unwrap() = Some(IpcError::RemoteUnavailable);
    assert_eq!(
        service
            .test_account_connection(cfg.clone(), credentials())
            .await,
        Err(IpcError::RemoteUnavailable)
    );

    cfg.auth_method = AuthMethod::LocalArchive;
    cfg.imap_host.clear();
    cfg.imap_port = 0;
    assert_eq!(
        service
            .test_account_connection(cfg, IncomingCredentials(AccountAuthMaterial::LocalArchive),)
            .await,
        Err(IpcError::InvalidRequest)
    );

    assert!(service.list_accounts().await.unwrap().is_empty());
    assert!(vault
        .get_secret(&VaultKey::for_account("transient-test", "imap-auth").unwrap())
        .is_err());
}

#[tokio::test]
async fn worker_failure_emits_diagnostic_and_update_auth_clears_it_preserving_mail() {
    let remote = Arc::new(FakeRemote::default());
    let (service, store, vault) = setup(remote.clone());
    let cfg = AccountConfigDto {
        account_id: "acct-oauth".into(),
        email_display: "reader@example.com".into(),
        imap_host: "imap.example.com".into(),
        imap_port: 993,
        auth_method: AuthMethod::OAuth2PublicPkce,
    };
    service
        .add_account(
            cfg.clone(),
            IncomingCredentials(AccountAuthMaterial::OAuth2PublicPkce {
                client_id: "client-1".into(),
                access_token: "access-1".into(),
                refresh_token: Some("refresh-1".into()),
                token_endpoint: Some("https://auth.example.com/token".into()),
            }),
        )
        .await
        .unwrap();

    store
        .insert_message(&MessageRecord {
            id: "msg-keep".into(),
            account_id: "acct-oauth".into(),
            folder: "INBOX".into(),
            uid: 1,
            message_id_header: Some("<keep@example.org>".into()),
            in_reply_to: None,
            references_header: None,
            subject: "Keep on reauth".into(),
            sender: "sender@example.org".into(),
            recipients: "reader@example.com".into(),
            received_at_epoch_ms: 1_730_000_000_000,
            is_read: false,
            has_attachments: false,
            category_badge: Some("Personal".into()),
            triage_badges: Vec::new(),
            raw_headers: "Subject: Keep\r\n\r\n".into(),
            text_body: Some("Body".into()),
            html_body: None,
            list_unsubscribe: None,
            list_unsubscribe_post: None,
            one_click_unsubscribe_supported: false,
            created_at_epoch_ms: 1_730_000_000_000,
            attachments: Vec::new(),
            tags: Vec::new(),
            is_starred: false,
            snooze_until_epoch_ms: None,
            provider_message_id: None,
        })
        .unwrap();

    // Unconfigured sync_result triggers FakeRemote::sync -> Err(Unsupported) -> unclassified,
    // while update_account_auth rotates vault material in place and preserves messages.
    service
        .update_account_auth(
            cfg,
            IncomingCredentials(AccountAuthMaterial::OAuth2PublicPkce {
                client_id: "client-1".into(),
                access_token: "access-2".into(),
                refresh_token: Some("refresh-2".into()),
                token_endpoint: Some("https://auth.example.com/token".into()),
            }),
        )
        .await
        .unwrap();
    let stored = AccountAuthMaterial::from_bytes(
        &vault
            .get_secret(&VaultKey::for_account("acct-oauth", "imap-auth").unwrap())
            .unwrap(),
    )
    .unwrap();
    match stored {
        AccountAuthMaterial::OAuth2PublicPkce { access_token, .. } => {
            assert_eq!(access_token, "access-2");
        }
        other => panic!("unexpected auth variant: {other:?}"),
    }
    assert!(store.get_message_by_id("msg-keep").unwrap().is_some());
    assert!(service.account_diagnostics().await.unwrap().is_empty());
}

#[tokio::test]
async fn quota_refresh_infers_badge_and_manual_override_persists_additively() {
    use mail_engine::{AccountBadgeOverride, AccountPlanBadge, AccountStorageQuota};
    let remote = Arc::new(FakeRemote::default());
    let (service, _store, _vault) = setup(remote.clone());
    let mut cfg = config("acct-google");
    cfg.imap_host = "imap.gmail.com".into();
    service
        .add_account(cfg.clone(), credentials())
        .await
        .unwrap();
    let id = "acct-google".to_string();

    // No quota yet: no badge, Auto override.
    let subs = service.account_subscriptions().await.unwrap();
    assert_eq!(subs[&id].effective_badge, None);
    assert_eq!(subs[&id].badge_override, AccountBadgeOverride::Auto);

    *remote.quota.lock().unwrap() = Some(AccountStorageQuota {
        used_bytes: 1024,
        limit_bytes: 2048 * 1024 * 1024 * 1024,
    });
    service.refresh_account_quota(&cfg).await;
    let subs = service.account_subscriptions().await.unwrap();
    assert_eq!(subs[&id].inferred_badge, Some(AccountPlanBadge::Pro));
    assert_eq!(subs[&id].effective_badge.as_deref(), Some("PRO"));
    assert_eq!(subs[&id].quota.unwrap().used_bytes, 1024);

    let hidden = service
        .set_account_badge(id.clone(), AccountBadgeOverride::Hidden)
        .await
        .unwrap();
    assert_eq!(hidden.effective_badge, None);
    let custom = service
        .set_account_badge(id.clone(), AccountBadgeOverride::Custom(" Work ".into()))
        .await
        .unwrap();
    assert_eq!(custom.effective_badge.as_deref(), Some("Work"));
    assert_eq!(
        service
            .set_account_badge(id.clone(), AccountBadgeOverride::Custom("  ".into()))
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service
            .set_account_badge("missing".into(), AccountBadgeOverride::Auto)
            .await,
        Err(IpcError::NotFound)
    );
    // Extras are additive: the account config still deserializes unchanged.
    assert_eq!(service.list_accounts().await.unwrap(), vec![cfg]);
}

#[tokio::test]
async fn attachment_save_streams_to_disk_rejects_symlinks_and_enforces_caps() {
    use triage_mail_desktop_lib::service::{
        MAX_PREVIEW_ATTACHMENT_BYTES, MAX_SAVE_ATTACHMENT_BYTES,
    };

    let remote = Arc::new(FakeRemote::default());
    let (service, store, _) = setup(remote.clone());
    seed(&service, &store).await;

    let mut oversized_preview = record();
    oversized_preview.id = "m-2".into();
    oversized_preview.uid = 2;
    oversized_preview.message_id_header = Some("<preview-cap@example.com>".into());
    oversized_preview.attachments = vec![
        AttachmentRecord {
            id: "a-2".into(),
            message_id: "m-2".into(),
            part_id: "2".into(),
            filename: "../invoice.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: (MAX_PREVIEW_ATTACHMENT_BYTES + 1) as i64,
            content_id: None,
            is_inline: false,
        },
        AttachmentRecord {
            id: "a-3".into(),
            message_id: "m-2".into(),
            part_id: "3".into(),
            filename: "huge.bin".into(),
            content_type: "application/octet-stream".into(),
            size_bytes: (MAX_SAVE_ATTACHMENT_BYTES + 1) as i64,
            content_id: None,
            is_inline: false,
        },
    ];
    let mut msg3 = record();
    msg3.id = "m-3".into();
    msg3.uid = 3;
    msg3.message_id_header = Some("<save-ok@example.com>".into());
    msg3.attachments = vec![AttachmentRecord {
        id: "a-4".into(),
        message_id: "m-3".into(),
        part_id: "2".into(),
        filename: "report.pdf".into(),
        content_type: "application/pdf".into(),
        size_bytes: 3,
        content_id: None,
        is_inline: false,
    }];
    store.insert_message(&oversized_preview).unwrap();
    store.insert_message(&msg3).unwrap();

    // Preview (cmd_fetch_attachment) rejects payloads > 10 MiB before network call.
    assert_eq!(
        service
            .fetch_attachment(AttachmentRequestDto {
                account_id: "acct-1".into(),
                folder: "INBOX".into(),
                uid: 2,
                part_id: "2".into(),
            })
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);

    // Save-As allows > 10 MiB up to 50 MiB, and rejects > 50 MiB before dialog/network.
    assert_eq!(
        service
            .prepare_attachment_save(&AttachmentRequestDto {
                account_id: "acct-1".into(),
                folder: "INBOX".into(),
                uid: 2,
                part_id: "2".into(),
            })
            .await
            .unwrap(),
        "../invoice.pdf"
    );
    assert_eq!(
        service
            .prepare_attachment_save(&AttachmentRequestDto {
                account_id: "acct-1".into(),
                folder: "INBOX".into(),
                uid: 2,
                part_id: "3".into(),
            })
            .await,
        Err(IpcError::InvalidRequest)
    );

    let dir = tempfile::tempdir().unwrap();
    let canonical_dir = dir.path().canonicalize().unwrap();
    let save_req = AttachmentRequestDto {
        account_id: "acct-1".into(),
        folder: "INBOX".into(),
        uid: 3,
        part_id: "2".into(),
    };

    // Reject traversal and directories before network fetch.
    assert_eq!(
        service
            .save_attachment_to_path(save_req.clone(), canonical_dir.join("../escape.pdf"))
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        service
            .save_attachment_to_path(save_req.clone(), canonical_dir.clone())
            .await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);

    #[cfg(unix)]
    {
        let real = canonical_dir.join("existing.pdf");
        std::fs::write(&real, b"keep-me").unwrap();
        let symlink = canonical_dir.join("link.pdf");
        std::os::unix::fs::symlink(&real, &symlink).unwrap();
        assert_eq!(
            service
                .save_attachment_to_path(save_req.clone(), symlink)
                .await,
            Err(IpcError::InvalidRequest)
        );
        assert_eq!(std::fs::read(&real).unwrap(), b"keep-me");
        let linked_dir = canonical_dir.join("linked-dir");
        std::os::unix::fs::symlink(&canonical_dir, &linked_dir).unwrap();
        assert_eq!(
            service
                .save_attachment_to_path(save_req.clone(), linked_dir.join("out.pdf"))
                .await,
            Err(IpcError::InvalidRequest)
        );
        assert_eq!(remote.attachments.load(Ordering::SeqCst), 0);
    }

    // Valid target saves bytes directly to disk and overwrites regular files safely.
    let target = canonical_dir.join("report.pdf");
    let saved = service
        .save_attachment_to_path(save_req.clone(), target.clone())
        .await
        .unwrap();
    assert_eq!(saved.bytes_written, 3);
    assert_eq!(std::fs::read(&target).unwrap(), vec![1, 2, 3]);
    assert_eq!(remote.attachments.load(Ordering::SeqCst), 1);

    // No staging temp files remain in the target directory.
    let entries: Vec<_> = std::fs::read_dir(&canonical_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!entries.iter().any(|name| name.starts_with(".triage-att-")));
}

#[tokio::test]
async fn account_aliases_persist_normalized_and_reject_invalid_input() {
    use triage_mail_desktop_lib::aliases::AccountAliasDto;
    let (service, _store, _vault) = setup(Arc::new(FakeRemote::default()));
    let cfg = config("acct-hub");
    service
        .add_account(cfg.clone(), credentials())
        .await
        .unwrap();
    let alias = |a: &str, l: Option<&str>| AccountAliasDto {
        address: a.into(),
        label: l.map(Into::into),
    };
    assert!(service.account_aliases().await.unwrap().is_empty());
    let saved = service
        .set_account_aliases(
            "acct-hub".into(),
            vec![
                alias(" Billing@Example.com ", Some(" Billing ")),
                alias("dev@example.com", Some(" ")),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        saved,
        vec![
            alias("billing@example.com", Some("Billing")),
            alias("dev@example.com", None)
        ]
    );
    assert_eq!(service.account_aliases().await.unwrap()["acct-hub"], saved);
    for bad in [
        vec![alias("not-an-email", None)],
        vec![alias("Name <a@example.com>", None)],
        vec![alias("a@example.com", None), alias("A@example.com", None)],
        (0..65)
            .map(|i| alias(&format!("u{i}@example.com"), None))
            .collect(),
    ] {
        assert_eq!(
            service.set_account_aliases("acct-hub".into(), bad).await,
            Err(IpcError::InvalidRequest)
        );
    }
    assert_eq!(
        service.set_account_aliases("missing".into(), vec![]).await,
        Err(IpcError::NotFound)
    );
    assert_eq!(service.account_aliases().await.unwrap()["acct-hub"], saved);
    assert_eq!(service.list_accounts().await.unwrap(), vec![cfg]);
}

#[tokio::test]
async fn recipient_search_requires_account_scope() {
    let (service, _store, _) = setup(Arc::new(FakeRemote::default()));
    let mut request = query(SearchMode::Lexical);
    request.recipient = Some("billing@example.com".into());
    assert!(matches!(
        service.search_messages(request).await,
        Err(IpcError::InvalidRequest)
    ));
}
