#![forbid(unsafe_code)]

//! Tauri v2 IPC bridge and typed DTO adapters connecting `apps/desktop` to core Rust crates.
//!
//! INVARIANT: Raw secrets, passwords, or OAuth tokens are never returned to the UI.
//! All IPC DTOs expose only opaque `account_id` identifiers and sanitized metadata.

use mail_engine::{AttachmentMeta, AuthMethod, ImapEndpointConfig};
use serde::{Deserialize, Serialize};
use storage::{HybridQuery, MessageRowSummary, SearchMode};
use triage_ai::{DeterministicTriageResult, TriageBadge, TriageCategory};
use vault::VaultKey;

/// Account configuration DTO sent from the UI when registering or listing an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountConfigDto {
    pub account_id: String,
    pub email_display: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub auth_method: AuthMethod,
}

impl AccountConfigDto {
    /// Converts the public DTO into an engine endpoint config and its corresponding `VaultKey`.
    pub fn into_endpoint_and_vault_key(self) -> Result<(ImapEndpointConfig, VaultKey), String> {
        let vault_key =
            VaultKey::for_account(&self.account_id, "imap-auth").map_err(|e| e.to_string())?;
        let is_proton = self.auth_method == AuthMethod::ProtonBridgeLocal;
        let endpoint = ImapEndpointConfig {
            account_id: self.account_id,
            host: self.imap_host,
            port: self.imap_port,
            auth_method: self.auth_method,
            allow_loopback_self_signed_tls: is_proton,
        };
        endpoint.validate_tls_policy().map_err(|e| e.to_string())?;
        Ok((endpoint, vault_key))
    }
}

/// Search request DTO received from the Svelte 5 UI (`cmd_search_messages`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchQueryDto {
    pub query: String,
    pub mode: SearchMode,
    pub limit: usize,
    pub offset: usize,
}

impl SearchQueryDto {
    /// Converts the IPC search request into a validated storage `HybridQuery`.
    pub fn into_hybrid_query(
        self,
        query_embedding_int8: Option<Vec<i8>>,
    ) -> Result<HybridQuery, String> {
        let hq = HybridQuery {
            query_text: self.query,
            query_embedding_int8,
            account_filter: None,
            mode: self.mode,
            limit: self.limit,
            offset: self.offset,
        };
        hq.validate().map_err(|e| e.to_string())?;
        Ok(hq)
    }
}

/// Triage mutation action invoked from the keyboard Command Palette (`cmd_triage_action`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriageActionDto {
    MarkRead,
    MarkUnread,
    Archive,
    Trash,
    Tag(String),
    OneClickUnsubscribe,
}

/// Full message detail DTO returned by `cmd_get_message_body`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDetailDto {
    pub id: String,
    pub subject: String,
    pub sender: String,
    pub text_plain: Option<String>,
    pub html_raw: Option<String>,
    pub triage_result: DeterministicTriageResult,
    pub attachments_meta: Vec<AttachmentMeta>,
}

/// Returns a deterministic baseline health summary of all linked core workspace crates.
pub fn workspace_bridge_health() -> (&'static str, MessageRowSummary, TriageBadge) {
    let sample_row = MessageRowSummary {
        id: "msg-001".to_string(),
        account_id: "acct-demo".to_string(),
        folder: "INBOX".to_string(),
        uid: 1,
        subject: "Welcome to triage-mail".to_string(),
        sender: "noreply@example.com".to_string(),
        received_at_epoch_ms: 1_730_000_000_000,
        is_read: false,
        has_attachments: false,
        category_badge: Some(format!("{:?}", TriageCategory::Personal)),
    };
    let sample_badge = TriageBadge::UnsubscribeAvailable {
        supports_one_click: true,
    };
    ("ok", sample_row, sample_badge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_account_dto_and_rejects_remote_proton_self_signed() {
        let valid_proton = AccountConfigDto {
            account_id: "proton-1".to_string(),
            email_display: "user@example.com".to_string(),
            imap_host: "127.0.0.1".to_string(),
            imap_port: 1143,
            auth_method: AuthMethod::ProtonBridgeLocal,
        };
        assert!(valid_proton.into_endpoint_and_vault_key().is_ok());

        let invalid_remote_proton = AccountConfigDto {
            account_id: "proton-remote".to_string(),
            email_display: "user@example.com".to_string(),
            imap_host: "mail.example.com".to_string(),
            imap_port: 1143,
            auth_method: AuthMethod::ProtonBridgeLocal,
        };
        assert!(invalid_remote_proton.into_endpoint_and_vault_key().is_err());
    }

    #[test]
    fn verifies_workspace_bridge_links_all_core_crates() {
        let (status, row, badge) = workspace_bridge_health();
        assert_eq!(status, "ok");
        assert_eq!(row.sender, "noreply@example.com");
        assert_eq!(
            badge,
            TriageBadge::UnsubscribeAvailable {
                supports_one_click: true
            }
        );
    }
}
