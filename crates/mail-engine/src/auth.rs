// INVARIANT: Zero OAuth tokens, client secrets, or user passwords may ever be exposed
// in cleartext through Debug or Display formatting.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;
use url::Url;

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
}
