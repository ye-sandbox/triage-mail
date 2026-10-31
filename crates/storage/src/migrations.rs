use rusqlite::Connection;

use crate::{FtsTokenizerMode, StorageError};

/// Current database schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

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
            self.apply_v5(&tx)?;
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

    fn apply_v5(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        // INVARIANT: This is a derived index, not a replacement for authoritative vec_messages.
        // Backfill and membership triggers commit with the schema version or roll back together.
        // INVARIANT: Trigger VALUES wraps the scalar subquery in vec_int8 so SQLite's
        // UPDATE materialization cannot strip the int8 subtype. Partition keys require
        // delete/reinsert; sqlite-vec 0.1.9 does not support updating them in place.
        tx.execute_batch(
            "CREATE VIRTUAL TABLE vec_messages_by_account USING vec0(
                message_id TEXT PRIMARY KEY,
                account_id TEXT PARTITION KEY,
                embedding_int8 int8[384] distance_metric=cosine
             );
             INSERT INTO vec_messages_by_account(message_id,account_id,embedding_int8)
                SELECT v.message_id,m.account_id,vec_int8(v.embedding_int8)
                FROM vec_messages v JOIN messages m ON m.id=v.message_id;
             CREATE TRIGGER messages_vector_insert AFTER INSERT ON messages
             WHEN EXISTS (SELECT 1 FROM vec_messages WHERE message_id=new.id) BEGIN
                INSERT INTO vec_messages_by_account(message_id,account_id,embedding_int8)
                VALUES(new.id,new.account_id,vec_int8((SELECT embedding_int8
                    FROM vec_messages WHERE message_id=new.id)));
             END;
             CREATE TRIGGER messages_vector_delete AFTER DELETE ON messages BEGIN
                DELETE FROM vec_messages_by_account WHERE message_id=old.id;
             END;
             CREATE TRIGGER messages_vector_account BEFORE UPDATE OF account_id ON messages
             WHEN old.account_id != new.account_id BEGIN
                DELETE FROM vec_messages_by_account WHERE message_id=old.id;
             END;
             CREATE TRIGGER messages_vector_account_insert AFTER UPDATE OF account_id ON messages
             WHEN old.account_id != new.account_id AND
                 EXISTS (SELECT 1 FROM vec_messages WHERE message_id=new.id) BEGIN
                INSERT INTO vec_messages_by_account(message_id,account_id,embedding_int8)
                VALUES(new.id,new.account_id,vec_int8((SELECT embedding_int8
                    FROM vec_messages WHERE message_id=new.id)));
             END;",
        )
        .map_err(|e| {
            StorageError::Migration(format!("failed to create account vector index: {e}"))
        })?;
        self.record_migration(tx, 5, "005_account_partitioned_vectors")
    }

    fn apply_v4(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        // INVARIANT: Connection metadata and the durable outbox contain no credentials.
        // Existing account/message schemas and public repository traits remain unchanged.
        tx.execute_batch(
            "
            CREATE TABLE account_connections (
                account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
                config_json TEXT NOT NULL
            );
            CREATE TABLE pending_mutations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                payload_json TEXT NOT NULL
            );
            CREATE TABLE message_folder_overrides (
                message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
                folder TEXT NOT NULL
            );
        ",
        )
        .map_err(|e| StorageError::Migration(e.to_string()))?;
        self.record_migration(tx, 4, "004_desktop_connections_and_outbox")
    }

    fn apply_v1(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
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
            ",
        )
        .map_err(|e| StorageError::Migration(format!("failed to execute v1 migration: {e}")))?;

        self.record_migration(tx, 1, "001_core_inbound_schema")?;
        Ok(())
    }

    fn apply_v2(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
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

        self.record_migration(tx, 2, "002_configurable_fts5_schema")?;
        Ok(())
    }

    fn apply_v3(&self, tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
        tx.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_messages USING vec0(
                message_id TEXT PRIMARY KEY,
                embedding_int8 int8[384] distance_metric=cosine
            );",
            [],
        )
        .map_err(|e| {
            StorageError::Migration(format!("failed to create vec_messages table: {e}"))
        })?;

        self.record_migration(tx, 3, "003_sqlite_vec_knn_schema")?;
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
        assert_eq!(applied_first, 5);

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
        assert!(tables.contains(&"messages_fts".to_string()));
        assert!(tables.contains(&"vec_messages".to_string()));
    }

    fn v4_fixture() -> Connection {
        let mut conn = open_in_memory_connection(&DatabaseKey::from_bytes([82; 32])).unwrap();
        seed_v4(&mut conn);
        conn
    }

    fn seed_v4(conn: &mut Connection) {
        let tx = conn.transaction().unwrap();
        let migrator = SchemaMigrator::new(Default::default());
        tx.execute_batch(
            "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,applied_at_epoch_ms INTEGER NOT NULL);",
        )
        .unwrap();
        migrator.apply_v1(&tx).unwrap();
        migrator.apply_v2(&tx).unwrap();
        migrator.apply_v3(&tx).unwrap();
        migrator.apply_v4(&tx).unwrap();
        tx.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0);
          INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
          received_at_epoch_ms,raw_headers,created_at_epoch_ms)
          VALUES ('live','a','INBOX',1,'synthetic','sender@example.com','reader@example.com',0,'',0);").unwrap();
        for id in ["live", "orphan"] {
            tx.execute(
                "INSERT INTO vec_messages VALUES (?1,vec_int8(?2))",
                rusqlite::params![id, vec![3u8; 384]],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    #[test]
    fn v5_backfills_only_live_vectors_and_is_idempotent() {
        let mut conn = v4_fixture();
        let migrator = SchemaMigrator::new(Default::default());
        assert_eq!(migrator.migrate(&mut conn).unwrap(), 1);
        let row: (String, String) = conn
            .query_row(
                "SELECT message_id,account_id FROM vec_messages_by_account",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, ("live".into(), "a".into()));
        assert_eq!(migrator.migrate(&mut conn).unwrap(), 0);
    }

    #[test]
    fn failed_v5_rolls_back_backfill_and_schema_version() {
        let mut conn = v4_fixture();
        conn.execute_batch(
            "CREATE TRIGGER messages_vector_account AFTER UPDATE ON messages BEGIN SELECT 1; END;",
        )
        .unwrap();
        let migrator = SchemaMigrator::new(Default::default());
        assert!(migrator.migrate(&mut conn).is_err());
        let version: u32 = conn
            .query_row("SELECT max(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 4);
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='vec_messages_by_account'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
        conn.execute_batch("DROP TRIGGER messages_vector_account")
            .unwrap();
        assert_eq!(migrator.migrate(&mut conn).unwrap(), 1);
    }

    #[test]
    fn encrypted_v4_upgrade_retains_partition_search_after_reopen() {
        use crate::{HybridQuery, MigrationRunner, SearchEngine, SearchMode, SqliteMailStore};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("upgrade.db");
        let key = DatabaseKey::from_bytes([83; 32]);
        let mut conn = crate::connection::open_encrypted_connection(&path, &key).unwrap();
        seed_v4(&mut conn);
        drop(conn);
        let query = HybridQuery {
            query_text: String::new(),
            query_embedding_int8: Some(vec![3; 384]),
            account_filter: Some("a".into()),
            mode: SearchMode::Semantic,
            limit: 10,
            offset: 0,
        };
        for _ in 0..2 {
            let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
            let rows = store.search(&query).unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].id, "live");
            assert_eq!(store.run_migrations().unwrap(), 0);
        }
    }
}
