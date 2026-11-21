use super::*;
use crate::remote::NativeRemoteMail;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vault::{SecretStore, VaultError};

#[derive(Default)]
struct TestVault(Mutex<HashMap<VaultKey, Vec<u8>>>);
impl SecretStore for TestVault {
    fn set_secret(&self, key: &VaultKey, bytes: &[u8]) -> Result<(), VaultError> {
        self.0.lock().unwrap().insert(key.clone(), bytes.to_vec());
        Ok(())
    }
    fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, VaultError> {
        self.0
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| VaultError::NotFound(key.as_str().into()))
    }
    fn delete_secret(&self, key: &VaultKey) -> Result<(), VaultError> {
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}
fn fixture() -> (
    Arc<DesktopService>,
    Arc<TestVault>,
    AccountConfigDto,
    IncomingAuthorization,
) {
    let vault = Arc::new(TestVault::default());
    let store = storage::SqliteMailStore::open_in_memory(
        &storage::DatabaseKey::from_bytes([51; 32]),
        Default::default(),
    )
    .unwrap();
    let service = DesktopService::new(
        Arc::new(store),
        vault.clone(),
        Arc::new(NativeRemoteMail::new(vault.clone())),
        None,
    );
    let config = AccountConfigDto {
        account_id: String::new(),
        email_display: "reader@example.com".into(),
        imap_host: "imap.example.com".into(),
        imap_port: 993,
        auth_method: AuthMethod::OAuth2PublicPkce,
    };
    let input = IncomingAuthorization(AuthorizationFields {
        authorization_endpoint: "https://auth.example.com/authorize".into(),
        token_endpoint: "https://auth.example.com/token".into(),
        scopes: "imap offline_access".into(),
        client_id: "synthetic-client".into(),
        client_secret: None,
    });
    (service, vault, config, input)
}
struct TestExchange(AtomicUsize);
#[async_trait]
impl TokenExchange for TestExchange {
    async fn exchange(&self, request: TokenRequest<'_>) -> Result<TokenResponse, IpcError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.code, "synthetic-code");
        assert_eq!(request.client_id, "synthetic-client");
        assert!(request.verifier.len() >= 43);
        assert!(request.redirect.starts_with("http://127.0.0.1:"));
        Ok(TokenResponse {
            access_token: "synthetic-access".into(),
            refresh_token: Some("synthetic-refresh".into()),
            token_type: "Bearer".into(),
        })
    }
}
fn callback(url: &str, valid_state: bool) -> Result<(), IpcError> {
    let url = Url::parse(url).unwrap();
    let values: HashMap<_, _> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    assert_eq!(values["code_challenge_method"], "S256");
    assert_eq!(values["response_type"], "code");
    assert!(!values.contains_key("client_secret"));
    assert!(!values.contains_key("access_type"));
    let redirect = Url::parse(&values["redirect_uri"]).unwrap();
    let state = if valid_state {
        values["state"].clone()
    } else {
        String::from("wrong-state")
    };
    tokio::spawn(async move {
        let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", redirect.port().unwrap()))
            .await
            .unwrap();
        socket.write_all(format!("GET /callback?code=synthetic-code&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes()).await.unwrap();
        let mut response = Vec::new();
        socket.read_to_end(&mut response).await.unwrap();
    });
    Ok(())
}

#[tokio::test]
async fn google_oauth_requests_offline_refresh_token_and_consent() {
    let (service, _vault, mut config, mut input) = fixture();
    config.auth_method = AuthMethod::OAuth2ByoCredentials;
    config.imap_host = "imap.gmail.com".into();
    input.0.authorization_endpoint = "https://accounts.google.com/o/oauth2/v2/auth".into();
    input.0.token_endpoint = "https://oauth2.googleapis.com/token".into();
    input.0.scopes = "https://mail.google.com/".into();
    input.0.client_secret = Some("synthetic-google-secret".into());
    let (_sender, cancel) = watch::channel(false);
    let exchange = TestExchange(AtomicUsize::new(0));
    let observed_url = Arc::new(Mutex::new(String::new()));
    let captured_url = observed_url.clone();
    let id = authorize(
        service,
        config,
        input,
        cancel,
        move |url| {
            *captured_url.lock().unwrap() = url.to_owned();
            let parsed = Url::parse(url).unwrap();
            let redirect = parsed
                .query_pairs()
                .find(|(k, _)| k == "redirect_uri")
                .unwrap()
                .1
                .into_owned();
            let state = parsed
                .query_pairs()
                .find(|(k, _)| k == "state")
                .unwrap()
                .1
                .into_owned();
            let port = Url::parse(&redirect).unwrap().port().unwrap();
            tokio::spawn(async move {
                let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .unwrap();
                socket
                    .write_all(
                        format!(
                            "GET /callback?code=synthetic-code&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                let mut response = Vec::new();
                socket.read_to_end(&mut response).await.unwrap();
            });
            Ok(())
        },
        &exchange,
    )
    .await
    .unwrap();
    assert!(!id.is_empty());
    let parsed = Url::parse(&observed_url.lock().unwrap()).unwrap();
    let params: HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(
        params.get("access_type").map(String::as_str),
        Some("offline")
    );
    assert_eq!(params.get("prompt").map(String::as_str), Some("consent"));
}
#[test]
fn validates_endpoints_and_redacts_bad_inputs_and_tokens() {
    for target in [
        "http://auth.example.com",
        "https://user:synthetic@auth.example.com",
        "https://auth.example.com/#fragment",
        "https://auth.example.com/?client_secret=synthetic",
        "file:///tmp/token",
    ] {
        assert!(endpoint(target).is_err());
    }
    assert!(endpoint("https://auth.example.com/oauth/token").is_ok());
    let error = serde_json::from_str::<IncomingAuthorization>(
        r#"{"client_id":"synthetic-private-value","client_secret":42}"#,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("invalid authorization material"));
    assert!(!error.contains("synthetic-private-value"));
    for response in [
        br#"{"error":"synthetic-private-value"}"#.as_slice(),
        br#"{"access_token":"synthetic","token_type":"mac"}"#,
        br#"{"access_token":"","token_type":"Bearer"}"#,
    ] {
        assert!(matches!(
            parse_tokens(response),
            Err(IpcError::RemoteUnavailable)
        ));
    }
}
#[test]
fn cancellation_is_window_scoped_and_duplicate_authorization_is_rejected() {
    let sessions = AuthorizationSessions::default();
    let (lease, receiver) = sessions.begin("main").unwrap();
    let (_other, other) = sessions.begin("other").unwrap();
    assert!(sessions.begin("main").is_err());
    sessions.cancel("main");
    assert!(*receiver.borrow());
    assert!(!*other.borrow());
    drop(lease);
    assert!(sessions.begin("main").is_ok());
}
#[tokio::test]
async fn registers_both_oauth_modes_without_exposing_or_leaving_staged_secrets() {
    for method in [
        AuthMethod::OAuth2PublicPkce,
        AuthMethod::OAuth2ByoCredentials,
    ] {
        let (service, vault, mut config, mut input) = fixture();
        config.auth_method = method;
        if method == AuthMethod::OAuth2ByoCredentials {
            input.0.client_secret = Some("synthetic-secret".into());
        }
        let (_sender, cancel) = watch::channel(false);
        let exchange = TestExchange(AtomicUsize::new(0));
        let id = authorize(
            service.clone(),
            config,
            input,
            cancel,
            |url| callback(url, true),
            &exchange,
        )
        .await
        .unwrap();
        assert!(!id.contains('@'));
        assert_eq!(exchange.0.load(Ordering::SeqCst), 1);
        let accounts = service.list_accounts().await.unwrap();
        assert_eq!(accounts.len(), 1);
        assert!(!serde_json::to_string(&accounts)
            .unwrap()
            .contains("synthetic"));
        let entries = vault.0.lock().unwrap();
        assert_eq!(entries.len(), 1);
        let auth = AccountAuthMaterial::from_bytes(entries.values().next().unwrap()).unwrap();
        assert_eq!(
            auth.token_endpoint(),
            Some("https://auth.example.com/token")
        );
        assert!(matches!(
            auth,
            AccountAuthMaterial::OAuth2PublicPkce { .. }
                | AccountAuthMaterial::OAuth2ByoCredentials { .. }
        ));
    }
}

#[tokio::test]
async fn clean_orphaned_staging_removes_pending_secret_on_startup() {
    let vault = Arc::new(TestVault::default());
    let staging_key = staging_oauth_vault_key().unwrap();
    vault
        .set_secret(&staging_key, b"leftover-staging-data")
        .unwrap();
    assert!(vault.get_secret(&staging_key).is_ok());

    let store = storage::SqliteMailStore::open_in_memory(
        &storage::DatabaseKey::from_bytes([52; 32]),
        Default::default(),
    )
    .unwrap();
    let _service = DesktopService::new(
        Arc::new(store),
        vault.clone(),
        Arc::new(NativeRemoteMail::new(vault.clone())),
        None,
    );

    // Staging secret was purged upon DesktopService initialization
    assert!(vault.get_secret(&staging_key).is_err());
}
#[tokio::test]
async fn rejects_wrong_state_and_cleans_vault_without_exchange() {
    let (service, vault, config, input) = fixture();
    let (_sender, cancel) = watch::channel(false);
    let exchange = TestExchange(AtomicUsize::new(0));
    assert_eq!(
        authorize(
            service.clone(),
            config,
            input,
            cancel,
            |url| callback(url, false),
            &exchange
        )
        .await,
        Err(IpcError::RemoteUnavailable)
    );
    assert_eq!(exchange.0.load(Ordering::SeqCst), 0);
    assert!(service.list_accounts().await.unwrap().is_empty());
    assert!(vault.0.lock().unwrap().is_empty());
}
#[tokio::test]
async fn cancellation_drops_callback_listener_and_cleans_staged_secrets() {
    let (service, vault, config, input) = fixture();
    let (sender, cancel) = watch::channel(false);
    let exchange = TestExchange(AtomicUsize::new(0));
    let address = Arc::new(Mutex::new(None));
    let captured = address.clone();
    let result = authorize(
        service.clone(),
        config,
        input,
        cancel,
        move |url| {
            let url = Url::parse(url).unwrap();
            let redirect = url
                .query_pairs()
                .find(|(k, _)| k == "redirect_uri")
                .unwrap()
                .1
                .into_owned();
            *captured.lock().unwrap() = Some(Url::parse(&redirect).unwrap().port().unwrap());
            sender.send_replace(true);
            Ok(())
        },
        &exchange,
    )
    .await;
    assert_eq!(result, Err(IpcError::OAuthCancelled));
    assert_eq!(exchange.0.load(Ordering::SeqCst), 0);
    assert!(vault.0.lock().unwrap().is_empty());
    let port = address.lock().unwrap().unwrap();
    assert!(tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_err());
}
#[tokio::test]
async fn invalid_configuration_and_browser_failure_cannot_register_an_account() {
    let (service, vault, mut config, input) = fixture();
    config.imap_port = 0;
    let (_sender, cancel) = watch::channel(false);
    let exchange = TestExchange(AtomicUsize::new(0));
    assert_eq!(
        authorize(
            service.clone(),
            config,
            input,
            cancel.clone(),
            |_| panic!("browser must remain closed"),
            &exchange
        )
        .await,
        Err(IpcError::InvalidRequest)
    );
    let (_, _, config, input) = fixture();
    assert_eq!(
        authorize(
            service.clone(),
            config,
            input,
            cancel,
            |_| Err(IpcError::RemoteUnavailable),
            &exchange
        )
        .await,
        Err(IpcError::RemoteUnavailable)
    );
    assert!(vault.0.lock().unwrap().is_empty());
    assert!(service.list_accounts().await.unwrap().is_empty());
}

struct DiscoveryFixture(AtomicUsize);
#[async_trait]
impl mail_engine::discovery::DiscoveryTransport for DiscoveryFixture {
    async fn ispdb(&self, domain: &str) -> Result<Vec<u8>, mail_engine::discovery::DiscoveryError> {
        assert_eq!(domain, "example.com");
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(br#"<clientConfig><emailProvider><incomingServer type="imap"><hostname>imap.example.com</hostname><port>993</port><socketType>SSL</socketType></incomingServer></emailProvider></clientConfig>"#.to_vec())
    }
    async fn probe(&self, host: &str) -> Result<bool, mail_engine::discovery::DiscoveryError> {
        assert_eq!(host, "imap.example.com");
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(mail_engine::discovery::DiscoveryError::Unavailable)
    }
    async fn mx(
        &self,
        domain: &str,
    ) -> Result<Vec<String>, mail_engine::discovery::DiscoveryError> {
        assert_eq!(domain, "example.com");
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(vec!["aspmx.l.google.com.".into()])
    }
}

#[tokio::test]
async fn mx_ipc_service_returns_preset_only_on_explicit_call() {
    let transport = DiscoveryFixture(AtomicUsize::new(0));
    let found = detect_provider_mx("reader@example.com", &transport)
        .await
        .unwrap();
    assert_eq!(found.source, mail_engine::discovery::DiscoverySource::Mx);
    assert_eq!(found.imap_host.as_deref(), Some("imap.gmail.com"));
    assert_eq!(transport.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        detect_provider_mx("not-an-email", &transport).await,
        Err(IpcError::InvalidRequest)
    );
}

#[tokio::test]
async fn discovery_ipc_service_is_offline_by_default_and_returns_secret_free_metadata() {
    let transport = DiscoveryFixture(AtomicUsize::new(0));
    let offline = discover_provider("reader@example.com", false, &transport)
        .await
        .unwrap();
    assert_eq!(
        offline.source,
        mail_engine::discovery::DiscoverySource::Unresolved
    );
    assert_eq!(transport.0.load(Ordering::SeqCst), 0);
    let online = discover_provider("reader@example.com", true, &transport)
        .await
        .unwrap();
    let json = serde_json::to_value(online).unwrap();
    assert_eq!(json["imap_host"], "imap.example.com");
    assert_eq!(json["imap_port"], 993);
    assert_eq!(json["use_ssl"], true);
    assert_eq!(json["recommended_auth"], "AppPassword");
    assert!(json["oauth_endpoints"].is_null());
    assert!(!json.to_string().contains("reader@"));
    assert_eq!(transport.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        discover_provider("bad\r\naddress", false, &transport).await,
        Err(IpcError::InvalidRequest)
    );
    assert_eq!(
        probe_provider("imap.example.com", &transport).await,
        Err(IpcError::RemoteUnavailable)
    );
    assert_eq!(
        probe_provider("127.0.0.1", &transport).await,
        Err(IpcError::InvalidRequest)
    );
}

#[tokio::test]
async fn oauth_callback_errors_classify_without_echoing_provider_text() {
    for (oauth_error, expected) in [
        ("access_denied", IpcError::OAuthAccessDenied),
        ("redirect_uri_mismatch", IpcError::OAuthInvalidClient),
    ] {
        let (service, vault, config, input) = fixture();
        let (_sender, cancel) = watch::channel(false);
        let exchange = TestExchange(AtomicUsize::new(0));
        let err_code = oauth_error.to_string();
        let result = authorize(
            service.clone(),
            config,
            input,
            cancel,
            move |url| {
                let parsed = Url::parse(url).unwrap();
                let redirect = parsed
                    .query_pairs()
                    .find(|(k, _)| k == "redirect_uri")
                    .unwrap()
                    .1
                    .into_owned();
                let state = parsed
                    .query_pairs()
                    .find(|(k, _)| k == "state")
                    .unwrap()
                    .1
                    .into_owned();
                let port = Url::parse(&redirect).unwrap().port().unwrap();
                tokio::spawn(async move {
                    let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
                        .await
                        .unwrap();
                    socket
                        .write_all(
                            format!(
                                "GET /callback?error={err_code}&error_description=synthetic-secret-desc&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
                            )
                            .as_bytes(),
                        )
                        .await
                        .unwrap();
                    let mut response = Vec::new();
                    socket.read_to_end(&mut response).await.unwrap();
                    let body = String::from_utf8_lossy(&response);
                    assert!(!body.contains("synthetic-secret-desc"));
                });
                Ok(())
            },
            &exchange,
        )
        .await;
        assert_eq!(result, Err(expected));
        assert_eq!(exchange.0.load(Ordering::SeqCst), 0);
        assert!(vault.0.lock().unwrap().is_empty());
    }
}

struct ReauthExchange;
#[async_trait]
impl TokenExchange for ReauthExchange {
    async fn exchange(&self, request: TokenRequest<'_>) -> Result<TokenResponse, IpcError> {
        assert_eq!(request.client_id, "synthetic-client");
        assert_eq!(request.client_secret, Some("synthetic-secret"));
        Ok(TokenResponse {
            access_token: "synthetic-renewed-access".into(),
            refresh_token: Some("synthetic-renewed-refresh".into()),
            token_type: "Bearer".into(),
        })
    }
}

#[tokio::test]
async fn reauthorize_updates_vault_tokens_in_place_without_deleting_indexed_mail() {
    let parse_err = serde_json::from_str::<IncomingReauthorization>(
        r#"{"account_id":"a1","client_secret":42,"client_id":"synthetic-private-id"}"#,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(parse_err.contains("invalid reauthorization material"));
    assert!(!parse_err.contains("synthetic-private-id"));

    let (service, vault, mut config, mut input) = fixture();
    config.auth_method = AuthMethod::OAuth2ByoCredentials;
    input.0.client_secret = Some("synthetic-secret".into());
    let (_sender, cancel) = watch::channel(false);
    let exchange = TestExchange(AtomicUsize::new(0));
    let id = authorize(
        service.clone(),
        config,
        input,
        cancel.clone(),
        |url| callback(url, true),
        &exchange,
    )
    .await
    .unwrap();

    use storage::MailRepository;
    // Seed indexed local message + checkpoint + diagnostic on the existing account.
    service
        .store
        .upsert_folder_sync_state(&storage::FolderSyncState {
            account_id: id.clone(),
            folder_name: "INBOX".into(),
            uidvalidity: 99,
            uidnext: 10,
            highest_modseq: Some(42),
        })
        .unwrap();
    service
        .store
        .insert_message(&storage::MessageRecord {
            id: "preserved-msg-1".into(),
            account_id: id.clone(),
            folder: "INBOX".into(),
            uid: 9,
            message_id_header: Some("<preserved@example.org>".into()),
            in_reply_to: None,
            references_header: None,
            subject: "Existing indexed email".into(),
            sender: "sender@example.org".into(),
            recipients: "reader@example.com".into(),
            received_at_epoch_ms: 1_730_000_000_000,
            is_read: false,
            has_attachments: false,
            category_badge: Some("Personal".into()),
            triage_badges: Vec::new(),
            raw_headers: "From: sender@example.org\r\nSubject: Existing\r\n\r\n".into(),
            text_body: Some("Keep me after reauthorization".into()),
            html_body: None,
            list_unsubscribe: None,
            list_unsubscribe_post: None,
            one_click_unsubscribe_supported: false,
            created_at_epoch_ms: 1_730_000_000_000,
            attachments: Vec::new(),
            tags: vec!["important".into()],
            is_starred: true,
            snooze_until_epoch_ms: None,
            provider_message_id: None,
        })
        .unwrap();
    service.record_account_diagnostic(
        &id,
        Some(crate::ConnectionDiagnosticCode::OAuthAccessDenied),
    );
    assert_eq!(
        service
            .account_diagnostics()
            .await
            .unwrap()
            .get(&id)
            .copied(),
        Some(crate::ConnectionDiagnosticCode::OAuthAccessDenied)
    );

    // Reauthorize omits client_id / client_secret / token_endpoint to reuse vault material.
    let reauth = IncomingReauthorization(ReauthorizationFields {
        account_id: id.clone(),
        authorization_endpoint: "https://auth.example.com/authorize".into(),
        token_endpoint: None,
        scopes: "imap offline_access".into(),
        client_id: None,
        client_secret: None,
    });
    let reauthorized_id = reauthorize(
        service.clone(),
        reauth,
        cancel,
        |url| callback(url, true),
        &ReauthExchange,
    )
    .await
    .unwrap();
    assert_eq!(reauthorized_id, id);

    // Vault tokens rotated in place while account ID, messages, tags, and sync state stay intact.
    let key = VaultKey::for_account(&id, "imap-auth").unwrap();
    let auth = AccountAuthMaterial::from_bytes(&vault.get_secret(&key).unwrap()).unwrap();
    match auth {
        AccountAuthMaterial::OAuth2ByoCredentials {
            access_token,
            refresh_token,
            ..
        } => {
            assert_eq!(access_token, "synthetic-renewed-access");
            assert_eq!(refresh_token.as_deref(), Some("synthetic-renewed-refresh"));
        }
        other => panic!("unexpected auth variant: {other:?}"),
    }
    assert_eq!(service.list_accounts().await.unwrap().len(), 1);
    let preserved = service
        .store
        .get_message_by_id("preserved-msg-1")
        .unwrap()
        .unwrap();
    assert_eq!(preserved.account_id, id);
    assert_eq!(preserved.tags, vec!["important"]);
    assert!(preserved.is_starred);
    let checkpoint = service
        .store
        .get_folder_sync_state(&id, "INBOX")
        .unwrap()
        .unwrap();
    assert_eq!(checkpoint.uidvalidity, 99);
    assert!(service.account_diagnostics().await.unwrap().is_empty());
}
