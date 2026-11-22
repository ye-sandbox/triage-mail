//! Tauri v2 IPC bridge and typed DTO adapters connecting `apps/desktop` to core Rust crates.
//!
//! INVARIANT: Raw secrets, passwords, or OAuth tokens are never returned to the UI.
//! All IPC DTOs expose only opaque `account_id` identifiers and sanitized metadata.

pub use mail_engine::discovery::DiscoveredProvider as DiscoveredProviderDto;
use mail_engine::{AttachmentMeta, AuthMethod, ImapEndpointConfig};
use serde::{Deserialize, Serialize};
use storage::{HybridQuery, MessageRowSummary, SearchMode};
pub use storage::{MailboxScope as MailboxScopeDto, MailboxView as MailboxViewDto};
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

/// Read-only status of local vault/database files and OS keyring availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultStatusDto {
    pub exists: bool,
    pub keyring_available: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<MailboxScopeDto>,
    /// Restricts an account-scoped search to messages delivered to this alias address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
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
    Star,
    Unstar,
    Snooze { until_epoch_ms: i64 },
    Unsnooze,
}

/// Full message detail DTO returned by `cmd_get_message_body`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDetailDto {
    pub id: String,
    pub subject: String,
    pub sender: String,
    #[serde(default)]
    pub recipients: String,
    pub headers: String,
    pub text_plain: Option<String>,
    pub html_raw: Option<String>,
    pub triage_result: DeterministicTriageResult,
    pub triage_badges: Vec<TriageBadge>,
    pub attachments_meta: Vec<AttachmentMeta>,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default)]
    pub snooze_until_epoch_ms: Option<i64>,
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
        tags: Vec::new(),
        folder_override: None,
        triage_badges: Vec::new(),
        is_starred: false,
        snooze_until_epoch_ms: None,
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
    fn mailbox_scope_is_optional_and_uses_existing_enum_encoding() {
        let legacy =
            serde_json::json!({ "query": "", "mode": "Lexical", "limit": 10, "offset": 0 });
        let request: SearchQueryDto = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(request.scope, None);
        assert_eq!(serde_json::to_value(&request).unwrap(), legacy);
        for scope in [
            MailboxScopeDto::GlobalAllMail,
            MailboxScopeDto::AccountView {
                account_id: "synthetic".into(),
                view: MailboxViewDto::Folder("Archive".into()),
            },
        ] {
            let mut value = legacy.clone();
            value["scope"] = serde_json::to_value(&scope).unwrap();
            let request: SearchQueryDto = serde_json::from_value(value).unwrap();
            assert_eq!(request.scope, Some(scope));
        }
        assert!(serde_json::from_value::<SearchQueryDto>(serde_json::json!({
            "query": "", "mode": "Lexical", "limit": 10, "offset": 0, "scope": "unknown"
        }))
        .is_err());
    }

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

/// Opaque storage message identifier.
pub type MessageId = String;
pub type MessageRowDto = MessageRowSummary;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageMutationDto {
    pub message_ids: Vec<MessageId>,
    pub action: TriageActionDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRequestDto {
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub part_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentBytesDto {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentSavedDto {
    pub bytes_written: u64,
}

/// Mailbox folder metadata returned to the UI (`cmd_list_folders`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderDto {
    pub account_id: String,
    pub folder_name: String,
    pub display_name: String,
    pub delimiter: Option<String>,
    pub role: String,
    pub is_selectable: bool,
    pub total_messages: Option<u32>,
    pub unread_messages: Option<u32>,
}

impl From<storage::FolderRecord> for FolderDto {
    fn from(r: storage::FolderRecord) -> Self {
        Self {
            account_id: r.account_id,
            folder_name: r.folder_name,
            display_name: r.display_name,
            delimiter: r.delimiter,
            role: r.role,
            is_selectable: r.is_selectable,
            total_messages: r.total_messages,
            unread_messages: r.unread_messages,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    Syncing,
    Idle,
    Retrying,
    MutationPending,
    MutationFailed,
    ResyncRequired,
}

pub use mail_engine::ConnectionDiagnosticCode;

/// Result of a transient, zero-persistence pre-save connection and authentication check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionTestResultDto {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<ConnectionDiagnosticCode>,
}

/// Desktop wire event, deliberately separate from mail_engine::SyncProgressEvent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncProgressEvent {
    pub account_id: String,
    pub state: SyncState,
    pub new_messages_count: usize,
    pub triage_alerts: Vec<TriageBadge>,
    #[serde(default)]
    pub embedding_errors_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<ConnectionDiagnosticCode>,
}

/// Summary returned after streaming a local `.mbox` or `.eml` archive into storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalArchiveImportDto {
    pub account_id: String,
    pub imported_count: usize,
    pub batches_committed: usize,
    pub last_uid: u32,
}

pub use triage_ai::{
    DueDateItem as DueDateItemDto, DueDateStatus as DueDateStatusDto,
    ReminderTrigger as ReminderTriggerDto, ScheduledReminderConfig as ScheduledReminderConfigDto,
};
