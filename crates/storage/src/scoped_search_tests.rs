use rusqlite::params;

use crate::{
    DatabaseKey, HybridQuery, MailboxScope, MailboxView, SearchEngine, SearchMode, SqliteMailStore,
};

fn fixture() -> SqliteMailStore {
    let store =
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([47; 32]), Default::default())
            .unwrap();
    {
        let conn = store.conn.lock().unwrap();
        conn.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.org','synthetic',0,0);
            INSERT INTO folders(account_id,folder_name,display_name,role,is_selectable,updated_at_epoch_ms) VALUES
            ('a','INBOX','Inbox','inbox',1,0),('a','Archive','Archive','archive',1,0),
            ('a','Empty','Empty','other',1,0),('a','Container','Container','other',0,0),
            ('b','INBOX','Inbox','inbox',1,0);").unwrap();
        for account in ["a", "b"] {
            for uid in 1..=100 {
                let id = format!("{account}-{uid:03}");
                let folder = if uid % 2 == 0 { "INBOX" } else { "Archive" };
                let snooze = (uid % 5 == 0).then_some(crate::now_epoch_ms() + 600_000);
                conn.execute("INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
                    received_at_epoch_ms,raw_headers,created_at_epoch_ms,is_starred,snooze_until_epoch_ms)
                    VALUES (?1,?2,?3,?4,'synthetic','sender@example.com','reader@example.org',?4,'',0,?5,?6)",
                    params![id, account, folder, uid, uid % 3 == 0, snooze]).unwrap();
                conn.execute(
                    "INSERT INTO message_folder_memberships VALUES (?1,?2,?3,?4,0)",
                    params![account, folder, uid, id],
                )
                .unwrap();
            }
        }
        conn.execute_batch(
            "INSERT INTO message_folder_memberships VALUES ('a','Archive',102,'a-002',0);
            INSERT INTO message_folder_overrides VALUES ('a-002','Archive'),('a-003','INBOX');",
        )
        .unwrap();
    }
    for account in ["a", "b"] {
        for uid in 1..=100 {
            let mut vector = vec![0; 384];
            vector[0] = 100;
            vector[1] = if account == "a" { uid as i8 } else { 0 };
            store
                .insert_message_embedding(&format!("{account}-{uid:03}"), &vector)
                .unwrap();
        }
    }
    store
}

fn query(mode: SearchMode, text: &str, offset: usize) -> HybridQuery {
    let mut vector = vec![0; 384];
    vector[0] = 100;
    HybridQuery {
        query_text: text.into(),
        query_embedding_int8: (mode != SearchMode::Lexical).then_some(vector),
        account_filter: None,
        mode,
        limit: 7,
        offset,
    }
}

fn scope(view: MailboxView) -> MailboxScope {
    MailboxScope::AccountView {
        account_id: "a".into(),
        view,
    }
}

fn eligible(uid: i32, view: &MailboxView) -> bool {
    let inbox = (uid % 2 == 0 && uid != 2) || uid == 3;
    match view {
        MailboxView::Inbox => inbox && uid % 5 != 0,
        MailboxView::Starred => uid % 3 == 0,
        MailboxView::Snoozed => uid % 5 == 0,
        MailboxView::Folder(folder) if folder == "Archive" => !inbox,
        _ => false,
    }
}

#[test]
fn scoped_modes_match_eligible_ranking_oracles_before_pagination() {
    let store = fixture();
    let views = [
        MailboxView::Inbox,
        MailboxView::Starred,
        MailboxView::Snoozed,
        MailboxView::Folder("Archive".into()),
        MailboxView::Folder("Empty".into()),
    ];
    for view in views {
        for (mode, text) in [
            (SearchMode::Lexical, ""),
            (SearchMode::Lexical, "synthetic"),
            (SearchMode::Semantic, ""),
            (SearchMode::Hybrid, "synthetic"),
            (SearchMode::Hybrid, ""),
        ] {
            let mut expected: Vec<_> = (1..=100).filter(|uid| eligible(*uid, &view)).collect();
            // INVARIANT: Identical FTS documents tie by insertion rowid; vectors [100, uid]
            // have strictly increasing cosine distance to [100, 0], so both RRF ranks agree.
            if mode == SearchMode::Lexical && text.is_empty() {
                expected.reverse();
            }
            for offset in [0, 7, 25, 100, 5000] {
                let rows = store
                    .search_scoped(&query(mode, text, offset), &scope(view.clone()))
                    .unwrap();
                let oracle: Vec<_> = expected
                    .iter()
                    .skip(offset)
                    .take(7)
                    .map(|uid| format!("a-{uid:03}"))
                    .collect();
                assert_eq!(
                    rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
                    oracle,
                    "{view:?} {mode:?} offset {offset}"
                );
                assert!(rows.iter().all(|row| row.account_id == "a"));
                if let Some(row) = rows.iter().find(|row| row.id == "a-003") {
                    assert_eq!(row.folder, "Archive");
                    assert_eq!(row.uid, 3);
                    assert_eq!(row.folder_override.as_deref(), Some("INBOX"));
                }
            }
        }
    }
}

#[test]
fn unknown_scopes_fail_closed_and_global_preserves_existing_results() {
    let store = fixture();
    let query = query(SearchMode::Lexical, "", 0);
    assert_eq!(
        store.search(&query).unwrap(),
        store
            .search_scoped(&query, &MailboxScope::GlobalAllMail)
            .unwrap()
    );
    for invalid in [
        MailboxScope::AccountView {
            account_id: "unknown".into(),
            view: MailboxView::Starred,
        },
        scope(MailboxView::Folder("missing".into())),
        scope(MailboxView::Folder("Container".into())),
        scope(MailboxView::Folder("INBOX' OR 1=1 --".into())),
    ] {
        assert!(matches!(
            store.search_scoped(&query, &invalid),
            Err(crate::StorageError::InvalidSearchQuery(_))
        ));
    }
    let conflicting = HybridQuery {
        account_filter: Some("b".into()),
        ..query
    };
    assert!(store
        .search_scoped(&conflicting, &scope(MailboxView::Starred))
        .is_err());
}

#[test]
fn snooze_boundary_and_multi_membership_do_not_lose_or_duplicate_messages() {
    let store = fixture();
    store
        .conn
        .lock()
        .unwrap()
        .execute_batch(
            "UPDATE messages SET snooze_until_epoch_ms=1;
        INSERT INTO message_folder_memberships VALUES ('a','INBOX',103,'a-004',0);",
        )
        .unwrap();
    let mut query = query(SearchMode::Lexical, "", 0);
    query.limit = 500;
    let inbox = store
        .search_scoped(&query, &scope(MailboxView::Inbox))
        .unwrap();
    assert_eq!(inbox.len(), 50);
    assert_eq!(inbox.iter().filter(|row| row.id == "a-004").count(), 1);
    assert!(store
        .search_scoped(&query, &scope(MailboxView::Snoozed))
        .unwrap()
        .is_empty());
    assert_eq!(
        store
            .search_scoped(&query, &scope(MailboxView::Folder("INBOX".into())))
            .unwrap()
            .len(),
        50
    );
}

#[test]
fn snoozes_expire_at_exact_deadline_without_reconciliation() {
    let store = fixture();
    store
        .conn
        .lock()
        .unwrap()
        .execute_batch("UPDATE messages SET snooze_until_epoch_ms=1000;")
        .unwrap();
    let mut query = query(SearchMode::Lexical, "", 0);
    query.limit = 500;
    assert!(store
        .search_scoped_at(&query, &scope(MailboxView::Inbox), 999)
        .unwrap()
        .is_empty());
    assert_eq!(
        store
            .search_scoped_at(&query, &scope(MailboxView::Snoozed), 999)
            .unwrap()
            .len(),
        100
    );
    assert_eq!(
        store
            .search_scoped_at(&query, &scope(MailboxView::Inbox), 1000)
            .unwrap()
            .len(),
        50
    );
    assert!(store
        .search_scoped_at(&query, &scope(MailboxView::Snoozed), 1000)
        .unwrap()
        .is_empty());
    assert_eq!(
        store
            .search_scoped_at(&query, &scope(MailboxView::Folder("INBOX".into())), 999)
            .unwrap()
            .len(),
        50
    );
}

#[test]
fn scoped_semantic_ranking_handles_ties_orphans_and_missing_embeddings() {
    let store = fixture();
    let mut vector = vec![0; 384];
    vector[0] = 100;
    store.insert_message_embedding("a-006", &vector).unwrap();
    store.insert_message_embedding("a-009", &vector).unwrap();
    {
        let conn = store.conn.lock().unwrap();
        conn.execute("DELETE FROM vec_messages WHERE message_id='a-003'", [])
            .unwrap();
        conn.execute("INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES ('orphan','a',vec_int8(?1))",
            [crate::search::int8_slice_to_bytes(&vector)]).unwrap();
    }
    let mut query = query(SearchMode::Semantic, "", 0);
    query.limit = 1;
    for (offset, expected) in ["a-006", "a-009", "a-012"].into_iter().enumerate() {
        query.offset = offset;
        let rows = store
            .search_scoped(&query, &scope(MailboxView::Starred))
            .unwrap();
        assert_eq!(rows[0].id, expected);
    }
    query.mode = SearchMode::Hybrid;
    query.query_embedding_int8 = None;
    query.offset = 0;
    let rows = store
        .search_scoped(&query, &scope(MailboxView::Starred))
        .unwrap();
    assert_eq!(rows[0].id, "a-099");
    query.query_text = "absentword".into();
    assert!(store
        .search_scoped(&query, &scope(MailboxView::Starred))
        .unwrap()
        .is_empty());
}

#[test]
fn scoped_semantic_preserves_sqlite_null_distance_order_for_zero_vectors() {
    let store = fixture();
    store
        .insert_message_embedding("a-098", &vec![0; 384])
        .unwrap();
    let query = query(SearchMode::Semantic, "", 0);
    let rows = store
        .search_scoped(&query, &scope(MailboxView::Inbox))
        .unwrap();
    // INVARIANT: Existing scalar cosine returns SQL NULL for zero-norm stored vectors,
    // which SQLite sorts before finite distances. Candidate frontiers must not hide them.
    assert_eq!(rows[0].id, "a-098");
    let zero_query = HybridQuery {
        query_embedding_int8: Some(vec![0; 384]),
        ..query
    };
    let rows = store
        .search_scoped(&zero_query, &scope(MailboxView::Inbox))
        .unwrap();
    let expected: Vec<_> = (1..=100)
        .filter(|uid| eligible(*uid, &MailboxView::Inbox))
        .take(7)
        .map(|uid| format!("a-{uid:03}"))
        .collect();
    assert_eq!(
        rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        expected
    );
}
