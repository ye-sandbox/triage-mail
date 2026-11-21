//! Local-first provider discovery. Online lookup and endpoint probing are separate user actions.
mod mx;
mod native;
mod presets;
mod xml;

use std::time::Duration;

use async_trait::async_trait;
use mail_parser::{Address, MessageParser};
use serde::{Deserialize, Serialize};

use crate::AuthMethod;
pub use native::NativeDiscovery;

pub const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    Preset,
    Ispdb,
    Mx,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthEndpoints {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub scopes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredProvider {
    pub domain: String,
    pub provider_name: String,
    pub imap_host: Option<String>,
    pub imap_port: u16,
    pub use_ssl: bool,
    pub recommended_auth: AuthMethod,
    pub oauth_endpoints: Option<OAuthEndpoints>,
    pub help_url: Option<String>,
    pub source: DiscoverySource,
}

impl DiscoveredProvider {
    fn unresolved(domain: String) -> Self {
        Self {
            provider_name: domain.clone(),
            domain,
            imap_host: None,
            imap_port: 993,
            use_ssl: true,
            recommended_auth: AuthMethod::AppPassword,
            oauth_endpoints: None,
            help_url: None,
            source: DiscoverySource::Unresolved,
        }
    }
}

/// Fixed errors never retain an address, provider payload or network diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DiscoveryError {
    #[error("invalid email address or host")]
    InvalidInput,
    #[error("provider discovery unavailable")]
    Unavailable,
}

#[async_trait]
pub trait DiscoveryTransport: Send + Sync {
    /// Receives only a normalized domain, never the local part of an address.
    async fn ispdb(&self, domain: &str) -> Result<Vec<u8>, DiscoveryError>;
    /// Explicitly confirmed public host, implicit TLS on port 993; no authentication.
    async fn probe(&self, host: &str) -> Result<bool, DiscoveryError>;
    /// MX exchange names of a normalized domain only; never the local part.
    async fn mx(&self, domain: &str) -> Result<Vec<String>, DiscoveryError>;
}

/// Accept exactly one RFC 5322 mailbox; reject control/header injection and address literals.
pub fn email_domain(email: &str) -> Result<String, DiscoveryError> {
    if email.len() > 320 || email.chars().any(char::is_control) {
        return Err(DiscoveryError::InvalidInput);
    }
    if let Some(domain) = quoted_local_domain(email.trim()) {
        return domain;
    }
    let raw = format!("From: {}\r\n\r\n", email.trim());
    let message = MessageParser::default()
        .parse_headers(&raw)
        .ok_or(DiscoveryError::InvalidInput)?;
    let from = message.from().ok_or(DiscoveryError::InvalidInput)?;
    let mut addresses = from.iter();
    let Some(Address::Mailbox(mailbox)) = addresses.next() else {
        return Err(DiscoveryError::InvalidInput);
    };
    if addresses.next().is_some() {
        return Err(DiscoveryError::InvalidInput);
    }
    let address = mailbox.address().ok_or(DiscoveryError::InvalidInput)?;
    let (local, domain) = address
        .rsplit_once('@')
        .ok_or(DiscoveryError::InvalidInput)?;
    if local.is_empty() || local.len() > 64 || (!local.starts_with('"') && local.contains('@')) {
        return Err(DiscoveryError::InvalidInput);
    }
    normalize_host(domain)
}

fn quoted_local_domain(email: &str) -> Option<Result<String, DiscoveryError>> {
    let quoted = email.strip_prefix('"')?;
    let mut escaped = false;
    for (index, ch) in quoted.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            let domain = quoted[index + 1..].strip_prefix('@')?;
            // INVARIANT: mail-parser interprets a bare quoted local part as a display name.
            // Preserve its RFC 5322 addr-spec meaning without weakening domain validation.
            return Some(if index == 0 || index > 64 {
                Err(DiscoveryError::InvalidInput)
            } else {
                normalize_host(domain)
            });
        }
    }
    None
}

pub(super) fn normalize_host(host: &str) -> Result<String, DiscoveryError> {
    if host.contains('%') || host.chars().any(char::is_whitespace) {
        return Err(DiscoveryError::InvalidInput);
    }
    let url::Host::Domain(host) =
        url::Host::parse(host).map_err(|_| DiscoveryError::InvalidInput)?
    else {
        return Err(DiscoveryError::InvalidInput);
    };
    if host.len() > 253
        || !host.contains('.')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(DiscoveryError::InvalidInput);
    }
    Ok(host)
}

/// Offline by default. An unresolved result contains no host and never starts a probe.
pub async fn discover(
    email: &str,
    online: bool,
    transport: &dyn DiscoveryTransport,
) -> Result<DiscoveredProvider, DiscoveryError> {
    let domain = email_domain(email)?;
    if let Some(preset) = presets::lookup(&domain) {
        return Ok(preset);
    }
    if online {
        // INVARIANT: One deadline covers DNS and the complete HTTP transfer; no retries/probes.
        // XML parsing is synchronous and bounded by MAX_CONFIG_BYTES and a depth limit.
        if let Ok(Ok(bytes)) =
            tokio::time::timeout(DISCOVERY_TIMEOUT, transport.ispdb(&domain)).await
        {
            if let Some(provider) = xml::parse(&domain, &bytes) {
                return Ok(provider);
            }
        }
    }
    Ok(DiscoveredProvider::unresolved(domain))
}

/// Explicit user action only (never called by `discover`). Falls back to the unresolved result.
pub async fn detect_mx(
    email: &str,
    transport: &dyn DiscoveryTransport,
) -> Result<DiscoveredProvider, DiscoveryError> {
    let domain = email_domain(email)?;
    // INVARIANT: One 3s deadline, <=10 records; the answer only selects a preset.
    if let Ok(Ok(raw)) = tokio::time::timeout(DISCOVERY_TIMEOUT, transport.mx(&domain)).await {
        if let Some(provider) = mx::classify(&domain, &mx::sanitize(raw)) {
            return Ok(provider);
        }
    }
    Ok(DiscoveredProvider::unresolved(domain))
}

pub async fn probe_confirmed_host(
    host: &str,
    transport: &dyn DiscoveryTransport,
) -> Result<bool, DiscoveryError> {
    let host = normalize_host(host)?;
    tokio::time::timeout(DISCOVERY_TIMEOUT, transport.probe(&host))
        .await
        .map_err(|_| DiscoveryError::Unavailable)?
}

#[cfg(test)]
mod tests;
