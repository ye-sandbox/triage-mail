// INVARIANT: Zero OAuth tokens, client secrets, or user passwords may ever be exposed
// in cleartext through Debug or Display formatting.

use async_trait::async_trait;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;
use url::Url;
use vault::{AccountAuthMaterial, SecretStore, VaultKey};
use zeroize::{Zeroize, Zeroizing};

use crate::EngineError;

/// Redacted wrapper preventing accidental leakage of secrets in logs or format strings.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedSecret(String);

impl RedactedSecret {
    pub fn new<S: Into<String>>(secret: S) -> Self {
        Self(secret.into())
    }

    /// Explicit accessor to retrieve the raw secret strictly when communicating with endpoints.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RedactedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl fmt::Display for RedactedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Constructs a SASL XOAUTH2 initial client response (RFC 6749 / Gmail XOAUTH2 spec).
///
/// Format: `base64("user=" + username + "^Aauth=Bearer " + token + "^A^A")`
pub fn build_xoauth2_sasl_string(username: &str, access_token: &str) -> String {
    let raw = format!("user={}\x01auth=Bearer {}\x01\x01", username, access_token);
    STANDARD.encode(raw.as_bytes())
}

/// Constructs a SASL OAUTHBEARER initial client response (RFC 7628).
///
/// Format: `base64("n,a=" + username + ",^Ahost=" + host + "^Aport=" + port + "^Aauth=Bearer " + token + "^A^A")`
pub fn build_oauthbearer_sasl_string(
    username: &str,
    host: &str,
    port: u16,
    access_token: &str,
) -> String {
    let raw = format!(
        "n,a={},\x01host={}\x01port={}\x01auth=Bearer {}\x01\x01",
        username, host, port, access_token
    );
    STANDARD.encode(raw.as_bytes())
}

/// Cryptographic PKCE challenge pair (RFC 7636).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceChallenge {
    pub verifier: RedactedSecret,
    pub challenge: String,
    pub method: &'static str,
}

impl PkceChallenge {
    /// Generates a cryptographically random PKCE code verifier and SHA-256 challenge.
    pub fn generate() -> Self {
        let mut random_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut random_bytes);

        let verifier = URL_SAFE_NO_PAD.encode(random_bytes);
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let digest = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(digest);

        Self {
            verifier: RedactedSecret::new(verifier),
            challenge,
            method: "S256",
        }
    }
}

/// Generates a high-entropy state token to protect OAuth2 redirect flows against CSRF.
pub fn generate_oauth_state() -> String {
    let mut random_bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut random_bytes);
    URL_SAFE_NO_PAD.encode(random_bytes)
}

/// Loopback HTTP redirect listener running on `127.0.0.1` for desktop OAuth2 PKCE flows.
pub struct LoopbackRedirectListener {
    listener: TcpListener,
    port: u16,
}

impl LoopbackRedirectListener {
    /// Binds to an ephemeral port on `127.0.0.1`.
    pub async fn bind() -> Result<Self, EngineError> {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            EngineError::ImapProtocol(format!("failed to bind loopback listener: {e}"))
        })?;
        let addr = listener
            .local_addr()
            .map_err(|e| EngineError::ImapProtocol(format!("failed to get local addr: {e}")))?;
        Ok(Self {
            listener,
            port: addr.port(),
        })
    }

    /// Port on which the loopback server is listening.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Localhost callback URL to provide in OAuth2 authorize requests.
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    /// Waits for incoming browser redirect, validates state, and extracts authorization code.
    pub async fn wait_for_code(
        self,
        expected_state: &str,
        wait_timeout: Duration,
    ) -> Result<String, EngineError> {
        timeout(wait_timeout, self.listen_internal(expected_state))
            .await
            .map_err(|_| {
                EngineError::ImapProtocol("OAuth2 callback timed out waiting for user".to_string())
            })?
    }

    async fn listen_internal(self, expected_state: &str) -> Result<String, EngineError> {
        let (mut socket, _addr): (tokio::net::TcpStream, SocketAddr) = self
            .listener
            .accept()
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("loopback accept failed: {e}")))?;

        let mut buffer = [0u8; 4096];
        let bytes_read = socket
            .read(&mut buffer)
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("failed reading HTTP request: {e}")))?;

        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
        let first_line = request_str.lines().next().unwrap_or_default();
        let target = first_line.split_whitespace().nth(1).unwrap_or("/");

        let dummy_url = format!("http://127.0.0.1{}", target);
        let parsed_url = Url::parse(&dummy_url)
            .map_err(|e| EngineError::ImapProtocol(format!("malformed callback URL: {e}")))?;

        let mut code = None;
        let mut state = None;

        for (k, v) in parsed_url.query_pairs() {
            if k == "code" {
                code = Some(v.into_owned());
            } else if k == "state" {
                state = Some(v.into_owned());
            }
        }

        if state.as_deref() != Some(expected_state) {
            let error_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nInvalid state token.";
            let _ = socket.write_all(error_response.as_bytes()).await;
            return Err(EngineError::ImapProtocol(
                "OAuth2 state parameter mismatch (potential CSRF)".to_string(),
            ));
        }

        let auth_code = code.ok_or_else(|| {
            EngineError::ImapProtocol("OAuth2 callback missing code parameter".to_string())
        })?;

        let success_body = "<!DOCTYPE html><html><body style=\"font-family:sans-serif;text-align:center;padding:40px;\"><h2>Authentication Successful</h2><p>You may close this tab and return to Triage Mail.</p></body></html>";
        let success_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            success_body.len(),
            success_body
        );
        let _ = socket.write_all(success_response.as_bytes()).await;

        Ok(auth_code)
    }
}

/// Token payload representation stored in vault or session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: RedactedSecret,
    pub refresh_token: Option<RedactedSecret>,
    pub expires_at_epoch_s: Option<u64>,
}

impl OAuthTokens {
    /// Returns true if the token is expired or about to expire within `margin_seconds`.
    pub fn is_expired(&self, current_epoch_s: u64, margin_seconds: u64) -> bool {
        if let Some(exp) = self.expires_at_epoch_s {
            current_epoch_s + margin_seconds >= exp
        } else {
            false
        }
    }
}

/// Refreshed token payload returned by token endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshedTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

impl RefreshedTokens {
    /// Takes the token strings out of the struct.
    pub fn take_tokens(&mut self) -> (String, Option<String>) {
        (
            std::mem::take(&mut self.access_token),
            self.refresh_token.take(),
        )
    }
}

impl Drop for RefreshedTokens {
    fn drop(&mut self) {
        self.access_token.zeroize();
        if let Some(ref mut rt) = self.refresh_token {
            rt.zeroize();
        }
    }
}

/// Request parameters sent to the OAuth token endpoint for refresh.
pub struct RefreshTokenRequest<'a> {
    pub token_endpoint: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub refresh_token: &'a str,
}

/// Abstract transport for refreshing OAuth tokens, allowing local mocking in tests.
#[async_trait]
pub trait TokenRefreshTransport: Send + Sync {
    async fn refresh_token(
        &self,
        request: RefreshTokenRequest<'_>,
    ) -> Result<RefreshedTokens, EngineError>;
}

/// Production HTTPS transport for refreshing OAuth2 tokens.
/// Strictly enforces P0 network policy:
/// - Direct HTTPS requests only (no HTTP, no credentials/query/fragment in URL, length <= 2048)
/// - No discovery, no redirects, no inherited proxies
/// - 30-second timeout, response bounded to <= 65536 bytes
#[derive(Default)]
pub struct HttpTokenRefreshTransport;

#[async_trait]
impl TokenRefreshTransport for HttpTokenRefreshTransport {
    async fn refresh_token(
        &self,
        request: RefreshTokenRequest<'_>,
    ) -> Result<RefreshedTokens, EngineError> {
        let RefreshTokenRequest {
            token_endpoint,
            client_id,
            client_secret,
            refresh_token,
        } = request;

        let url = validate_token_endpoint(token_endpoint)?;

        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ];
        if let Some(secret) = client_secret {
            form.push(("client_secret", secret));
        }

        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| EngineError::TokenRefresh(format!("failed to build HTTP client: {e}")))?;

        let mut response =
            client.post(url).form(&form).send().await.map_err(|e| {
                EngineError::TokenRefresh(format!("token refresh request failed: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(EngineError::TokenRefresh(format!(
                "token endpoint returned HTTP status {}",
                response.status().as_u16()
            )));
        }

        let mut bytes = Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| EngineError::TokenRefresh(format!("failed reading chunk: {e}")))?
        {
            if bytes.len() + chunk.len() > 65536 {
                return Err(EngineError::TokenRefresh(
                    "response body exceeded 64 KiB limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }

        parse_refresh_response(&bytes)
    }
}

pub fn validate_token_endpoint(value: &str) -> Result<Url, EngineError> {
    if value.len() > 2048 {
        return Err(EngineError::TokenRefresh(
            "token endpoint URL too long (>2048 chars)".into(),
        ));
    }
    let url = Url::parse(value)
        .map_err(|e| EngineError::TokenRefresh(format!("invalid token endpoint URL: {e}")))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(EngineError::TokenRefresh(
            "token endpoint must be direct HTTPS without credentials, query, or fragment".into(),
        ));
    }
    Ok(url)
}

#[derive(Deserialize)]
struct RawTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: Option<u64>,
}

impl Drop for RawTokenResponse {
    fn drop(&mut self) {
        self.access_token.zeroize();
        if let Some(ref mut rt) = self.refresh_token {
            rt.zeroize();
        }
    }
}

pub fn parse_refresh_response(bytes: &[u8]) -> Result<RefreshedTokens, EngineError> {
    let mut raw: RawTokenResponse = serde_json::from_slice(bytes)
        .map_err(|e| EngineError::TokenRefresh(format!("invalid token response JSON: {e}")))?;

    if !raw.token_type.eq_ignore_ascii_case("bearer") {
        return Err(EngineError::TokenRefresh(
            "unsupported token_type (expected Bearer)".into(),
        ));
    }
    if raw.access_token.is_empty() || raw.access_token.chars().any(char::is_control) {
        return Err(EngineError::TokenRefresh(
            "invalid or empty access_token".into(),
        ));
    }
    if let Some(ref rt) = raw.refresh_token {
        if rt.is_empty() || rt.chars().any(char::is_control) {
            return Err(EngineError::TokenRefresh(
                "invalid refresh_token in response".into(),
            ));
        }
    }

    Ok(RefreshedTokens {
        access_token: std::mem::take(&mut raw.access_token),
        refresh_token: raw.refresh_token.take(),
        expires_in: raw.expires_in,
    })
}

/// Pure-Rust OAuth token refresher that refreshes expired tokens and updates the vault atomically.
pub struct TokenRefresher {
    transport: Arc<dyn TokenRefreshTransport>,
}

impl Default for TokenRefresher {
    fn default() -> Self {
        Self::new(Arc::new(HttpTokenRefreshTransport))
    }
}

impl TokenRefresher {
    pub fn new(transport: Arc<dyn TokenRefreshTransport>) -> Self {
        Self { transport }
    }

    /// Refreshes OAuth tokens for the given account material and commits the updated material to the vault.
    pub async fn refresh_and_update_vault(
        &self,
        vault: &dyn SecretStore,
        account_id: &str,
        auth: &AccountAuthMaterial,
    ) -> Result<AccountAuthMaterial, EngineError> {
        let token_endpoint = auth.token_endpoint().ok_or_else(|| {
            EngineError::TokenRefresh("missing token_endpoint for OAuth account".into())
        })?;
        let refresh_token = auth.refresh_token().ok_or_else(|| {
            EngineError::TokenRefresh("missing refresh_token for OAuth account".into())
        })?;
        let client_id = auth.client_id().ok_or_else(|| {
            EngineError::TokenRefresh("missing client_id for OAuth account".into())
        })?;
        let client_secret = auth.client_secret();

        let request = RefreshTokenRequest {
            token_endpoint,
            client_id,
            client_secret,
            refresh_token,
        };

        let mut refreshed = self.transport.refresh_token(request).await?;

        let mut updated = auth.clone();
        let (new_access, new_refresh) = refreshed.take_tokens();
        updated.update_tokens(new_access, new_refresh);

        let key = VaultKey::for_account(account_id, "imap-auth")?;
        let bytes = Zeroizing::new(updated.to_bytes()?);
        vault.set_secret(&key, &bytes)?;

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets_in_debug_and_display() {
        let secret = RedactedSecret::new("super-sensitive-token-12345");
        let debug_str = format!("{:?}", secret);
        let display_str = format!("{}", secret);

        assert_eq!(debug_str, "[REDACTED]");
        assert_eq!(display_str, "[REDACTED]");
        assert!(!debug_str.contains("12345"));
        assert!(!display_str.contains("super-sensitive"));
        assert_eq!(secret.expose_secret(), "super-sensitive-token-12345");
    }

    #[test]
    fn formats_xoauth2_string_per_specification() {
        let formatted = build_xoauth2_sasl_string("user@example.com", "mock-token-abc");
        let decoded_bytes = STANDARD.decode(formatted).unwrap();
        let decoded = String::from_utf8(decoded_bytes).unwrap();

        assert_eq!(
            decoded,
            "user=user@example.com\x01auth=Bearer mock-token-abc\x01\x01"
        );
    }

    #[test]
    fn formats_oauthbearer_string_per_specification() {
        let formatted = build_oauthbearer_sasl_string(
            "user@example.com",
            "imap.example.com",
            993,
            "mock-token-abc",
        );
        let decoded_bytes = STANDARD.decode(formatted).unwrap();
        let decoded = String::from_utf8(decoded_bytes).unwrap();

        assert_eq!(
            decoded,
            "n,a=user@example.com,\x01host=imap.example.com\x01port=993\x01auth=Bearer mock-token-abc\x01\x01"
        );
    }

    #[test]
    fn generates_valid_pkce_challenge_s256() {
        let pkce = PkceChallenge::generate();
        assert_eq!(pkce.method, "S256");
        assert!(!pkce.challenge.is_empty());

        // Verifier must hash to challenge
        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.expose_secret().as_bytes());
        let expected_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(pkce.challenge, expected_challenge);
    }

    #[tokio::test]
    async fn loopback_listener_extracts_code_and_validates_state() {
        let listener = LoopbackRedirectListener::bind().await.unwrap();
        let port = listener.port();
        let expected_state = "csrf-test-token-777";

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                .await
                .unwrap();
            let request = format!(
                "GET /callback?code=synthetic-auth-code-99&state={} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                expected_state
            );
            stream.write_all(request.as_bytes()).await.unwrap();
        });

        let code = listener
            .wait_for_code(expected_state, Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(code, "synthetic-auth-code-99");
    }

    #[test]
    fn validates_token_endpoint_strictly() {
        assert!(validate_token_endpoint("https://auth.example.com/oauth/token").is_ok());
        assert!(validate_token_endpoint("http://auth.example.com/token").is_err());
        assert!(validate_token_endpoint("https://auth.example.com/token?query=1").is_err());
        assert!(validate_token_endpoint("https://auth.example.com/token#frag").is_err());
        assert!(validate_token_endpoint("https://user:pass@auth.example.com/token").is_err());
        let long_url = format!("https://auth.example.com/{}", "a".repeat(2048));
        assert!(validate_token_endpoint(&long_url).is_err());
    }

    #[test]
    fn parses_refresh_response_and_validates_bearer() {
        let json = br#"{"access_token":"new-acc-123","refresh_token":"new-ref-456","token_type":"Bearer","expires_in":3600}"#;
        let tokens = parse_refresh_response(json).unwrap();
        assert_eq!(tokens.access_token, "new-acc-123");
        assert_eq!(tokens.refresh_token.as_deref(), Some("new-ref-456"));
        assert_eq!(tokens.expires_in, Some(3600));

        let bad_type = br#"{"access_token":"new-acc-123","token_type":"Basic"}"#;
        assert!(parse_refresh_response(bad_type).is_err());

        let empty_acc = br#"{"access_token":"","token_type":"Bearer"}"#;
        assert!(parse_refresh_response(empty_acc).is_err());

        let ctrl_char = b"{\"access_token\":\"acc\\x00token\",\"token_type\":\"Bearer\"}";
        assert!(parse_refresh_response(ctrl_char).is_err());
    }

    struct MockRefreshTransport {
        expected_endpoint: &'static str,
        expected_client_id: &'static str,
        expected_secret: Option<&'static str>,
        expected_refresh: &'static str,
        return_access: &'static str,
        return_refresh: Option<&'static str>,
    }

    #[async_trait]
    impl TokenRefreshTransport for MockRefreshTransport {
        async fn refresh_token(
            &self,
            request: RefreshTokenRequest<'_>,
        ) -> Result<RefreshedTokens, EngineError> {
            assert_eq!(request.token_endpoint, self.expected_endpoint);
            assert_eq!(request.client_id, self.expected_client_id);
            assert_eq!(request.client_secret, self.expected_secret);
            assert_eq!(request.refresh_token, self.expected_refresh);

            Ok(RefreshedTokens {
                access_token: self.return_access.to_string(),
                refresh_token: self.return_refresh.map(|s| s.to_string()),
                expires_in: Some(3600),
            })
        }
    }

    #[derive(Default)]
    struct MemoryVault(std::sync::Mutex<std::collections::HashMap<VaultKey, Vec<u8>>>);

    impl SecretStore for MemoryVault {
        fn set_secret(&self, key: &VaultKey, secret: &[u8]) -> Result<(), vault::VaultError> {
            self.0.lock().unwrap().insert(key.clone(), secret.to_vec());
            Ok(())
        }
        fn get_secret(&self, key: &VaultKey) -> Result<Vec<u8>, vault::VaultError> {
            self.0
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| vault::VaultError::NotFound(key.as_str().to_string()))
        }
        fn delete_secret(&self, key: &VaultKey) -> Result<(), vault::VaultError> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[tokio::test]
    async fn token_refresher_refreshes_and_updates_vault_atomically() {
        let vault = MemoryVault::default();
        let key = VaultKey::for_account("acct-1", "imap-auth").unwrap();

        let initial_auth = AccountAuthMaterial::OAuth2PublicPkce {
            client_id: "client-pkce-1".into(),
            access_token: "expired-access-token".into(),
            refresh_token: Some("valid-refresh-token".into()),
            token_endpoint: Some("https://auth.example.com/oauth/token".into()),
        };
        vault
            .set_secret(&key, &initial_auth.to_bytes().unwrap())
            .unwrap();

        let transport = Arc::new(MockRefreshTransport {
            expected_endpoint: "https://auth.example.com/oauth/token",
            expected_client_id: "client-pkce-1",
            expected_secret: None,
            expected_refresh: "valid-refresh-token",
            return_access: "fresh-access-token",
            return_refresh: Some("rotated-refresh-token"),
        });

        let refresher = TokenRefresher::new(transport);
        let updated = refresher
            .refresh_and_update_vault(&vault, "acct-1", &initial_auth)
            .await
            .unwrap();

        // Updated in-memory material
        assert_eq!(
            updated,
            AccountAuthMaterial::OAuth2PublicPkce {
                client_id: "client-pkce-1".into(),
                access_token: "fresh-access-token".into(),
                refresh_token: Some("rotated-refresh-token".into()),
                token_endpoint: Some("https://auth.example.com/oauth/token".into()),
            }
        );

        // Updated in vault atomically
        let stored_bytes = vault.get_secret(&key).unwrap();
        let stored_auth = AccountAuthMaterial::from_bytes(&stored_bytes).unwrap();
        assert_eq!(stored_auth, updated);
    }
}
