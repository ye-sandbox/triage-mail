//! Read-only access to an already initialized encrypted cache, without workers or migrations.
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;

use crate::{
    DatabaseKey, FtsTokenizerMode, HybridQuery, MessageRowSummary, SearchMode, StorageError,
};

pub const MAX_TEXT_BYTES: usize = 64 * 1024;

/// An encrypted cache reader deliberately exposing no connection or mutation APIs.
pub struct CachedMailReader(Connection);

#[derive(Debug, Serialize)]
pub struct CachedPlainText {
    pub id: String,
    pub account_id: String,
    pub text_plain: Option<String>,
    pub truncated: bool,
}

fn unavailable(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("cached mail unavailable".into())
}

impl CachedMailReader {
    pub fn open(path: &Path, key: &DatabaseKey) -> Result<Self, StorageError> {
        crate::vec_ext::register_sqlite_vec()?;
        // INVARIANT: No CREATE, immutable mode, journal-mode changes or migrator. WAL
        // readers must see committed desktop writes and may require existing SHM access.
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(unavailable)?;
        conn.pragma_update(None, "key", key.to_sqlcipher_hex())
            .map_err(unavailable)?;
        conn.execute_batch(
            "PRAGMA query_only=ON; PRAGMA temp_store=MEMORY; PRAGMA busy_timeout=5000;",
        )
        .map_err(unavailable)?;
        let reader = Self(conn);
        reader.snapshot(|conn| crate::settings::read_tokenizer(conn).map(|_| ()))?;
        Ok(reader)
    }

    fn snapshot<T>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let tx = self.0.unchecked_transaction().map_err(unavailable)?;
        let (count, min, max): (u32, u32, u32) = tx.query_row(
            "SELECT COUNT(*), COALESCE(MIN(version),0), COALESCE(MAX(version),0) FROM schema_migrations",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(unavailable)?;
        let current = crate::migrations::CURRENT_SCHEMA_VERSION;
        // INVARIANT: Validate schema within the same snapshot as every read. No upgrade
        // is attempted, even if another process changes the schema after this open.
        if count != current || min != 1 || max != current {
            return Err(StorageError::Query("unsupported cache schema".into()));
        }
        read(&tx)
    }

    pub fn tokenizer_mode(&self) -> Result<FtsTokenizerMode, StorageError> {
        self.snapshot(crate::settings::read_tokenizer)
    }

    pub fn accounts(&self, limit: usize, offset: usize) -> Result<Vec<String>, StorageError> {
        let query = lexical_query(String::new(), None, limit, offset);
        query.validate()?;
        self.snapshot(|conn| {
            let mut stmt = conn
                .prepare("SELECT id FROM accounts ORDER BY id LIMIT ?1 OFFSET ?2")
                .map_err(unavailable)?;
            let rows = stmt
                .query_map(rusqlite::params![limit as i64, offset as i64], |r| r.get(0))
                .map_err(unavailable)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(unavailable)
        })
    }

    pub fn search(
        &self,
        text: String,
        account: Option<String>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        if text.len() > 32 * 1024 {
            return Err(StorageError::InvalidSearchQuery("query too long".into()));
        }
        let query = lexical_query(text, account, limit, offset);
        self.snapshot(|conn| crate::search::execute_lexical_search(conn, &query))
    }

    pub fn show(&self, id: &str) -> Result<Option<CachedPlainText>, StorageError> {
        self.snapshot(|conn| {
            // INVARIANT: Bound bytes before loading into Rust; do not read HTML, headers
            // or attachment data. An incomplete final UTF-8 scalar is omitted.
            conn.query_row(
                "SELECT id,account_id,substr(CAST(text_body AS BLOB),1,?2),
                 length(CAST(text_body AS BLOB)) > ?3 FROM messages WHERE id=?1",
                rusqlite::params![id, MAX_TEXT_BYTES as i64, MAX_TEXT_BYTES as i64],
                |row| {
                    let bytes: Option<Vec<u8>> = row.get(2)?;
                    let text_plain = bytes.map(bounded_text).transpose()?;
                    Ok(CachedPlainText {
                        id: row.get(0)?,
                        account_id: row.get(1)?,
                        text_plain,
                        truncated: row.get::<_, Option<bool>>(3)?.unwrap_or(false),
                    })
                },
            )
            .optional()
            .map_err(unavailable)
        })
    }
}

fn bounded_text(mut bytes: Vec<u8>) -> rusqlite::Result<String> {
    if let Err(error) = std::str::from_utf8(&bytes) {
        if error.error_len().is_some() {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(error),
            ));
        }
        bytes.truncate(error.valid_up_to());
    }
    String::from_utf8(bytes).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn lexical_query(
    query_text: String,
    account_filter: Option<String>,
    limit: usize,
    offset: usize,
) -> HybridQuery {
    HybridQuery {
        query_text,
        account_filter,
        limit,
        offset,
        mode: SearchMode::Lexical,
        query_embedding_int8: None,
    }
}

#[cfg(test)]
mod tests;
