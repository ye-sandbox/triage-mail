use super::*;
use crate::SqliteMailStore;

fn fixture() -> (tempfile::TempDir, DatabaseKey, SqliteMailStore) {
    let dir = tempfile::tempdir().unwrap();
    let key = DatabaseKey::from_bytes([17; 32]);
    let store =
        SqliteMailStore::open(dir.path().join("mail.db"), &key, FtsTokenizerMode::Trigram).unwrap();
    store.conn.lock().unwrap().execute_batch(
        "PRAGMA wal_autocheckpoint=0;
         INSERT INTO accounts VALUES ('a','reader@example.org','AppPassword',0,0);
         INSERT INTO accounts VALUES ('b','other@example.org','AppPassword',0,0);
         INSERT INTO messages (id,account_id,folder,uid,subject,sender,recipients,
             received_at_epoch_ms,raw_headers,text_body,html_body,created_at_epoch_ms)
         VALUES ('m1','a','INBOX',1,'boleto','sender@example.org','reader@example.org',1,'','cached text','<img src=x>',0),
                ('m2','b','INBOX',2,'boleto','sender@example.org','other@example.org',2,'','other text',NULL,0);"
    ).unwrap();
    (dir, key, store)
}

#[test]
fn sees_committed_wal_and_concurrent_updates_without_logical_writes() {
    let (dir, key, store) = fixture();
    let path = dir.path().join("mail.db");
    assert!(dir.path().join("mail.db-wal").metadata().unwrap().len() > 0);
    let reader = CachedMailReader::open(&path, &key).unwrap();
    assert_eq!(reader.tokenizer_mode().unwrap(), FtsTokenizerMode::Trigram);
    assert_eq!(reader.accounts(1, 1).unwrap(), vec!["b"]);
    let query = lexical_query("boleto".into(), Some("a".into()), 10, 0);
    let expected = crate::SearchEngine::search(&store, &query).unwrap();
    assert_eq!(
        reader
            .search("boleto".into(), Some("a".into()), 10, 0)
            .unwrap(),
        expected
    );
    assert!(reader
        .search("boleto".into(), Some("missing".into()), 10, 0)
        .unwrap()
        .is_empty());
    let body = reader.show("m1").unwrap().unwrap();
    assert_eq!(body.text_plain.as_deref(), Some("cached text"));
    assert!(!body.truncated);
    assert!(reader.show("missing").unwrap().is_none());
    assert!(reader
        .0
        .execute("UPDATE messages SET is_read=1", [])
        .is_err());
    assert!(reader.0.execute_batch("CREATE TABLE forbidden(x)").is_err());
    // Even disabling query_only cannot defeat the underlying READONLY open flags.
    reader.0.execute_batch("PRAGMA query_only=OFF").unwrap();
    assert!(reader.0.execute("DELETE FROM messages", []).is_err());
    reader.0.execute_batch("PRAGMA query_only=ON").unwrap();
    let writer = std::thread::spawn(move || {
        store
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE messages SET text_body='new committed text' WHERE id='m1'",
                [],
            )
            .unwrap();
        store
    });
    let store = writer.join().unwrap();
    assert_eq!(
        reader.show("m1").unwrap().unwrap().text_plain.as_deref(),
        Some("new committed text")
    );
    let conn = store.conn.lock().unwrap();
    let unread: u32 = conn
        .query_row("SELECT COUNT(*) FROM messages WHERE is_read=0", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(unread, 2);
    assert_eq!(
        conn.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert!(reader.accounts(501, 0).is_err());
    assert!(reader.search("x".repeat(32769), None, 1, 0).is_err());
}

#[test]
fn bounds_plain_text_and_never_substitutes_html() {
    let (dir, key, store) = fixture();
    let text = format!("{}é-end", "x".repeat(MAX_TEXT_BYTES - 1));
    store
        .conn
        .lock()
        .unwrap()
        .execute("UPDATE messages SET text_body=?1 WHERE id='m1'", [&text])
        .unwrap();
    store
        .conn
        .lock()
        .unwrap()
        .execute("UPDATE messages SET text_body=NULL WHERE id='m2'", [])
        .unwrap();
    let reader = CachedMailReader::open(&dir.path().join("mail.db"), &key).unwrap();
    let body = reader.show("m1").unwrap().unwrap();
    assert!(body.truncated);
    assert_eq!(body.text_plain.unwrap().len(), MAX_TEXT_BYTES - 1);
    assert!(reader.show("m2").unwrap().unwrap().text_plain.is_none());
}

#[test]
fn refuses_missing_wrong_key_and_incompatible_schema_without_repair() {
    let (dir, key, store) = fixture();
    let missing = dir.path().join("absent.db");
    assert!(CachedMailReader::open(&missing, &key).is_err());
    assert!(!missing.exists());
    let path = dir.path().join("mail.db");
    assert!(CachedMailReader::open(&path, &DatabaseKey::from_bytes([18; 32])).is_err());
    let reader = CachedMailReader::open(&path, &key).unwrap();
    store
        .conn
        .lock()
        .unwrap()
        .execute("DELETE FROM schema_migrations WHERE version=11", [])
        .unwrap();
    assert!(reader.accounts(1, 0).is_err());
    assert!(CachedMailReader::open(&path, &key).is_err());
    let conn = store.conn.lock().unwrap();
    assert_eq!(
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        10
    );
    conn.execute("INSERT INTO schema_migrations VALUES (12,'future',0)", [])
        .unwrap();
    assert!(CachedMailReader::open(&path, &key).is_err());
}
