use crate::StorageError;

/// Registers the `sqlite-vec` C-extension with SQLite via `sqlite3_auto_extension`.
///
/// INVARIANT: `crates/storage` enforces `#![deny(unsafe_code)]`. This function provides the
/// single isolated block permitted to execute unsafe FFI strictly for registering `sqlite-vec`
/// once across the entire process lifetime.
#[allow(unsafe_code)]
pub fn register_sqlite_vec() -> Result<(), StorageError> {
    static ONCE: std::sync::Once = std::sync::Once::new();
    let mut err_msg: Option<String> = None;

    ONCE.call_once(|| {
        unsafe {
            // INVARIANT: sqlite3_vec_init is linked from sqlite_vec crate and registered
            // via sqlite3_auto_extension so all new SQLite connections automatically recognize `vec0`.
            let init_fn: unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *mut std::os::raw::c_char,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> std::os::raw::c_int =
                std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
            let rc = rusqlite::ffi::sqlite3_auto_extension(Some(init_fn));
            if rc != rusqlite::ffi::SQLITE_OK {
                err_msg = Some(format!(
                    "failed to register sqlite-vec auto extension: SQLite error code {rc}"
                ));
            }
        }
    });

    if let Some(msg) = err_msg {
        return Err(StorageError::Migration(msg));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn registers_and_executes_vec_version() {
        register_sqlite_vec().expect("sqlite-vec registration should succeed");
        let conn = Connection::open_in_memory().expect("open in-memory db");
        let version: String = conn
            .query_row("SELECT vec_version()", [], |r| r.get(0))
            .expect("vec_version query");
        assert!(
            version.starts_with('v'),
            "vec_version should start with 'v', got {version}"
        );

        conn.execute(
            "CREATE VIRTUAL TABLE test_vec USING vec0(
                message_id text primary key,
                embedding_int8 int8[384] distance_metric=cosine
            );",
            [],
        )
        .expect("create vec0 virtual table");

        let vec_data = vec![1i8; 384];
        let vec_json = format!(
            "[{}]",
            vec_data
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        conn.execute(
            "INSERT INTO test_vec(message_id, embedding_int8) VALUES (?1, vec_int8(?2));",
            rusqlite::params!["msg_1", vec_json],
        )
        .expect("insert vec0");

        let distance: f32 = conn
            .query_row(
                "SELECT distance FROM test_vec WHERE embedding_int8 MATCH vec_int8(?1) AND k = 1",
                rusqlite::params![vec_json],
                |r| r.get(0),
            )
            .expect("knn match");
        assert!(
            distance < 0.001,
            "distance to identical vector should be ~0, got {distance}"
        );

        let vec_bytes: Vec<u8> = vec_data.iter().map(|&b| b as u8).collect();
        conn.execute(
            "INSERT INTO test_vec(message_id, embedding_int8) VALUES (?1, vec_int8(?2));",
            rusqlite::params!["msg_2", vec_bytes],
        )
        .expect("insert vec0 with blob vec_int8");

        let distance2: f32 = conn
            .query_row(
                "SELECT distance FROM test_vec WHERE embedding_int8 MATCH vec_int8(?1) AND k = 1",
                rusqlite::params![vec_bytes],
                |r| r.get(0),
            )
            .expect("knn match with blob vec_int8");
        assert!(
            distance2 < 0.001,
            "distance to identical vector blob should be ~0, got {distance2}"
        );
    }
}
