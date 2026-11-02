use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use mail_engine::sync::IncrementalSyncResult;
use mail_engine::{AuthMethod, SyncEventSink, UnsubscribeDescriptor};
use storage::{
    AccountRecord, AttachmentRecord, DatabaseKey, FolderSyncState, MailRepository, MessageRecord,
    SearchMode, SqliteMailStore,
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
    sync_result: Mutex<Option<IncrementalSyncResult>>,
}

#[async_trait]
impl RemoteMail for FakeRemote {
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
        if self.fail {
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
    }
}
fn mutation(action: TriageActionDto) -> TriageMutationDto {
    TriageMutationDto {
        message_ids: vec!["m-1".into()],
        action,
    }
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
        .commit_sync_batch(&[], &[(1, false)], &checkpoint, false)
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
        .commit_sync_batch(&[refreshed], &[(77, false)], &state, false)
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
