#![deny(unsafe_code)]

//! Single-file SQLite WAL + Full-DB SQLCipher + Configurable FTS5 + `sqlite-vec` KNN contracts.
//!
//! INVARIANT: Use full-database SQLCipher encryption exclusively; never encrypt individual
//! text columns with AEAD, as doing so prevents SQLite `FTS5` from indexing message bodies.
//! INVARIANT: `unsafe_code` is denied across this crate and may only be allowed in the single
//! isolated FFI block that registers `sqlite-vec` via `sqlite3_auto_extension`.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use vault::VaultError;

pub mod alerts;
pub mod avatars;
pub mod backup;
pub mod categories;
pub mod connection;
pub mod conversations;
pub mod desktop;
pub mod migrations;
pub mod models;
pub mod portable;
pub mod reminders;
pub mod repository;
pub mod rules;
pub mod saved_views;
mod scoped_search;
pub mod search;
pub mod sender_review;
pub mod settings;
pub mod vec_ext;

pub use avatars::AccountAvatar;
pub use connection::DatabaseKey;
pub use models::{
    now_epoch_ms, AccountRecord, AttachmentRecord, FolderRecord, MessageFlagsDelta,
    MessageFolderMembershipRecord, MessageRecord,
};
pub use reminders::ReminderJobRecord;
pub use repository::SqliteMailStore;
pub use scoped_search::{MailboxScope, MailboxView};

#[cfg(test)]
mod scoped_search_tests;

/// Errors returned by SQLCipher storage, migrations, and hybrid search operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("vault error while provisioning database key: {0}")]
    Vault(#[from] VaultError),
    #[error("database migration failed: {0}")]
    Migration(String),
    #[error("query execution failed: {0}")]
    Query(String),
    #[error("invalid search query parameters: {0}")]
    InvalidSearchQuery(String),
}

/// Configurable SQLite FTS5 tokenizer mode.
/// Defaults to `Unicode61RemoveDiacritics` (`unicode61 remove_diacritics 2`) for lean index size
/// and fast prefix/diacritic-insensitive matching across Portuguese and English emails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FtsTokenizerMode {
    #[default]
    Unicode61RemoveDiacritics,
    Trigram,
}

/// Incremental IMAP synchronization checkpoint stored per folder.
///
/// INVARIANT: Persisting `uidvalidity`, `uidnext`, and `highest_modseq` (RFC 7162 CONDSTORE)
/// prevents rescanning all message flags upon reconnect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderSyncState {
    pub account_id: String,
    pub folder_name: String,
    pub uidvalidity: u32,
    pub uidnext: u32,
    pub highest_modseq: Option<u64>,
}

/// Search execution strategy combining FTS5 lexical match and `sqlite-vec` (`int8` quantized) KNN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    Lexical,
    Semantic,
    Hybrid,
}

/// Parameters for sub-15ms hybrid search queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridQuery {
    pub query_text: String,
    pub query_embedding_int8: Option<Vec<i8>>,
    pub account_filter: Option<String>,
    pub mode: SearchMode,
    pub limit: usize,
    pub offset: usize,
}

impl HybridQuery {
    /// Validates query bounds before execution.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.limit == 0 || self.limit > 500 {
            return Err(StorageError::InvalidSearchQuery(
                "limit must be between 1 and 500".to_string(),
            ));
        }
        if self
            .offset
            .checked_add(self.limit)
            .is_none_or(|end| i64::try_from(end).is_err())
        {
            return Err(StorageError::InvalidSearchQuery(
                "pagination exceeds SQLite integer bounds".to_string(),
            ));
        }
        if self.mode == SearchMode::Semantic && self.query_embedding_int8.is_none() {
            return Err(StorageError::InvalidSearchQuery(
                "semantic search requires an int8 query embedding".to_string(),
            ));
        }
        Ok(())
    }
}

/// Lightweight message row returned for virtualized 60fps inbox rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRowSummary {
    pub id: String,
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub subject: String,
    pub sender: String,
    pub received_at_epoch_ms: i64,
    pub is_read: bool,
    pub has_attachments: bool,
    pub category_badge: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Display-only local move intent; folder/UID remain canonical server identity.
    #[serde(default)]
    pub folder_override: Option<String>,
    #[serde(default)]
    pub triage_badges: Vec<triage_ai::TriageBadge>,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default)]
    pub snooze_until_epoch_ms: Option<i64>,
}

/// Schema migration runner contract.
pub trait MigrationRunner: Send + Sync {
    /// Applies pending WAL and SQLCipher schema migrations idempotently.
    fn run_migrations(&self) -> Result<u32, StorageError>;
}

/// Repository contract for accounts, folder sync checkpoints, and message persistence.
pub trait MailRepository: Send + Sync {
    /// Retrieves the incremental IMAP sync checkpoint for a folder.
    fn get_folder_sync_state(
        &self,
        account_id: &str,
        folder_name: &str,
    ) -> Result<Option<FolderSyncState>, StorageError>;

    /// Updates the incremental IMAP sync checkpoint for a folder.
    fn upsert_folder_sync_state(&self, state: &FolderSyncState) -> Result<(), StorageError>;
}

/// Hybrid search engine contract (`FTS5` + `sqlite-vec`).
pub trait SearchEngine: Send + Sync {
    /// Executes a lexical, semantic, or hybrid search query targeting `< 15ms` latency.
    fn search(&self, query: &HybridQuery) -> Result<Vec<MessageRowSummary>, StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fts_tokenizer_to_unicode61_remove_diacritics() {
        assert_eq!(
            FtsTokenizerMode::default(),
            FtsTokenizerMode::Unicode61RemoveDiacritics
        );
    }

    #[test]
    fn validates_hybrid_query_constraints() {
        let valid = HybridQuery {
            query_text: "boleto".to_string(),
            query_embedding_int8: None,
            account_filter: None,
            mode: SearchMode::Lexical,
            limit: 50,
            offset: 0,
        };
        assert_eq!(valid.validate(), Ok(()));

        let invalid_limit = HybridQuery {
            limit: 0,
            ..valid.clone()
        };
        assert!(matches!(
            invalid_limit.validate(),
            Err(StorageError::InvalidSearchQuery(_))
        ));

        let missing_embedding = HybridQuery {
            mode: SearchMode::Semantic,
            query_embedding_int8: None,
            ..valid
        };
        assert!(matches!(
            missing_embedding.validate(),
            Err(StorageError::InvalidSearchQuery(_))
        ));
    }
}

#[cfg(test)]
mod partition_tests;

pub mod embeddings;
#[cfg(test)]
mod embeddings_tests;
