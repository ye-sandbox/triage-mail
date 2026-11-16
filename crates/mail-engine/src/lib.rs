#![forbid(unsafe_code)]

//! Agnostic IMAP transport (`ImapConnector`), transparent multi-auth, MIME parser,
//! and RFC 8058 One-Click Unsubscribe contracts.
//!
//! INVARIANT: Never fetch full `RFC822` payloads during bulk background sync; fetch only
//! `ENVELOPE`, `FLAGS`, `UID`, `MODSEQ`, and text/HTML body structure to keep RAM and SQLite lean.
//! Binary attachments are fetched exclusively on-demand via `IMAP BODY.PEEK[section]`.
//! INVARIANT: Self-signed TLS certificates are permitted strictly when connecting to loopback
//! (`127.0.0.1` or `::1`) for local Proton Mail Bridge, never for remote IMAP hosts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vault::VaultError;

pub mod auth;
pub mod bodystructure;
pub mod connector;
pub mod discovery;
pub mod idle;
pub mod mbox;
pub mod mime_parser;
pub mod mutations;
pub mod protocol;
pub mod provider;
pub mod sync;
pub mod tls;
pub mod unsubscribe;

pub const DEFAULT_SYNC_UID_BATCH_SIZE: u32 = 300;

pub use mbox::{
    extract_gmail_labels, parse_eml_bytes, parse_eml_reader, LocalArchiveFormat, MboxFlavor,
    MboxReadConfig, MboxReader, ParsedArchiveMessage, DEFAULT_MAX_MBOX_MESSAGE_BYTES,
};

pub use auth::{
    validate_token_endpoint, HttpTokenRefreshTransport, RefreshTokenRequest, RefreshedTokens,
    TokenRefreshTransport, TokenRefresher,
};
pub use bodystructure::{
    extract_body_structure_metadata, extract_bodystructure_sexpr, parse_bodystructure_str,
    BodyPartStructure, ExtractedBodyStructure, MultipartStructure, SinglePartStructure,
    TextSectionInfo,
};
pub use connector::DefaultImapConnector;
pub use idle::{
    AdaptivePollConfig, AdaptivePollSchedule, IdleConfig, IdleCycleOutcome, IdleManager,
    DEFAULT_IDLE_HEARTBEAT_SECS,
};
pub use protocol::{
    decode_modified_utf7, parse_list_line, DiscoveredFolder, FolderStatus, IdleNotification,
    IdleWaitOutcome, ImapSession, RawFetchedMessage, TaggedStatus, UidFlags,
};
pub type NativeSession =
    protocol::ImapSession<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>;
pub use provider::{
    classify_folder, is_gmail_virtual_all_mail, ProviderKind, ServerCapabilities, SpecialFolderRole,
};
pub use unsubscribe::{
    validate_rfc8058_descriptor, validate_rfc8058_uri, Rfc8058ClientConfig,
    Rfc8058UnsubscribeClient, UnsubscribeOutcome, RFC8058_CONTENT_TYPE, RFC8058_POST_BODY,
};

/// Errors returned by IMAP transport, authentication, and MIME extraction.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("vault credential error: {0}")]
    Vault(#[from] VaultError),
    #[error("insecure TLS configuration rejected for non-loopback host `{0}`")]
    InsecureTlsRejected(String),
    #[error("IMAP protocol error: {0}")]
    ImapProtocol(String),
    #[error("IMAP authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("OAuth token refresh failed: {0}")]
    TokenRefresh(String),
    #[error("MIME parsing error: {0}")]
    MimeParse(String),
    #[error("RFC 8058 One-Click Unsubscribe failed: {0}")]
    UnsubscribeFailed(String),
}

/// Transparent authentication mode configured by the user for an email account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Standard IMAP password or provider-issued App Password.
    AppPassword,
    /// Public OAuth2 PKCE (`client_id` only, loopback `127.0.0.1` redirect).
    OAuth2PublicPkce,
    /// User-supplied Bring-Your-Own OAuth2 (`client_id` + `client_secret` stored in `vault`).
    OAuth2ByoCredentials,
    /// Local Proton Mail Bridge on loopback (`127.0.0.1:1143`).
    ProtonBridgeLocal,
    /// Offline local archive account (`.mbox` / `.eml` import, zero IMAP transport).
    LocalArchive,
}

/// Connection endpoint configuration for an IMAP account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImapEndpointConfig {
    pub account_id: String,
    pub host: String,
    pub port: u16,
    pub auth_method: AuthMethod,
    pub allow_loopback_self_signed_tls: bool,
}

impl ImapEndpointConfig {
    /// Returns true if the configured host is strictly a local loopback address.
    pub fn is_loopback_host(&self) -> bool {
        matches!(self.host.trim(), "127.0.0.1" | "::1" | "localhost")
    }

    /// Validates TLS security invariants before establishing a connection.
    pub fn validate_tls_policy(&self) -> Result<(), EngineError> {
        if self.allow_loopback_self_signed_tls && !self.is_loopback_host() {
            return Err(EngineError::InsecureTlsRejected(self.host.clone()));
        }
        Ok(())
    }
}

/// Metadata for a lazy attachment (fetched on demand via `IMAP BODY.PEEK[section]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub part_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub content_id: Option<String>,
    pub is_inline: bool,
}

/// RFC 2369 / RFC 8058 Unsubscribe header metadata extracted from a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeDescriptor {
    pub https_uri: Option<String>,
    pub mailto_uri: Option<String>,
    /// True when `List-Unsubscribe-Post: List-Unsubscribe=One-Click` (RFC 8058) is present
    /// and a valid HTTPS URI is present in `List-Unsubscribe`.
    pub supports_rfc8058_one_click: bool,
    pub raw_list_unsubscribe: Option<String>,
    pub raw_list_unsubscribe_post: Option<String>,
}

/// Lightweight parsed message representation produced during inbound sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedInboundMessage {
    pub uid: u32,
    pub modseq: Option<u64>,
    pub provider_message_id: Option<String>,
    pub message_id_header: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub references_header: Option<String>,
    pub subject: String,
    pub sender: String,
    pub recipients: Vec<String>,
    pub date_epoch_ms: Option<i64>,
    pub text_plain: Option<String>,
    pub html_raw: Option<String>,
    pub raw_headers: Option<String>,
    pub unsubscribe: Option<UnsubscribeDescriptor>,
    pub attachments: Vec<AttachmentMeta>,
}

pub use mime_parser::{build_thread_tree, StalwartMimeParser, ThreadNode};

/// Event emitted by background sync or `IDLE` workers to decoupled consumers (GUI or CLI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncProgressEvent {
    pub account_id: String,
    pub folder: String,
    pub synced_count: usize,
    pub uidvalidity: u32,
    pub highest_modseq: Option<u64>,
}

/// Observer trait decoupling `mail-engine` from `tauri::ipc::Channel` or CLI stdout.
pub trait SyncEventSink: Send + Sync {
    /// Handles a batch synchronization progress notification.
    fn on_sync_progress(&self, event: &SyncProgressEvent) -> Result<(), EngineError>;
}

/// Agnostic IMAP transport contract isolating upstream IMAP crate types (`async-imap` / `imap-next`).
#[async_trait]
pub trait ImapConnector: Send + Sync {
    /// Checks folder status (`UIDVALIDITY`, `UIDNEXT`, `HIGHESTMODSEQ`) and fetches incremental messages.
    async fn sync_incremental(
        &self,
        endpoint: &ImapEndpointConfig,
        folder: &str,
        last_uid: u32,
        last_modseq: Option<u64>,
        sink: &dyn SyncEventSink,
    ) -> Result<Vec<ParsedInboundMessage>, EngineError>;

    /// Fetches a single attachment part on demand via `IMAP BODY.PEEK[section]`.
    async fn fetch_attachment_bytes(
        &self,
        endpoint: &ImapEndpointConfig,
        folder: &str,
        uid: u32,
        part_id: &str,
    ) -> Result<Vec<u8>, EngineError>;
}

/// RFC 5322 / MIME extractor contract.
pub trait MimeExtractor: Send + Sync {
    /// Extracts headers, text/HTML bodies, RFC 8058 unsubscribe metadata, and lazy attachment descriptors.
    fn parse_lightweight(
        &self,
        raw_headers_and_body: &[u8],
    ) -> Result<ParsedInboundMessage, EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_loopback_tls_invariant_for_proton_bridge() {
        let loopback = ImapEndpointConfig {
            account_id: "proton-local".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1143,
            auth_method: AuthMethod::ProtonBridgeLocal,
            allow_loopback_self_signed_tls: true,
        };
        assert_eq!(loopback.validate_tls_policy(), Ok(()));

        let remote_insecure = ImapEndpointConfig {
            account_id: "remote-imap".to_string(),
            host: "imap.example.com".to_string(),
            port: 993,
            auth_method: AuthMethod::AppPassword,
            allow_loopback_self_signed_tls: true,
        };
        assert_eq!(
            remote_insecure.validate_tls_policy(),
            Err(EngineError::InsecureTlsRejected(
                "imap.example.com".to_string()
            ))
        );
    }
}
