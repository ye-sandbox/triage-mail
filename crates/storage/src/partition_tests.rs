//! Transactional account-partitioned vector index regressions; synthetic local fixtures only.
use crate::{DatabaseKey, FolderSyncState, HybridQuery, SearchEngine, SearchMode, SqliteMailStore};

fn store() -> SqliteMailStore {
    let store =
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([81; 32]), Default::default())
            .unwrap();
    store.conn.lock().unwrap().execute_batch(
        "INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.com','synthetic',0,0);"
    ).unwrap();
    message(&store, "one", "a", 1);
    message(&store, "two", "a", 2);
    store
}

fn message(store: &SqliteMailStore, id: &str, account: &str, uid: u32) {
    store
        .conn
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
         received_at_epoch_ms,raw_headers,created_at_epoch_ms)
         VALUES (?1,?2,'INBOX',?3,'synthetic','sender@example.com','reader@example.com',0,'',0)",
            rusqlite::params![id, account, uid],
        )
        .unwrap();
}

fn vector(x: i8, y: i8) -> Vec<i8> {
    let mut values = vec![0; 384];
    values[0] = x;
    values[1] = y;
    values
}

fn ids(store: &SqliteMailStore, account: Option<&str>) -> Vec<String> {
    store
        .search(&HybridQuery {
            query_text: String::new(),
            query_embedding_int8: Some(vector(100, 0)),
            account_filter: account.map(Into::into),
            mode: SearchMode::Semantic,
            limit: 10,
            offset: 0,
        })
        .unwrap()
        .into_iter()
        .map(|row| row.id)
        .collect()
}

fn count(store: &SqliteMailStore, table: &str) -> i64 {
    store
        .conn
        .lock()
        .unwrap()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn replacement_rollback_and_partition_queries_are_atomic() {
    let store = store();
    store
        .insert_message_embedding("one", &vector(100, 0))
        .unwrap();
    store
        .insert_message_embedding("two", &vector(80, 60))
        .unwrap();
    assert_eq!(ids(&store, Some("a")), ["one", "two"]);

    // Update embedding for "one" to point opposite direction
    store
        .insert_message_embedding("one", &vector(-100, 0))
        .unwrap();
    assert_eq!(ids(&store, Some("a")), ["two", "one"]);

    // Invalid dimensions roll back cleanly
    assert!(store.insert_message_embedding("one", &[1, 2, 3]).is_err());
    assert_eq!(ids(&store, Some("a")), ["two", "one"]);
    assert_eq!(count(&store, "vec_messages"), 2);

    // Second account message and vector
    message(&store, "other", "b", 1);
    store
        .insert_message_embedding("other", &vector(90, 0))
        .unwrap();
    assert_eq!(count(&store, "vec_messages"), 3);

    // Account filtering isolates partitions
    assert_eq!(ids(&store, Some("a")), ["two", "one"]);
    assert_eq!(ids(&store, Some("b")), ["other"]);

    // Global (unfiltered) KNN searches across partitions correctly
    let global = ids(&store, None);
    assert_eq!(global.len(), 3);
    assert_eq!(global[0], "other"); // 90, 0 has highest similarity with query 100, 0 after "one" was negated
}

#[test]
fn folder_reset_removes_partition_membership() {
    let store = store();
    store
        .insert_message_embedding("one", &vector(100, 0))
        .unwrap();
    store
        .insert_message_embedding("two", &vector(80, 60))
        .unwrap();
    message(&store, "other", "b", 3);
    store
        .insert_message_embedding("other", &vector(100, 0))
        .unwrap();

    let state = FolderSyncState {
        account_id: "a".into(),
        folder_name: "INBOX".into(),
        uidvalidity: 999,
        uidnext: 1,
        highest_modseq: None,
    };
    store.commit_sync_batch(&[], &[], &state, true).unwrap();
    assert!(ids(&store, Some("a")).is_empty());
    assert_eq!(ids(&store, Some("b")), ["other"]);
    assert_eq!(count(&store, "vec_messages"), 1);

    let state_b = FolderSyncState {
        account_id: "b".into(),
        folder_name: "INBOX".into(),
        uidvalidity: 999,
        uidnext: 1,
        highest_modseq: None,
    };
    store.commit_sync_batch(&[], &[], &state_b, true).unwrap();
    assert_eq!(count(&store, "vec_messages"), 0);
}
