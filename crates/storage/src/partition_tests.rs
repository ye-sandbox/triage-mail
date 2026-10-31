//! Transactional derived-index regressions; synthetic local fixtures only.
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

fn ids(store: &SqliteMailStore, account: &str) -> Vec<String> {
    store
        .search(&HybridQuery {
            query_text: String::new(),
            query_embedding_int8: Some(vector(100, 0)),
            account_filter: Some(account.into()),
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
fn replacement_rollback_and_late_message_membership_are_atomic() {
    let store = store();
    store
        .insert_message_embedding("one", &vector(100, 0))
        .unwrap();
    store
        .insert_message_embedding("two", &vector(80, 60))
        .unwrap();
    assert_eq!(ids(&store, "a"), ["one", "two"]);
    store
        .insert_message_embedding("one", &vector(-100, 0))
        .unwrap();
    assert_eq!(ids(&store, "a"), ["two", "one"]);
    assert!(store.insert_message_embedding("one", &[1, 2, 3]).is_err());
    assert_eq!(ids(&store, "a"), ["two", "one"]);
    assert_eq!(count(&store, "vec_messages"), 2);
    assert_eq!(count(&store, "vec_messages_by_account"), 2);
    store
        .insert_message_embedding("late", &vector(100, 0))
        .unwrap();
    assert_eq!(count(&store, "vec_messages_by_account"), 2);
    message(&store, "late", "b", 3);
    assert_eq!(ids(&store, "b"), ["late"]);
    store
        .conn
        .lock()
        .unwrap()
        .execute("UPDATE messages SET account_id='b' WHERE id='two'", [])
        .unwrap();
    assert_eq!(ids(&store, "a"), ["one"]);
    assert_eq!(ids(&store, "b"), ["late", "two"]);
}

#[test]
fn folder_reset_and_account_cascade_remove_partition_membership() {
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
    assert!(ids(&store, "a").is_empty());
    assert_eq!(ids(&store, "b"), ["other"]);
    assert_eq!(count(&store, "vec_messages_by_account"), 1);
    store
        .conn
        .lock()
        .unwrap()
        .execute("DELETE FROM accounts WHERE id='b'", [])
        .unwrap();
    assert_eq!(count(&store, "vec_messages_by_account"), 0);
}
