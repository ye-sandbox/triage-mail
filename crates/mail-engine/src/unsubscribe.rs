// INVARIANT: RFC 8058 One-Click Unsubscribe requests are the ONLY outbound HTTP calls allowed
// from `crates/mail-engine`, and they MUST only be triggered on explicit user action.
// INVARIANT: Enforce strict HTTPS domain validation and disable redirects to prevent SSRF
// against loopback, link-local, or private network endpoints.

use std::time::Duration;
use url::{Host, Url};

use crate::{EngineError, UnsubscribeDescriptor};

pub(crate) mod dns;

/// Exact form body mandated by RFC 8058 Section 3.1.
pub const RFC8058_POST_BODY: &str = "List-Unsubscribe=One-Click";

/// Content-Type header mandated by RFC 8058 Section 3.1.
pub const RFC8058_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

/// Default timeout for RFC 8058 HTTP POST execution (10 seconds).
pub const DEFAULT_UNSUBSCRIBE_TIMEOUT_SECS: u64 = 10;

/// Configuration for the RFC 8058 One-Click Unsubscribe HTTP executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rfc8058ClientConfig {
    pub timeout: Duration,
    /// Strictly for local loopback integration tests (`127.0.0.1` / `localhost`).
    /// Must remain `false` in production.
    pub allow_loopback_http_for_tests: bool,
}

impl Default for Rfc8058ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_UNSUBSCRIBE_TIMEOUT_SECS),
            allow_loopback_http_for_tests: false,
        }
    }
}

/// Successful execution summary of an RFC 8058 One-Click Unsubscribe POST request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsubscribeOutcome {
    pub target_uri: String,
    pub status_code: u16,
}

/// Validates an RFC 8058 target URI against scheme, credential, and SSRF domain invariants.
pub fn validate_rfc8058_uri(
    uri_str: &str,
    allow_loopback_http_for_tests: bool,
) -> Result<Url, EngineError> {
    let parsed = Url::parse(uri_str.trim()).map_err(|e| {
        EngineError::UnsubscribeFailed(format!("invalid RFC 8058 URI `{uri_str}`: {e}"))
    })?;

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(EngineError::UnsubscribeFailed(
            "RFC 8058 URI must not contain embedded credentials".to_string(),
        ));
    }

    let is_loopback = match parsed.host() {
        Some(Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };

    if allow_loopback_http_for_tests && is_loopback {
        if parsed.scheme() != "https" && parsed.scheme() != "http" {
            return Err(EngineError::UnsubscribeFailed(format!(
                "unsupported URI scheme `{}`",
                parsed.scheme()
            )));
        }
        return Ok(parsed);
    }

    if parsed.scheme() != "https" {
        return Err(EngineError::UnsubscribeFailed(format!(
            "RFC 8058 requires `https` scheme, got `{}`",
            parsed.scheme()
        )));
    }

    match parsed.host() {
        Some(Host::Domain(domain)) => {
            let trimmed = domain.trim().to_ascii_lowercase();
            if trimmed == "localhost"
                || trimmed.ends_with(".local")
                || trimmed.ends_with(".internal")
                || !trimmed.contains('.')
            {
                return Err(EngineError::UnsubscribeFailed(format!(
                    "rejected non-public unsubscribe domain `{domain}`"
                )));
            }
            if trimmed
                .split('.')
                .any(|label| label.is_empty() || label.starts_with('-') || label.ends_with('-'))
            {
                return Err(EngineError::UnsubscribeFailed(format!(
                    "malformed domain labels in `{domain}`"
                )));
            }
            Ok(parsed)
        }
        Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => Err(EngineError::UnsubscribeFailed(
            "RFC 8058 target URI must use a public DNS domain name, not an IP literal".to_string(),
        )),
        None => Err(EngineError::UnsubscribeFailed(
            "RFC 8058 target URI is missing a host".to_string(),
        )),
    }
}

/// Validates an [`UnsubscribeDescriptor`] before performing an RFC 8058 One-Click POST.
pub fn validate_rfc8058_descriptor(
    descriptor: &UnsubscribeDescriptor,
    allow_loopback_http_for_tests: bool,
) -> Result<Url, EngineError> {
    if !descriptor.supports_rfc8058_one_click {
        return Err(EngineError::UnsubscribeFailed(
            "message does not support RFC 8058 One-Click Unsubscribe".to_string(),
        ));
    }

    let uri = descriptor.https_uri.as_deref().ok_or_else(|| {
        EngineError::UnsubscribeFailed(
            "missing HTTPS URI in List-Unsubscribe descriptor".to_string(),
        )
    })?;

    validate_rfc8058_uri(uri, allow_loopback_http_for_tests)
}

/// Asynchronous HTTP POST executor for RFC 8058 One-Click Unsubscribe.
pub struct Rfc8058UnsubscribeClient {
    config: Rfc8058ClientConfig,
    http: reqwest::Client,
}

impl Rfc8058UnsubscribeClient {
    /// Builds a new `Rfc8058UnsubscribeClient` with redirects disabled and strict timeouts.
    pub fn new(config: Rfc8058ClientConfig) -> Result<Self, EngineError> {
        Self::with_resolver(config, dns::resolver(config.allow_loopback_http_for_tests))
    }

    fn with_resolver(
        config: Rfc8058ClientConfig,
        resolver: impl reqwest::dns::Resolve + 'static,
    ) -> Result<Self, EngineError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            // INVARIANT: Proxy DNS must not bypass the address policy or leak unsubscribe URLs.
            .no_proxy()
            .dns_resolver(std::sync::Arc::new(resolver))
            .build()
            .map_err(|e| {
                EngineError::UnsubscribeFailed(format!("failed building HTTP client: {e}"))
            })?;

        Ok(Self { config, http })
    }

    /// Executes the RFC 8058 One-Click HTTP POST (`List-Unsubscribe=One-Click`) against the validated URI.
    pub async fn execute_one_click(
        &self,
        descriptor: &UnsubscribeDescriptor,
    ) -> Result<UnsubscribeOutcome, EngineError> {
        let target_url =
            validate_rfc8058_descriptor(descriptor, self.config.allow_loopback_http_for_tests)?;

        let response = self
            .http
            .post(target_url.clone())
            .header(reqwest::header::CONTENT_TYPE, RFC8058_CONTENT_TYPE)
            .body(RFC8058_POST_BODY)
            .send()
            .await
            .map_err(|e| {
                EngineError::UnsubscribeFailed(format!("HTTP POST request failed: {e}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(EngineError::UnsubscribeFailed(format!(
                "unsubscribe endpoint returned HTTP {}",
                status.as_u16()
            )));
        }

        Ok(UnsubscribeOutcome {
            target_uri: target_url.to_string(),
            status_code: status.as_u16(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::dns::{Addrs, Name, Resolve, Resolving};
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    struct FixedDns(Vec<SocketAddr>);

    impl Resolve for FixedDns {
        fn resolve(&self, _: Name) -> Resolving {
            let addresses = self.0.clone();
            Box::pin(async move { Ok(Box::new(addresses.into_iter()) as Addrs) })
        }
    }

    fn guarded_dns(
        addresses: &[&str],
        allow_loopback_for_tests: bool,
    ) -> dns::PublicResolver<FixedDns> {
        dns::PublicResolver {
            inner: FixedDns(addresses.iter().map(|a| a.parse().unwrap()).collect()),
            allow_loopback_for_tests,
        }
    }

    #[tokio::test]
    async fn dns_rejects_empty_private_and_mixed_answers_without_a_second_lookup() {
        for addresses in [
            vec![],
            vec!["127.0.0.1:0"],
            vec!["10.0.0.1:0"],
            vec!["[::ffff:127.0.0.1]:0"],
            vec!["8.8.8.8:0", "169.254.169.254:0"],
            vec!["[2606:4700:4700::1111]:0", "[fc00::1]:0"],
        ] {
            for test_mode in [false, true] {
                assert!(guarded_dns(&addresses, test_mode)
                    .resolve("unsubscribe.example.org".parse().unwrap())
                    .await
                    .is_err());
            }
        }
        let expected = vec![
            "8.8.8.8:0".parse::<SocketAddr>().unwrap(),
            "[2606:4700:4700::1111]:0".parse().unwrap(),
        ];
        let actual: Vec<_> = guarded_dns(&["8.8.8.8:0", "[2606:4700:4700::1111]:0"], false)
            .resolve("unsubscribe.example.org".parse().unwrap())
            .await
            .unwrap()
            .collect();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn loopback_dns_exception_is_only_for_explicit_localhost_test_mode() {
        for (host, enabled, allowed) in [
            ("localhost", false, false),
            ("localhost", true, true),
            ("unsubscribe.example.org", true, false),
        ] {
            assert_eq!(
                guarded_dns(&["127.0.0.1:0", "[::1]:0"], enabled)
                    .resolve(host.parse().unwrap())
                    .await
                    .is_ok(),
                allowed
            );
        }
        assert!(guarded_dns(&["10.0.0.1:0"], true)
            .resolve("localhost".parse().unwrap())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn http_connector_rejects_public_hostname_resolving_to_loopback_before_connecting() {
        let uri = "https://unsubscribe.example.org/one-click";
        // Reproduces the gap: hostname syntax validation alone accepts this target.
        assert!(validate_rfc8058_uri(uri, false).is_ok());
        let client = Rfc8058UnsubscribeClient::with_resolver(
            Default::default(),
            guarded_dns(&["127.0.0.1:0"], false),
        )
        .unwrap();
        let error = client
            .http
            .post(uri)
            .body(RFC8058_POST_BODY)
            .send()
            .await
            .unwrap_err();
        let mut source: &dyn std::error::Error = &error;
        let mut blocked = false;
        loop {
            blocked |= source
                .to_string()
                .contains("unsubscribe DNS returned a non-public");
            match source.source() {
                Some(next) => source = next,
                None => break,
            }
        }
        assert!(
            blocked,
            "request must fail at the DNS policy, before any socket connection"
        );
    }

    #[test]
    fn validates_rfc8058_uris_and_rejects_ssrf_or_insecure_schemes() {
        // Valid RFC 2606 HTTPS URI
        let valid = validate_rfc8058_uri("https://unsubscribe.example.com/one-click?id=42", false);
        assert!(valid.is_ok());

        // Reject plain HTTP in production mode
        assert!(validate_rfc8058_uri("http://unsubscribe.example.com/one-click", false).is_err());

        // Reject embedded credentials
        assert!(
            validate_rfc8058_uri("https://user:pass@unsubscribe.example.com/unsub", false).is_err()
        );

        // Reject IP literals (SSRF protection)
        assert!(validate_rfc8058_uri("https://127.0.0.1/unsub", false).is_err());
        assert!(validate_rfc8058_uri("https://169.254.169.254/latest/meta-data", false).is_err());

        // Reject localhost / internal single-label hosts in production mode
        assert!(validate_rfc8058_uri("https://localhost/unsub", false).is_err());
        assert!(validate_rfc8058_uri("https://intranet/unsub", false).is_err());
        assert!(validate_rfc8058_uri("https://mailer.internal/unsub", false).is_err());
    }

    #[test]
    fn rejects_descriptor_without_rfc8058_post_flag() {
        let desc = UnsubscribeDescriptor {
            https_uri: Some("https://unsubscribe.example.org/u/99".to_string()),
            mailto_uri: None,
            supports_rfc8058_one_click: false,
            raw_list_unsubscribe: Some("<https://unsubscribe.example.org/u/99>".to_string()),
            raw_list_unsubscribe_post: None,
        };
        assert!(validate_rfc8058_descriptor(&desc, false).is_err());
    }

    #[tokio::test]
    async fn executes_rfc8058_http_post_with_exact_form_body() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let n = socket.read(&mut buf).await.unwrap();
            let raw_req = String::from_utf8_lossy(&buf[..n]).to_string();

            assert!(raw_req.starts_with("POST /unsubscribe/token-abc HTTP/1.1\r\n"));
            assert!(raw_req
                .to_ascii_lowercase()
                .contains("content-type: application/x-www-form-urlencoded\r\n"));
            assert!(raw_req.ends_with("List-Unsubscribe=One-Click"));

            let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let client = Rfc8058UnsubscribeClient::new(Rfc8058ClientConfig {
            timeout: Duration::from_secs(5),
            allow_loopback_http_for_tests: true,
        })
        .unwrap();

        let descriptor = UnsubscribeDescriptor {
            https_uri: Some(format!("http://{}/unsubscribe/token-abc", addr)),
            mailto_uri: Some("mailto:unsub@example.com".to_string()),
            supports_rfc8058_one_click: true,
            raw_list_unsubscribe: Some(format!("<http://{}/unsubscribe/token-abc>", addr)),
            raw_list_unsubscribe_post: Some("List-Unsubscribe=One-Click".to_string()),
        };

        let outcome = client.execute_one_click(&descriptor).await.unwrap();
        assert_eq!(outcome.status_code, 200);
        server_task.await.unwrap();
    }
}
