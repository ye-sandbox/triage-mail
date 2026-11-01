use serde::{Deserialize, Serialize};

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
}
