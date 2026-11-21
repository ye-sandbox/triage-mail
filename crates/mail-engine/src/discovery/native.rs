use std::sync::Arc;

use async_trait::async_trait;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::TcpStream;

use super::{DiscoveryError, DiscoveryTransport, DISCOVERY_TIMEOUT, MAX_CONFIG_BYTES};
use crate::unsubscribe::dns;
use crate::{AuthMethod, ImapEndpointConfig};

pub struct NativeDiscovery;

#[async_trait]
impl DiscoveryTransport for NativeDiscovery {
    async fn ispdb(&self, domain: &str) -> Result<Vec<u8>, DiscoveryError> {
        let domain = super::normalize_host(domain)?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(Arc::new(dns::resolver(false)))
            .timeout(DISCOVERY_TIMEOUT)
            .build()
            .map_err(|_| DiscoveryError::Unavailable)?;
        let mut response = client
            .get(format!("https://autoconfig.thunderbird.net/v1.1/{domain}"))
            .send()
            .await
            .map_err(|_| DiscoveryError::Unavailable)?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|n| n > MAX_CONFIG_BYTES as u64)
        {
            return Err(DiscoveryError::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| DiscoveryError::Unavailable)?
        {
            if bytes.len() + chunk.len() > MAX_CONFIG_BYTES {
                return Err(DiscoveryError::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn mx(&self, domain: &str) -> Result<Vec<String>, DiscoveryError> {
        let domain = super::normalize_host(domain)?;
        let resolver = hickory_resolver::TokioResolver::builder_tokio()
            .and_then(|b| b.build())
            .map_err(|_| DiscoveryError::Unavailable)?;
        let lookup = resolver
            .mx_lookup(format!("{domain}."))
            .await
            .map_err(|_| DiscoveryError::Unavailable)?;
        Ok(lookup
            .answers()
            .iter()
            .filter_map(|r| match &r.data {
                hickory_resolver::proto::rr::RData::MX(mx) => Some(mx.exchange.to_utf8()),
                _ => None,
            })
            .take(super::mx::MAX_MX_RECORDS * 2)
            .collect())
    }

    async fn probe(&self, host: &str) -> Result<bool, DiscoveryError> {
        let host = super::normalize_host(host)?;
        let addresses: Vec<_> = tokio::net::lookup_host((host.as_str(), 993))
            .await
            .map_err(|_| DiscoveryError::Unavailable)?
            .collect();
        // INVARIANT: Use only this validated DNS snapshot, including connection fallback.
        if addresses.is_empty() || addresses.iter().any(|a| !dns::public_address(a.ip())) {
            return Err(DiscoveryError::Unavailable);
        }
        let stream = TcpStream::connect(addresses.as_slice())
            .await
            .map_err(|_| DiscoveryError::Unavailable)?;
        let endpoint = ImapEndpointConfig {
            account_id: "discovery".into(),
            host: host.clone(),
            port: 993,
            auth_method: AuthMethod::AppPassword,
            allow_loopback_self_signed_tls: false,
        };
        let connector =
            crate::tls::create_tls_connector(&endpoint).map_err(|_| DiscoveryError::Unavailable)?;
        let name = ServerName::try_from(host).map_err(|_| DiscoveryError::InvalidInput)?;
        let stream = connector
            .connect(name, stream)
            .await
            .map_err(|_| DiscoveryError::Unavailable)?;
        // INVARIANT: A TCP listener alone does not prove IMAP. Require verified TLS and a
        // bounded IMAP greeting, without sending credentials or commands to an unknown host.
        let mut greeting = String::new();
        BufReader::new(stream)
            .take(1024)
            .read_line(&mut greeting)
            .await
            .map_err(|_| DiscoveryError::Unavailable)?;
        Ok(greeting.ends_with("\r\n") && greeting.to_ascii_uppercase().starts_with("* OK "))
    }
}
