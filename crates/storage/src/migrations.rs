use rusqlite::Connection;

use crate::{FtsTokenizerMode, StorageError};

/// Current database schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 7;

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

        if current_version < 2 {
            self.apply_v2(&tx)?;
            applied_count += 1;
        }

        if current_version < 3 {
            self.apply_v3(&tx)?;
            applied_count += 1;
        }

        if current_version < 4 {
            self.apply_v4(&tx)?;
            applied_count += 1;
        }

        if current_version < 5 {
            tx.execute_batch("CREATE TABLE saved_views (id TEXT PRIMARY KEY NOT NULL, revision INTEGER NOT NULL, definition_json TEXT NOT NULL);
                CREATE TABLE saved_view_deleted_ids (id TEXT PRIMARY KEY NOT NULL);")
                .map_err(|_| StorageError::Migration("saved views migration failed".into()))?;
            self.record_migration(&tx, 5, "005_saved_views")?;
            applied_count += 1;
        }

        if current_version < 6 {
            self.apply_v6(&tx)?;
            applied_count += 1;
        }

        if current_version < 7 {
            tx.execute_batch(include_str!("categories/schema.sql"))
                .map_err(|_| StorageError::Migration("category migration failed".into()))?;
            self.record_migration(&tx, 7, "007_local_categories")?;
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

    fn apply_v4(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        tx.execute_batch(include_str!("alerts/schema.sql"))
            .map_err(|_| StorageError::Migration("failed to execute v4".into()))?;
        self.record_migration(tx, 4, "004_alert_deliveries")
    }

    fn apply_v6(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS reminder_configs (
                id TEXT PRIMARY KEY NOT NULL,
                revision INTEGER NOT NULL,
                definition_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS reminder_config_deleted_ids (
                id TEXT PRIMARY KEY NOT NULL
            );
            CREATE TABLE IF NOT EXISTS reminder_jobs (
                id TEXT PRIMARY KEY NOT NULL,
                config_id TEXT,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                trigger_type TEXT NOT NULL,
                due_epoch_ms INTEGER NOT NULL,
                destination_ids TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                dispatched_at_epoch_ms INTEGER,
                created_at_epoch_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_reminder_jobs_pending ON reminder_jobs(status, due_epoch_ms);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_jobs_dedup ON reminder_jobs(message_id, trigger_type, due_epoch_ms);
            CREATE TABLE IF NOT EXISTS due_date_dismissals (
                message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
                dismissed_at_epoch_ms INTEGER NOT NULL
            );
            ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to execute v6: {e}")))?;
        self.record_migration(tx, 6, "006_scheduled_reminders")
    }

    fn apply_v3(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        // INVARIANT: Rules and queue state remain inside the same encrypted database.
        tx.execute_batch(
            "
            CREATE TABLE local_rules (
                id TEXT PRIMARY KEY NOT NULL, revision INTEGER NOT NULL,
                position INTEGER NOT NULL, definition_json TEXT NOT NULL
            );
            CREATE TABLE rule_deleted_ids (id TEXT PRIMARY KEY NOT NULL);
            CREATE TABLE rule_baselines (
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                folder TEXT NOT NULL, uidvalidity INTEGER NOT NULL, frontier INTEGER NOT NULL,
                PRIMARY KEY(account_id, folder)
            );
            CREATE TABLE rule_events (
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                event_key TEXT NOT NULL, PRIMARY KEY(account_id,event_key)
            );
            CREATE TABLE rule_jobs (
                message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
                rules_json TEXT NOT NULL, processed INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX idx_rule_jobs_pending ON rule_jobs(processed, message_id);
        ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to execute v3: {e}")))?;
        self.record_migration(tx, 3, "003_local_rules")
    }

    fn apply_v2(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        // INVARIANT: Append-only Migration v2 (ADR-010). Adds folder catalog, message folder memberships,
        // message flag and snooze columns, and encrypted account avatars.
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS folders (
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                folder_name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                delimiter TEXT,
                role TEXT NOT NULL DEFAULT 'other',
                is_selectable INTEGER NOT NULL DEFAULT 1,
                total_messages INTEGER,
                unread_messages INTEGER,
                updated_at_epoch_ms INTEGER NOT NULL,
                PRIMARY KEY (account_id, folder_name)
            );

            CREATE INDEX IF NOT EXISTS idx_folders_account_role
            ON folders(account_id, role);

            CREATE TABLE IF NOT EXISTS message_folder_memberships (
                account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                folder TEXT NOT NULL,
                uid INTEGER NOT NULL,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                created_at_epoch_ms INTEGER NOT NULL,
                PRIMARY KEY (account_id, folder, uid)
            );

            CREATE INDEX IF NOT EXISTS idx_memberships_message_id
            ON message_folder_memberships(message_id);

            CREATE INDEX IF NOT EXISTS idx_memberships_account_folder
            ON message_folder_memberships(account_id, folder);

            CREATE TABLE IF NOT EXISTS account_avatars (
                account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
                avatar_bytes BLOB NOT NULL,
                mime_type TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                updated_at_epoch_ms INTEGER NOT NULL
            );
            ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to execute v2 tables: {e}")))?;

        // Conditionally add columns to messages if they don't already exist
        let columns: Vec<String> = tx
            .prepare("PRAGMA table_info(messages);")
            .map_err(|e| {
                StorageError::Migration(format!("failed to inspect messages columns: {e}"))
            })?
            .query_map([], |row| row.get(1))
            .map_err(|e| StorageError::Migration(format!("failed to query messages columns: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| {
                StorageError::Migration(format!("failed to collect messages columns: {e}"))
            })?;

        if !columns.iter().any(|c| c == "is_starred") {
            tx.execute(
                "ALTER TABLE messages ADD COLUMN is_starred INTEGER NOT NULL DEFAULT 0;",
                [],
            )
            .map_err(|e| {
                StorageError::Migration(format!("failed to add is_starred column: {e}"))
            })?;
        }

        if !columns.iter().any(|c| c == "snooze_until_epoch_ms") {
            tx.execute(
                "ALTER TABLE messages ADD COLUMN snooze_until_epoch_ms INTEGER;",
                [],
            )
            .map_err(|e| {
                StorageError::Migration(format!("failed to add snooze_until_epoch_ms column: {e}"))
            })?;
        }

        if !columns.iter().any(|c| c == "provider_message_id") {
            tx.execute(
                "ALTER TABLE messages ADD COLUMN provider_message_id TEXT;",
                [],
            )
            .map_err(|e| {
                StorageError::Migration(format!("failed to add provider_message_id column: {e}"))
            })?;
        }

        tx.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_messages_account_starred_received
            ON messages(account_id, is_starred, received_at_epoch_ms DESC);

            CREATE INDEX IF NOT EXISTS idx_messages_snooze_until
            ON messages(snooze_until_epoch_ms);

            CREATE INDEX IF NOT EXISTS idx_messages_account_provider_msgid
            ON messages(account_id, provider_message_id);

            -- Backfill memberships for messages already existing in v1
            INSERT OR IGNORE INTO message_folder_memberships (account_id, folder, uid, message_id, created_at_epoch_ms)
            SELECT account_id, folder, uid, id, created_at_epoch_ms FROM messages;

            -- Backfill folders from folder_sync_state
            INSERT OR IGNORE INTO folders (account_id, folder_name, display_name, delimiter, role, is_selectable, total_messages, unread_messages, updated_at_epoch_ms)
            SELECT account_id, folder_name, folder_name, '/',
                CASE WHEN lower(folder_name) = 'inbox' THEN 'inbox' ELSE 'other' END,
                1, NULL, NULL, updated_at_epoch_ms
            FROM folder_sync_state;
            ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to execute v2 indices and backfill: {e}")))?;

        self.record_migration(tx, 2, "002_account_mailboxes_v2")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{open_in_memory_connection, DatabaseKey};

    #[test]
    fn v2_upgrade_preserves_mail_and_adds_rules_without_rewriting_history() {
        let key = DatabaseKey::from_bytes([75; 32]);
        let mut conn = open_in_memory_connection(&key).unwrap();
        let migrator = SchemaMigrator::new(FtsTokenizerMode::Unicode61RemoveDiacritics);
        {
            let tx = conn.transaction().unwrap();
            tx.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at_epoch_ms INTEGER NOT NULL)").unwrap();
            migrator.apply_v1(&tx).unwrap();
            migrator.apply_v2(&tx).unwrap();
            tx.execute_batch("INSERT INTO accounts VALUES ('a','reader@example.com','synthetic',0,0);
                INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms)
                VALUES ('old','a','INBOX',1,'Old','sender@example.org','reader@example.com',0,'',0)").unwrap();
            tx.commit().unwrap();
        }
        assert_eq!(migrator.migrate(&mut conn).unwrap(), 5);
        assert_eq!(migrator.migrate(&mut conn).unwrap(), 0);
        assert_eq!(
            conn.query_row("SELECT subject FROM messages WHERE id='old'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "Old"
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM rule_jobs", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT max(version) FROM schema_migrations", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            7
        );
    }

    #[test]
    fn executes_migrations_idempotently() {
        let key = DatabaseKey::from_bytes([77u8; 32]);
        let mut conn = open_in_memory_connection(&key).expect("open conn");

        let migrator = SchemaMigrator::new(FtsTokenizerMode::Unicode61RemoveDiacritics);
        let applied_first = migrator.migrate(&mut conn).expect("first migration");
        assert_eq!(applied_first, 7);

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
        assert!(tables.contains(&"folders".to_string()));
        assert!(tables.contains(&"message_folder_memberships".to_string()));
        assert!(tables.contains(&"account_avatars".to_string()));
        assert!(tables.contains(&"saved_views".to_string()));
        assert!(tables.contains(&"reminder_configs".to_string()));
        assert!(tables.contains(&"reminder_jobs".to_string()));
        assert!(tables.contains(&"due_date_dismissals".to_string()));
        assert!(!tables.contains(&"vec_messages_by_account".to_string()));

        // Verify messages columns
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(messages);")
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(columns.contains(&"is_starred".to_string()));
        assert!(columns.contains(&"snooze_until_epoch_ms".to_string()));
        assert!(columns.contains(&"provider_message_id".to_string()));
    }
}
