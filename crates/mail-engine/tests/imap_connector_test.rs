// INVARIANT: Integration tests must use synthetic RFC 2606 addresses (`user@example.com`,
// `user@example.org`) and never leak real email addresses or tokens.
// INVARIANT: Never fetch full RFC822 bodies in bulk sync; binary attachments are fetched
// on-demand via IMAP BODY.PEEK[section].

use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
use vault::SecretStore;

use mail_engine::protocol::ImapSession;
use mail_engine::provider::{ProviderKind, ServerCapabilities};
use mail_engine::sync::{execute_incremental_sync, IncrementalSyncQuery};
use mail_engine::tls::create_tls_connector;
use mail_engine::{AuthMethod, EngineError, ImapEndpointConfig, SyncEventSink, SyncProgressEvent};

struct MockProgressSink {
    notified_count: AtomicUsize,
    last_event: std::sync::Mutex<Option<SyncProgressEvent>>,
}

impl MockProgressSink {
    fn new() -> Self {
        Self {
            notified_count: AtomicUsize::new(0),
            last_event: std::sync::Mutex::new(None),
        }
    }
}

impl SyncEventSink for MockProgressSink {
    fn on_sync_progress(&self, event: &SyncProgressEvent) -> Result<(), EngineError> {
        self.notified_count
            .fetch_add(event.synced_count, Ordering::SeqCst);
        let mut last = self.last_event.lock().unwrap();
        *last = Some(event.clone());
        Ok(())
    }
}

#[tokio::test]
async fn test_provider_quirk_normalization_gmail_and_condstore() {
    let (client_stream, mut server_stream) = duplex(8192);
    let mut session = ImapSession::new(client_stream);

    tokio::spawn(async move {
        // Read CAPABILITY
        let mut buf = [0u8; 1024];
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("CAPABILITY"));

        // Gmail capabilities with CONDSTORE, X-GM-EXT-1, and AUTH=XOAUTH2
        let caps_resp = "* CAPABILITY IMAP4rev1 CONDSTORE SPECIAL-USE IDLE MOVE X-GM-EXT-1 AUTH=XOAUTH2\r\nA0001 OK CAPABILITY completed\r\n";
        server_stream.write_all(caps_resp.as_bytes()).await.unwrap();

        // Read XOAUTH2 AUTHENTICATE
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("AUTHENTICATE XOAUTH2"));

        let auth_resp = "A0002 OK [CAPABILITY IMAP4rev1] Authentication successful\r\n";
        server_stream.write_all(auth_resp.as_bytes()).await.unwrap();
    });

    let caps = session.capability().await.unwrap();
    assert!(caps.supports_condstore());
    assert!(caps.x_gm_ext1);
    assert!(caps.xoauth2);

    let provider = ProviderKind::detect("imap.gmail.com", Some(&caps));
    assert_eq!(provider, ProviderKind::Gmail);

    let auth_res = session
        .authenticate_xoauth2("user@example.com", "ya29.synthetic-token-123")
        .await;
    assert!(auth_res.is_ok());
}

#[tokio::test]
async fn test_provider_quirk_normalization_outlook_condstore_absence_and_fallback() {
    let (client_stream, mut server_stream) = duplex(8192);
    let mut session = ImapSession::new(client_stream);
    let sink = MockProgressSink::new();

    tokio::spawn(async move {
        let mut buf = [0u8; 2048];

        // 1. SELECT INBOX
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("SELECT \"INBOX\""));

        // Outlook response: NO CONDSTORE, standard UIDVALIDITY
        let select_resp = "* 2 EXISTS\r\n* OK [UIDVALIDITY 777] UIDs valid\r\n* OK [UIDNEXT 300]\r\nA0001 OK [READ-WRITE] SELECT completed\r\n";
        server_stream
            .write_all(select_resp.as_bytes())
            .await
            .unwrap();

        // 2. Fallback query: UID FETCH 1:* (UID FLAGS)
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("UID FETCH 1:* (UID FLAGS)"));

        let fetch_flags_resp = "* 1 FETCH (UID 100 FLAGS (\\Seen))\r\n* 2 FETCH (UID 101 FLAGS (\\Answered))\r\nA0002 OK UID FETCH completed\r\n";
        server_stream
            .write_all(fetch_flags_resp.as_bytes())
            .await
            .unwrap();
    });

    // Outlook server lacks CONDSTORE
    let caps = ServerCapabilities::from_tokens(["IMAP4rev1", "IDLE", "SPECIAL-USE"]);
    assert!(!caps.supports_condstore());

    let query = IncrementalSyncQuery {
        account_id: "outlook-acct",
        folder: "INBOX",
        local_uidvalidity: 777,
        local_last_uid: 299, // no new messages
        local_last_modseq: None,
    };
    let sync_res = execute_incremental_sync(&mut session, query, &caps, &sink)
        .await
        .unwrap();

    assert!(!sync_res.uidvalidity_reset);
    assert_eq!(sync_res.flag_deltas.len(), 2);
    assert_eq!(sync_res.flag_deltas[0].uid, 100);
    assert_eq!(sync_res.flag_deltas[0].flags, vec!["\\Seen"]);
    assert_eq!(sync_res.flag_deltas[1].uid, 101);
    assert_eq!(sync_res.flag_deltas[1].flags, vec!["\\Answered"]);
}

#[tokio::test]
async fn test_incremental_sync_uidvalidity_invalidation_handling() {
    let (client_stream, mut server_stream) = duplex(4096);
    let mut session = ImapSession::new(client_stream);
    let sink = MockProgressSink::new();

    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        let _ = server_stream.read(&mut buf).await.unwrap();

        // Server reports unexpected UIDVALIDITY (e.g. mailbox reconstructed after restore)
        let resp = "* 10 EXISTS\r\n* OK [UIDVALIDITY 99999] Reset occurred\r\n* OK [UIDNEXT 11]\r\nA0001 OK SELECT completed\r\n";
        server_stream.write_all(resp.as_bytes()).await.unwrap();
    });

    let caps = ServerCapabilities::default();
    let query = IncrementalSyncQuery {
        account_id: "acct-resync",
        folder: "INBOX",
        local_uidvalidity: 12345, // previous stored UIDVALIDITY
        local_last_uid: 500,
        local_last_modseq: None,
    };
    let res = execute_incremental_sync(&mut session, query, &caps, &sink)
        .await
        .unwrap();

    assert!(res.uidvalidity_reset);
    assert_eq!(res.uidvalidity, 99999);
    assert_eq!(res.new_messages.len(), 0);
}

#[tokio::test]
async fn test_incremental_sync_ingests_lightweight_messages_with_rfc8058_unsubscribe() {
    let (client_stream, mut server_stream) = duplex(8192);
    let mut session = ImapSession::new(client_stream);
    let sink = MockProgressSink::new();

    tokio::spawn(async move {
        let mut buf = [0u8; 2048];

        // 1. SELECT
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("SELECT \"INBOX\""));

        let select_resp = "* 1 EXISTS\r\n* OK [UIDVALIDITY 555] UIDs valid\r\n* OK [UIDNEXT 106]\r\n* OK [HIGHESTMODSEQ 888]\r\nA0001 OK SELECT completed\r\n";
        server_stream
            .write_all(select_resp.as_bytes())
            .await
            .unwrap();

        // 2a. UID SEARCH UID 101:*
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("UID SEARCH"));
        server_stream
            .write_all(b"* SEARCH 105\r\nA0002 OK UID SEARCH completed\r\n")
            .await
            .unwrap();

        // 2b. Phase 1: UID FETCH 105 (UID MODSEQ FLAGS BODYSTRUCTURE BODY.PEEK[HEADER])
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("BODY.PEEK[HEADER]"));
        assert!(cmd.contains("BODYSTRUCTURE"));
        assert!(!cmd.contains("BODY.PEEK[TEXT]"));
        assert!(!cmd.contains("RFC822"));

        let synthetic_headers = concat!(
            "From: newsletter@example.com\r\n",
            "To: subscriber@example.org\r\n",
            "Subject: Monthly Tech Digest\r\n",
            "Message-ID: <digest-105@example.com>\r\n",
            "List-Unsubscribe: <https://example.com/unsub?id=105>, <mailto:unsub@example.com>\r\n",
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n\r\n"
        );
        let synthetic_body = "Here is your monthly technical digest with triage tips.";
        let bs = r#"("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 55 1 NIL NIL NIL NIL)"#;

        let fetch_phase1 = format!(
            "* 1 FETCH (UID 105 MODSEQ (888) FLAGS (\\Seen) BODYSTRUCTURE {} BODY[HEADER] {{{}}}\r\n{})\r\nA0003 OK UID FETCH completed\r\n",
            bs,
            synthetic_headers.len(),
            synthetic_headers
        );
        server_stream
            .write_all(fetch_phase1.as_bytes())
            .await
            .unwrap();

        // 2c. Phase 2: UID FETCH 105 (BODY.PEEK[1])
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("BODY.PEEK[1]"));
        assert!(!cmd.contains("BODY.PEEK[TEXT]"));
        assert!(!cmd.contains("RFC822"));

        let fetch_phase2 = format!(
            "* 1 FETCH (UID 105 BODY[1] {{{}}}\r\n{})\r\nA0004 OK UID FETCH completed\r\n",
            synthetic_body.len(),
            synthetic_body
        );
        server_stream
            .write_all(fetch_phase2.as_bytes())
            .await
            .unwrap();

        // 3. CHANGEDSINCE 800 query
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("CHANGEDSINCE 800"));

        let modseq_resp = "A0005 OK UID FETCH completed\r\n";
        server_stream
            .write_all(modseq_resp.as_bytes())
            .await
            .unwrap();
    });

    let caps = ServerCapabilities::from_tokens(["IMAP4rev1", "CONDSTORE"]);
    let query = IncrementalSyncQuery {
        account_id: "acct-news",
        folder: "INBOX",
        local_uidvalidity: 555,
        local_last_uid: 100, // last_uid
        local_last_modseq: Some(800),
    };
    let sync_res = execute_incremental_sync(&mut session, query, &caps, &sink)
        .await
        .unwrap();

    assert_eq!(sync_res.new_messages.len(), 1);
    let msg = &sync_res.new_messages[0];
    assert_eq!(msg.uid, 105);
    assert_eq!(msg.modseq, Some(888));
    assert_eq!(msg.subject, "Monthly Tech Digest");
    assert_eq!(msg.sender, "newsletter@example.com");
    assert_eq!(
        msg.message_id_header.as_deref(),
        Some("digest-105@example.com")
    );
    assert_eq!(
        msg.text_plain.as_deref(),
        Some("Here is your monthly technical digest with triage tips.")
    );

    // Verify RFC 8058 One-Click Unsubscribe metadata extracted during sync
    let unsub = msg
        .unsubscribe
        .as_ref()
        .expect("unsubscribe metadata present");
    assert!(unsub.supports_rfc8058_one_click);
    assert_eq!(
        unsub.https_uri.as_deref(),
        Some("https://example.com/unsub?id=105")
    );

    assert_eq!(sink.notified_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_lazy_attachment_fetch_on_demand() {
    let (client_stream, mut server_stream) = duplex(4096);
    let mut session = ImapSession::new(client_stream);

    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(cmd.contains("UID FETCH 105 (BODY.PEEK[2])"));

        let synthetic_pdf_bytes = b"%PDF-1.4 synthetic binary attachment bytes";
        let fetch_resp = format!(
            "* 1 FETCH (UID 105 BODY[2] {{{}}}\r\n{}\r\n)\r\nA0001 OK UID FETCH completed\r\n",
            synthetic_pdf_bytes.len(),
            String::from_utf8_lossy(synthetic_pdf_bytes)
        );
        server_stream
            .write_all(fetch_resp.as_bytes())
            .await
            .unwrap();
    });

    let bytes = session.fetch_attachment_bytes(105, "2").await.unwrap();
    assert_eq!(bytes, b"%PDF-1.4 synthetic binary attachment bytes");
}

#[test]
fn test_default_connector_vault_integration_and_auth_material_roundtrip() {
    let cipher = std::sync::Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
        b"synthetic-vault-pass-99",
    ));
    let fallback = std::sync::Arc::new(vault::EncryptedFallbackStore::in_memory(cipher));
    let auto_store = vault::AutoSecretStore::with_service("triage-mail-test", fallback);
    let store_arc = std::sync::Arc::new(auto_store);

    let connector = mail_engine::DefaultImapConnector::new(store_arc.clone());

    // Save synthetic OAuth2 PKCE auth material
    let auth = vault::AccountAuthMaterial::OAuth2PublicPkce {
        client_id: "synthetic-client-id-pkce".to_string(),
        access_token: "ya29.synthetic-access-token-999".to_string(),
        refresh_token: Some("1//synthetic-refresh-token-888".to_string()),
    };
    let key = vault::VaultKey::for_account("acct-pkce", "imap-auth").unwrap();
    store_arc
        .set_secret(&key, &auth.to_bytes().unwrap())
        .unwrap();

    // Connector retrieves from vault
    let loaded = connector.load_auth_material("acct-pkce").unwrap();
    assert_eq!(loaded, auth);

    // Verify debug representation does not leak tokens
    let debug_repr = format!("{:?}", loaded);
    assert!(debug_repr.contains("[REDACTED]"));
    assert!(!debug_repr.contains("ya29.synthetic-access-token-999"));
}

#[test]
fn test_multilingual_folder_classification_and_gmail_all_mail_dedup() {
    use mail_engine::provider::{classify_folder, is_gmail_virtual_all_mail, SpecialFolderRole};

    // Portuguese
    assert_eq!(
        classify_folder("Caixa de Entrada", &[] as &[&str]),
        Some(SpecialFolderRole::Inbox)
    );
    assert_eq!(
        classify_folder("Lixeira", &[] as &[&str]),
        Some(SpecialFolderRole::Trash)
    );
    assert_eq!(
        classify_folder("Itens Enviados", &[] as &[&str]),
        Some(SpecialFolderRole::Sent)
    );
    assert_eq!(
        classify_folder("Arquivo", &[] as &[&str]),
        Some(SpecialFolderRole::Archive)
    );

    // Spanish
    assert_eq!(
        classify_folder("Bandeja de entrada", &[] as &[&str]),
        Some(SpecialFolderRole::Inbox)
    );
    assert_eq!(
        classify_folder("Papelera", &[] as &[&str]),
        Some(SpecialFolderRole::Trash)
    );

    // Gmail virtual folders
    assert!(is_gmail_virtual_all_mail("[Gmail]/All Mail"));
    assert!(is_gmail_virtual_all_mail("[Gmail]/Todos os e-mails"));
    assert_eq!(
        classify_folder("[Gmail]/All Mail", &[] as &[&str]),
        Some(SpecialFolderRole::AllMail)
    );
}

#[test]
fn test_loopback_tls_policy_strictly_guards_proton_mail_bridge() {
    let loopback = ImapEndpointConfig {
        account_id: "proton-user".to_string(),
        host: "127.0.0.1".to_string(),
        port: 1143,
        auth_method: AuthMethod::ProtonBridgeLocal,
        allow_loopback_self_signed_tls: true,
    };
    assert!(create_tls_connector(&loopback).is_ok());

    let remote_attack = ImapEndpointConfig {
        account_id: "attacker-domain".to_string(),
        host: "imap.evil-mitm.example.org".to_string(),
        port: 993,
        auth_method: AuthMethod::AppPassword,
        allow_loopback_self_signed_tls: true,
    };
    let err = create_tls_connector(&remote_attack).err().unwrap();
    assert_eq!(
        err,
        EngineError::InsecureTlsRejected("imap.evil-mitm.example.org".to_string())
    );
}
