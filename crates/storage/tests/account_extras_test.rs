use storage::desktop::AccountConnectionExtra;
use storage::{now_epoch_ms, AccountRecord, DatabaseKey, SqliteMailStore};
use tempfile::tempdir;

#[test]
fn extras_round_trip_additively_without_clobbering_config() {
    let dir = tempdir().expect("tempdir");
    let key = DatabaseKey::from_bytes([0x42; 32]);
    let store = SqliteMailStore::open(
        dir.path().join("t.db"),
        &key,
        storage::FtsTokenizerMode::Unicode61RemoveDiacritics,
    )
    .expect("open");
    let now = now_epoch_ms();
    let acct = AccountRecord {
        id: "a1".into(),
        email: "a1@example.com".into(),
        auth_type: "Password".into(),
        created_at_epoch_ms: now,
        updated_at_epoch_ms: now,
    };
    store
        .insert_account_connection(&acct, r#"{"imap_host":"imap.example.com"}"#)
        .unwrap();
    let q = AccountConnectionExtra::Quota;
    assert_eq!(store.get_account_connection_extra("a1", q).unwrap(), None);
    assert!(store
        .set_account_connection_extra("a1", q, r#"{"used_bytes":1,"limit_bytes":2}"#)
        .unwrap());
    let got = store
        .get_account_connection_extra("a1", q)
        .unwrap()
        .unwrap();
    assert_eq!(got, r#"{"used_bytes":1,"limit_bytes":2}"#);
    let cfg = store.get_account_connection("a1").unwrap().unwrap();
    assert!(cfg.contains("imap.example.com"));
    assert!(!store
        .set_account_connection_extra("missing", q, "{}")
        .unwrap());
}
