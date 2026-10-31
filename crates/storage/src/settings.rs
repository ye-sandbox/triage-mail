//! Transactional FTS reconfiguration; table SQL is the persisted source of truth.
use std::sync::atomic::Ordering;

use rusqlite::Connection;

use crate::{FtsTokenizerMode, SqliteMailStore, StorageError};

pub(crate) fn read_tokenizer(conn: &Connection) -> Result<FtsTokenizerMode, StorageError> {
    let sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='messages_fts'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| StorageError::Query(e.to_string()))?;
    Ok(if sql.contains("tokenize='trigram'") {
        FtsTokenizerMode::Trigram
    } else {
        FtsTokenizerMode::Unicode61RemoveDiacritics
    })
}

impl SqliteMailStore {
    pub fn set_tokenizer(&self, mode: FtsTokenizerMode) -> Result<(), StorageError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        if read_tokenizer(&conn)? == mode {
            return Ok(());
        }
        let tokenizer = match mode {
            FtsTokenizerMode::Trigram => "trigram",
            FtsTokenizerMode::Unicode61RemoveDiacritics => "unicode61 remove_diacritics 2",
        };
        // INVARIANT: Readers and ingestion share this connection lock. Rebuild and replacement
        // commit together; rollback preserves the old index and its external-content triggers.
        let tx = conn
            .transaction()
            .map_err(|e| StorageError::Query(e.to_string()))?;
        tx.execute_batch(&format!(
            "DROP TABLE messages_fts;
             CREATE VIRTUAL TABLE messages_fts USING fts5(subject, sender, text_body,
                 content='messages', content_rowid='rowid', tokenize='{tokenizer}');
             INSERT INTO messages_fts(messages_fts) VALUES ('rebuild');"
        ))
        .map_err(|e| StorageError::Query(e.to_string()))?;
        tx.commit()
            .map_err(|e| StorageError::Query(e.to_string()))?;
        self.tokenizer_mode.store(
            u8::from(mode == FtsTokenizerMode::Trigram),
            Ordering::Release,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatabaseKey;

    #[test]
    fn tokenizer_rebuild_persists_and_keeps_triggers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let key = DatabaseKey::from_bytes([12; 32]);
        let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        store.conn.lock().unwrap().execute_batch(
            "INSERT INTO accounts VALUES ('a','reader@example.com','AppPassword',0,0);
             INSERT INTO messages (id,account_id,folder,uid,subject,sender,recipients,
                 received_at_epoch_ms,raw_headers,created_at_epoch_ms)
             VALUES ('m','a','INBOX',1,'alphabet','sender@example.com','reader@example.com',0,'',0);"
        ).unwrap();
        store.set_tokenizer(FtsTokenizerMode::Trigram).unwrap();
        drop(store);
        let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        assert_eq!(store.tokenizer_mode(), FtsTokenizerMode::Trigram);
        let conn = store.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM messages_fts WHERE messages_fts MATCH 'pha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        conn.execute("UPDATE messages SET subject='different' WHERE id='m'", [])
            .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM messages_fts WHERE messages_fts MATCH 'pha'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        drop(conn);
        store
            .set_tokenizer(FtsTokenizerMode::Unicode61RemoveDiacritics)
            .unwrap();
        assert_eq!(
            store.tokenizer_mode(),
            FtsTokenizerMode::Unicode61RemoveDiacritics
        );
    }
}
