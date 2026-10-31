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
    assert_eq!(result, Err(IpcError::RemoteUnavailable));
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
