use rusqlite::Connection;

use crate::{FtsTokenizerMode, StorageError};

/// Current database schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Executes database schema migrations idempotently.
pub struct SchemaMigrator {
    tokenizer_mode: FtsTokenizerMode,
}

impl SchemaMigrator {
    /// Creates a migrator with the specified FTS5 tokenizer configuration.
    pub fn new(tokenizer_mode: FtsTokenizerMode) -> Self {
        Self { tokenizer_mode }
    }

    /// Runs all pending schema migrations inside a transaction.
    pub fn migrate(&self, conn: &mut Connection) -> Result<u32, StorageError> {
        let tx = conn
            .transaction()
            .map_err(|e| StorageError::Migration(format!("failed to start migration tx: {e}")))?;

        // Ensure schema_migrations table exists
        tx.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                applied_at_epoch_ms INTEGER NOT NULL
            );",
            [],
        )
        .map_err(|e| StorageError::Migration(format!("failed to create migrations table: {e}")))?;

        let current_version: u32 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations;",
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                StorageError::Migration(format!("failed to query current schema version: {e}"))
            })?;

        let mut applied_count = 0;

        if current_version < 1 {
            self.apply_v1(&tx)?;
            applied_count += 1;
        }

        tx.commit()
            .map_err(|e| StorageError::Migration(format!("failed to commit migration tx: {e}")))?;

        Ok(applied_count)
    }

    fn record_migration(
        &self,
        tx: &rusqlite::Transaction<'_>,
        version: u32,
        name: &str,
    ) -> Result<(), StorageError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at_epoch_ms) VALUES (?1, ?2, ?3);",
            rusqlite::params![version, name, now_ms],
        )
        .map_err(|e| StorageError::Migration(format!("failed to record migration v{version}: {e}")))?;

        Ok(())
    }

    fn apply_v1(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        // INVARIANT: Pre-release v0.1.0 schema consolidation (ADR-007). All tables, FTS5 indices,
        // and single account-partitioned vec0 table initialize atomically in v1.
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY NOT NULL,
                email TEXT NOT NULL UNIQUE,
                auth_type TEXT NOT NULL,
                created_at_epoch_ms INTEGER NOT NULL,
                updated_at_epoch_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS folder_sync_state (
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                folder_name TEXT NOT NULL,
                uidvalidity INTEGER NOT NULL,
                uidnext INTEGER NOT NULL,
                highest_modseq INTEGER,
                updated_at_epoch_ms INTEGER NOT NULL,
                PRIMARY KEY (account_id, folder_name)
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY NOT NULL,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                folder TEXT NOT NULL,
                uid INTEGER NOT NULL,
                message_id_header TEXT,
                in_reply_to TEXT,
                references_header TEXT,
                subject TEXT NOT NULL,
                sender TEXT NOT NULL,
                recipients TEXT NOT NULL,
                received_at_epoch_ms INTEGER NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                has_attachments INTEGER NOT NULL DEFAULT 0,
                category_badge TEXT,
                triage_badges TEXT NOT NULL DEFAULT '[]',
                raw_headers TEXT NOT NULL,
                text_body TEXT,
                html_body TEXT,
                list_unsubscribe TEXT,
                list_unsubscribe_post TEXT,
                one_click_unsubscribe_supported INTEGER NOT NULL DEFAULT 0,
                created_at_epoch_ms INTEGER NOT NULL,
                UNIQUE(account_id, folder, uid)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_account_folder_received 
            ON messages(account_id, folder, received_at_epoch_ms DESC);

            CREATE INDEX IF NOT EXISTS idx_messages_received 
            ON messages(received_at_epoch_ms DESC);

            CREATE TABLE IF NOT EXISTS attachments (
                id TEXT PRIMARY KEY NOT NULL,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                part_id TEXT NOT NULL,
                filename TEXT NOT NULL,
                content_type TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                content_id TEXT,
                is_inline INTEGER NOT NULL DEFAULT 0,
                UNIQUE(message_id, part_id)
            );

            CREATE INDEX IF NOT EXISTS idx_attachments_message_id 
            ON attachments(message_id);

            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY NOT NULL,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                tag_name TEXT NOT NULL,
                created_at_epoch_ms INTEGER NOT NULL,
                UNIQUE(message_id, tag_name)
            );

            CREATE INDEX IF NOT EXISTS idx_tags_tag_name ON tags(tag_name);

            CREATE TABLE IF NOT EXISTS account_connections (
                account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
                config_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS pending_mutations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS message_folder_overrides (
                message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
                folder TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to execute v1 core tables: {e}")))?;

        let tokenizer_clause = match self.tokenizer_mode {
            FtsTokenizerMode::Unicode61RemoveDiacritics => {
                "tokenize='unicode61 remove_diacritics 2'"
            }
            FtsTokenizerMode::Trigram => "tokenize='trigram'",
        };

        let create_fts_sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                subject,
                sender,
                text_body,
                content='messages',
                content_rowid='rowid',
                {tokenizer_clause}
            );"
        );

        tx.execute(&create_fts_sql, [])
            .map_err(|e| StorageError::Migration(format!("failed to create messages_fts: {e}")))?;

        tx.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(rowid, subject, sender, text_body)
                VALUES (new.rowid, new.subject, new.sender, coalesce(new.text_body, ''));
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, subject, sender, text_body)
                VALUES ('delete', old.rowid, old.subject, old.sender, coalesce(old.text_body, ''));
            END;

            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, subject, sender, text_body)
                VALUES ('delete', old.rowid, old.subject, old.sender, coalesce(old.text_body, ''));
                INSERT INTO messages_fts(rowid, subject, sender, text_body)
                VALUES (new.rowid, new.subject, new.sender, coalesce(new.text_body, ''));
            END;
            ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to create FTS triggers: {e}")))?;

        // INVARIANT: Single account-partitioned vector index for sub-15ms filtered KNN.
        // Unfiltered KNN queries across all partitions; no duplicate vector table or reconciliation triggers.
        tx.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_messages USING vec0(
                message_id TEXT PRIMARY KEY,
                account_id TEXT PARTITION KEY,
                embedding_int8 int8[384] distance_metric=cosine
            );",
            [],
        )
        .map_err(|e| {
            StorageError::Migration(format!("failed to create vec_messages table: {e}"))
        })?;

        self.record_migration(tx, 1, "001_canonical_schema")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{open_in_memory_connection, DatabaseKey};

    #[test]
    fn executes_migrations_idempotently() {
        let key = DatabaseKey::from_bytes([77u8; 32]);
        let mut conn = open_in_memory_connection(&key).expect("open conn");

        let migrator = SchemaMigrator::new(FtsTokenizerMode::Unicode61RemoveDiacritics);
        let applied_first = migrator.migrate(&mut conn).expect("first migration");
        assert_eq!(applied_first, 1);

        let applied_second = migrator.migrate(&mut conn).expect("second migration");
        assert_eq!(applied_second, 0, "subsequent migration must be a no-op");

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"accounts".to_string()));
        assert!(tables.contains(&"folder_sync_state".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"attachments".to_string()));
        assert!(tables.contains(&"tags".to_string()));
        assert!(tables.contains(&"account_connections".to_string()));
        assert!(tables.contains(&"pending_mutations".to_string()));
        assert!(tables.contains(&"message_folder_overrides".to_string()));
        assert!(tables.contains(&"messages_fts".to_string()));
        assert!(tables.contains(&"vec_messages".to_string()));
        assert!(!tables.contains(&"vec_messages_by_account".to_string()));
    }
}
