use std::path::Path;

use rusqlite::Connection;

use crate::vec_ext::register_sqlite_vec;
use crate::StorageError;

/// 256-bit symmetric encryption key for full-database SQLCipher protection.
#[derive(Clone)]
pub struct DatabaseKey([u8; 32]);

impl DatabaseKey {
    /// Constructs a database encryption key from raw 32-byte key material.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Formats the raw 256-bit key into SQLCipher raw key hex notation (`x'...'`).
    ///
    /// INVARIANT: Using raw 256-bit keys with `x'...'` bypasses PBKDF2 key derivation,
    /// providing sub-millisecond connection setup while enforcing AES-256 encryption.
    pub fn to_sqlcipher_hex(&self) -> String {
        let mut hex = String::with_capacity(67);
        hex.push_str("x'");
        for byte in &self.0 {
            use std::fmt::Write;
            let _ = write!(hex, "{byte:02x}");
        }
        hex.push('\'');
        hex
    }
}

impl std::fmt::Debug for DatabaseKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DatabaseKey([REDACTED])")
    }
}

/// Opens a connection to a SQLCipher encrypted database, applies performance pragmas,
/// and registers the `sqlite-vec` extension.
pub fn open_encrypted_connection<P: AsRef<Path>>(
    path: P,
    key: &DatabaseKey,
) -> Result<Connection, StorageError> {
    register_sqlite_vec()?;

    let conn = Connection::open(path).map_err(|e| StorageError::Query(e.to_string()))?;
    configure_connection(&conn, key)?;
    Ok(conn)
}

/// Opens an in-memory SQLCipher encrypted database for hermetic testing.
pub fn open_in_memory_connection(key: &DatabaseKey) -> Result<Connection, StorageError> {
    register_sqlite_vec()?;

    let conn = Connection::open_in_memory().map_err(|e| StorageError::Query(e.to_string()))?;
    configure_connection(&conn, key)?;
    Ok(conn)
}

/// Applies SQLCipher encryption key and performance pragmas to the connection.
fn configure_connection(conn: &Connection, key: &DatabaseKey) -> Result<(), StorageError> {
    // INVARIANT: PRAGMA key MUST be executed before any other database operations.
    let key_hex = key.to_sqlcipher_hex();
    conn.pragma_update(None, "key", &key_hex)
        .map_err(|e| StorageError::Query(format!("failed to set SQLCipher key: {e}")))?;

    // Validate key validity immediately by querying sqlite_master
    conn.query_row("SELECT count(*) FROM sqlite_master;", [], |_| Ok(()))
        .map_err(|e| {
            StorageError::Query(format!(
                "database key verification failed or corrupted database: {e}"
            ))
        })?;

    // INVARIANT: WAL mode + MEMORY temp_store + foreign keys + busy_timeout
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA temp_store = MEMORY;
        PRAGMA busy_timeout = 5000;
        ",
    )
    .map_err(|e| StorageError::Query(format!("failed to configure SQLite pragmas: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_round_trip_and_rejects_invalid_key() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test_vault.db");

        let key = DatabaseKey::from_bytes([42u8; 32]);
        let wrong_key = DatabaseKey::from_bytes([99u8; 32]);

        // 1. Create encrypted database and insert sensitive data
        {
            let conn = open_encrypted_connection(&db_path, &key).expect("open encrypted db");
            conn.execute(
                "CREATE TABLE secrets (id TEXT PRIMARY KEY, sensitive TEXT NOT NULL);",
                [],
            )
            .expect("create table");
            conn.execute(
                "INSERT INTO secrets (id, sensitive) VALUES ('1', 'TOP_SECRET_FEBRABAN_BOLETO');",
                [],
            )
            .expect("insert secret");
        }

        // 2. Verify raw file content is encrypted (does not contain plaintext)
        let raw_bytes = std::fs::read(&db_path).expect("read raw db bytes");
        let raw_str = String::from_utf8_lossy(&raw_bytes);
        assert!(
            !raw_str.contains("TOP_SECRET_FEBRABAN_BOLETO"),
            "Database file MUST NOT contain plaintext"
        );
        assert!(
            !raw_str.contains("SQLite format 3"),
            "SQLCipher header MUST NOT expose unencrypted SQLite magic"
        );

        // 3. Opening with wrong key MUST fail
        let wrong_res = open_encrypted_connection(&db_path, &wrong_key);
        assert!(
            wrong_res.is_err(),
            "Opening with wrong key must return an error"
        );

        // 4. Opening with correct key MUST succeed and read data
        {
            let conn = open_encrypted_connection(&db_path, &key).expect("reopen with correct key");
            let val: String = conn
                .query_row("SELECT sensitive FROM secrets WHERE id = '1'", [], |r| {
                    r.get(0)
                })
                .expect("query secret");
            assert_eq!(val, "TOP_SECRET_FEBRABAN_BOLETO");
        }
    }
}
