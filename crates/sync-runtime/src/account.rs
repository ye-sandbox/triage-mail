//! Headless account connection configuration loaded from `account_connections`.

use mail_engine::{AuthMethod, EngineError, ImapEndpointConfig};
use serde::{Deserialize, Serialize};
use vault::VaultKey;

/// Headless account connection configuration loaded from storage (`account_connections`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountConnectionConfig {
    pub account_id: String,
    pub email_display: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub auth_method: AuthMethod,
}

impl AccountConnectionConfig {
    /// Returns `true` when this account represents an offline local `.mbox`/`.eml` archive.
    #[must_use]
    pub fn is_local_archive(&self) -> bool {
        self.auth_method == AuthMethod::LocalArchive
    }

    /// Converts the configuration into an engine endpoint and its corresponding `VaultKey`.
    pub fn into_endpoint_and_vault_key(
        self,
    ) -> Result<(ImapEndpointConfig, VaultKey), EngineError> {
        let vault_key = VaultKey::for_account(&self.account_id, "imap-auth")
            .map_err(|e| EngineError::ImapProtocol(e.to_string()))?;
        let is_proton = self.auth_method == AuthMethod::ProtonBridgeLocal;
        let endpoint = ImapEndpointConfig {
            account_id: self.account_id,
            host: self.imap_host,
            port: self.imap_port,
            auth_method: self.auth_method,
            allow_loopback_self_signed_tls: is_proton,
        };
        endpoint.validate_tls_policy()?;
        Ok((endpoint, vault_key))
    }
}
