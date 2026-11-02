use serde::{Deserialize, Serialize};

/// Returns the current epoch timestamp in milliseconds.
pub fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Account configuration record persisted in SQLite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    pub email: String,
    pub auth_type: String,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

/// Attachment metadata record.
///
/// INVARIANT: Binary attachment content is NEVER stored eagerly in SQLite.
/// Only metadata (MIME type, size, part_id, filename) is saved locally;
/// raw bytes are retrieved on demand via `IMAP BODY.PEEK`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRecord {
    pub id: String,
    pub message_id: String,
    pub part_id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub content_id: Option<String>,
    pub is_inline: bool,
}

/// Account folder record persisted in SQLite (Migration v2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderRecord {
    pub account_id: String,
    pub folder_name: String,
    pub display_name: String,
    pub delimiter: Option<String>,
    pub role: String,
    pub is_selectable: bool,
    pub total_messages: Option<u32>,
    pub unread_messages: Option<u32>,
    pub updated_at_epoch_ms: i64,
}

/// Mapping between an account/folder/UID and canonical message identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFolderMembershipRecord {
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub message_id: String,
    pub created_at_epoch_ms: i64,
}

/// Flag delta for reconciling read and starred flags for a message UID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFlagsDelta {
    pub uid: u32,
    pub is_read: bool,
    pub is_starred: bool,
}

/// Inbound email message record persisted in SQLite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub message_id_header: Option<String>,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    pub subject: String,
    pub sender: String,
    pub recipients: String,
    pub received_at_epoch_ms: i64,
    pub is_read: bool,
    pub has_attachments: bool,
    pub category_badge: Option<String>,
    #[serde(default)]
    pub triage_badges: Vec<triage_ai::TriageBadge>,
    pub raw_headers: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub list_unsubscribe: Option<String>,
    pub list_unsubscribe_post: Option<String>,
    pub one_click_unsubscribe_supported: bool,
    pub created_at_epoch_ms: i64,
    pub attachments: Vec<AttachmentRecord>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_starred: bool,
    pub snooze_until_epoch_ms: Option<i64>,
    pub provider_message_id: Option<String>,
}
