// INVARIANT: Agnostic IMAP transport isolating protocol mechanics behind ImapConnector trait.
// INVARIANT: Never fetch full RFC822 payloads during bulk sync; fetch headers, flags,
// and body structure only. Binary attachments are fetched on demand via BODY.PEEK[part].

use async_trait::async_trait;
use rustls_pki_types::ServerName;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use vault::{AccountAuthMaterial, SecretStore, VaultKey};

use crate::auth::{HttpTokenRefreshTransport, TokenRefreshTransport, TokenRefresher};
use crate::protocol::ImapSession;
use crate::provider::ServerCapabilities;
use crate::sync::{execute_incremental_sync, IncrementalSyncQuery};
use crate::tls::create_tls_connector;
use crate::{EngineError, ImapConnector, ImapEndpointConfig, ParsedInboundMessage, SyncEventSink};

/// Default production IMAP connector backed by `tokio-rustls` and `crates/vault`.
pub struct DefaultImapConnector {
    vault: Arc<dyn SecretStore>,
    refresher: Arc<dyn TokenRefreshTransport>,
}

impl DefaultImapConnector {
    /// Creates a new connector with access to encrypted secret storage and default HTTPS refresh transport.
    pub fn new(vault: Arc<dyn SecretStore>) -> Self {
        Self::with_refresher(vault, Arc::new(HttpTokenRefreshTransport))
    }

    /// Creates a new connector with a custom token refresh transport (useful for testing).
    pub fn with_refresher(
        vault: Arc<dyn SecretStore>,
        refresher: Arc<dyn TokenRefreshTransport>,
    ) -> Self {
        Self { vault, refresher }
    }

    /// Retrieves stored authentication material for an account from the vault.
    pub fn load_auth_material(&self, account_id: &str) -> Result<AccountAuthMaterial, EngineError> {
        let key = VaultKey::for_account(account_id, "imap-auth")?;
        let bytes = self.vault.get_secret(&key)?;
        AccountAuthMaterial::from_bytes(&bytes).map_err(EngineError::Vault)
    }

    /// Establishes TLS 1.3 connection and authenticates an IMAP session.
    pub async fn connect_and_authenticate(
        &self,
        endpoint: &ImapEndpointConfig,
        auth: &AccountAuthMaterial,
    ) -> Result<(ImapSession<TlsStream<TcpStream>>, ServerCapabilities), EngineError> {
        self.connect_and_authenticate_as(endpoint, &endpoint.account_id, auth)
            .await
    }

    async fn open_raw_session(
        &self,
        endpoint: &ImapEndpointConfig,
    ) -> Result<(ImapSession<TlsStream<TcpStream>>, ServerCapabilities), EngineError> {
        let connector = create_tls_connector(endpoint)?;
        let addr = format!("{}:{}", endpoint.host, endpoint.port);
        let tcp_stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
            .await
            .map_err(|e| {
                EngineError::ImapProtocol(format!("TCP connect to `{addr}` failed: {e}"))
            })?;

        let server_name = ServerName::try_from(endpoint.host.as_str())
            .map_err(|e| {
                EngineError::ImapProtocol(format!("invalid server name `{}`: {e}", endpoint.host))
            })?
            .to_owned();

        let tls_stream = connector
            .connect(server_name, tcp_stream)
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("TLS handshake failed: {e}")))?;

        let mut session = ImapSession::new(tls_stream);

        // Read server greeting banner
        let _greeting = session.read_greeting().await?;
        let caps = session.capability().await.unwrap_or_default();

        Ok((session, caps))
    }

    async fn try_authenticate(
        session: &mut ImapSession<TlsStream<TcpStream>>,
        username: &str,
        auth: &AccountAuthMaterial,
    ) -> Result<(), EngineError> {
        match auth {
            AccountAuthMaterial::AppPassword { secret } => session.login(username, secret).await,
            AccountAuthMaterial::OAuth2PublicPkce { access_token, .. } => {
                session.authenticate_xoauth2(username, access_token).await
            }
            AccountAuthMaterial::OAuth2ByoCredentials { access_token, .. } => {
                session.authenticate_xoauth2(username, access_token).await
            }
            AccountAuthMaterial::ProtonBridgeLocal { bridge_password } => {
                session.login(username, bridge_password).await
            }
        }
    }

    /// Authenticates with a login independent of the opaque account identifier.
    /// Transparently refreshes expired OAuth2 access tokens and retries once if SASL authentication fails.
    pub async fn connect_and_authenticate_as(
        &self,
        endpoint: &ImapEndpointConfig,
        username: &str,
        auth: &AccountAuthMaterial,
    ) -> Result<(ImapSession<TlsStream<TcpStream>>, ServerCapabilities), EngineError> {
        // INVARIANT: Account IDs select vault entries; only the explicit username is sent as login.
        if username.is_empty() || username.chars().any(char::is_control) {
            return Err(EngineError::ImapProtocol("invalid IMAP username".into()));
        }
        endpoint.validate_tls_policy()?;

        let (mut session, mut caps) = self.open_raw_session(endpoint).await?;
        let auth_res = Self::try_authenticate(&mut session, username, auth).await;

        match auth_res {
            Ok(()) => {}
            Err(EngineError::AuthenticationFailed(ref _reason))
                if auth.token_endpoint().is_some() && auth.refresh_token().is_some() =>
            {
                // INVARIANT: When SASL authentication fails on an OAuth account with a refresh token,
                // refresh the access token against the configured token endpoint, update the vault atomically,
                // and reconnect a fresh session to retry authentication once.
                let refresher = TokenRefresher::new(self.refresher.clone());
                let refreshed_auth = refresher
                    .refresh_and_update_vault(self.vault.as_ref(), &endpoint.account_id, auth)
                    .await?;

                let (mut fresh_session, fresh_caps) = self.open_raw_session(endpoint).await?;
                Self::try_authenticate(&mut fresh_session, username, &refreshed_auth).await?;
                session = fresh_session;
                caps = fresh_caps;
            }
            Err(e) => return Err(e),
        }

        // Re-read capabilities post-authentication
        if let Ok(post_auth_caps) = session.capability().await {
            caps = post_auth_caps;
        }

        Ok((session, caps))
    }
}

#[async_trait]
impl ImapConnector for DefaultImapConnector {
    async fn sync_incremental(
        &self,
        endpoint: &ImapEndpointConfig,
        folder: &str,
        last_uid: u32,
        last_modseq: Option<u64>,
        sink: &dyn SyncEventSink,
    ) -> Result<Vec<ParsedInboundMessage>, EngineError> {
        let auth = self.load_auth_material(&endpoint.account_id)?;
        let (mut session, caps) = self.connect_and_authenticate(endpoint, &auth).await?;

        let mut all_messages = Vec::new();
        let mut current_last_uid = last_uid;
        let mut current_modseq = last_modseq;

        loop {
            let query = IncrementalSyncQuery {
                account_id: &endpoint.account_id,
                folder,
                local_uidvalidity: 0, // caller or storage layer handles persistence
                local_last_uid: current_last_uid,
                local_last_modseq: current_modseq,
            };

            let res = execute_incremental_sync(&mut session, query, &caps, sink).await?;
            let batch_len = res.new_messages.len();
            if res.uidvalidity_reset {
                return Ok(res.new_messages);
            }
            all_messages.extend(res.new_messages);
            current_last_uid = res.new_last_uid;
            current_modseq = res.highest_modseq;

            if (batch_len as u32) < crate::DEFAULT_SYNC_UID_BATCH_SIZE {
                break;
            }
        }

        Ok(all_messages)
    }

    async fn fetch_attachment_bytes(
        &self,
        endpoint: &ImapEndpointConfig,
        folder: &str,
        uid: u32,
        part_id: &str,
    ) -> Result<Vec<u8>, EngineError> {
        let auth = self.load_auth_material(&endpoint.account_id)?;
        let (mut session, _caps) = self.connect_and_authenticate(endpoint, &auth).await?;

        let _ = session.select_folder(folder).await?;
        session.fetch_attachment_bytes(uid, part_id).await
    }
}
