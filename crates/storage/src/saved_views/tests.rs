use super::*;
use crate::{DatabaseKey, MailboxScope, MailboxView};

fn fixture() -> SqliteMailStore {
    let store =
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([61; 32]), Default::default())
            .unwrap();
    seed(&store);
    store
}
fn seed(store: &SqliteMailStore) {
    let conn = store.conn.lock().unwrap();
    conn.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.org','synthetic',0,0);
    INSERT INTO folders(account_id,folder_name,display_name,role,is_selectable,updated_at_epoch_ms) VALUES ('a','INBOX','Inbox','inbox',1,0);
    WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<600)
    INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms,is_read,is_starred,has_attachments,triage_badges)
    SELECT printf('m-%04d',i),CASE WHEN i%2=0 THEN 'a' ELSE 'b' END,'INBOX',i,'invoice','Client <client@example.org>','reader@example.com',i,'',0,i%3=0,i%4=0,i%5=0,
    '[{\"FebrabanBoleto\":{\"digitable_line\":\"synthetic\",\"amount_cents\":1200,\"due_date_iso\":\"2026-11-02\"}}]' FROM n;
    INSERT INTO message_folder_memberships SELECT account_id,'INBOX',uid,id,0 FROM messages;
    INSERT INTO message_folder_memberships SELECT account_id,'All Mail',uid+600,id,0 FROM messages;
    INSERT INTO tags SELECT id||'-tag',id,'client',0 FROM messages WHERE uid%7=0;").unwrap();
    for i in 1..=600 {
        conn.execute("INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES (?1,?2,vec_int8(?3))", params![format!("m-{i:04}"),if i%2==0 {"a"} else {"b"},vec![1_u8;384]]).unwrap();
    }
}
fn view() -> SavedView {
    SavedView {
        version: 1,
        id: "view".into(),
        revision: 0,
        name: "Clients".into(),
        query: String::new(),
        mode: SearchMode::Lexical,
        accounts: vec!["a".into()],
        scope: None,
        filters: ViewFilters::default(),
    }
}
fn ids(rows: Vec<MessageRowSummary>) -> Vec<String> {
    rows.into_iter().map(|r| r.id).collect()
}
const NOW: i64 = 1793577600000; // 2026-11-02 00:00 UTC
fn search(
    store: &SqliteMailStore,
    view: &SavedView,
    offset: usize,
    now: i64,
) -> Vec<MessageRowSummary> {
    store
        .search_saved_view_at(
            &view.id,
            view.revision,
            Some(vec![1; 384]),
            (200, offset),
            now,
        )
        .unwrap()
}

#[test]
fn encrypted_reopen_revision_conflicts_and_deleted_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mail.db");
    let key = DatabaseKey::from_bytes([62; 32]);
    let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    seed(&store);
    let saved = store.save_saved_view(view()).unwrap();
    assert!(store.save_saved_view(view()).is_err());
    let mut renamed = saved.clone();
    renamed.name = "Renamed".into();
    let renamed = store.save_saved_view(renamed).unwrap();
    assert!(store
        .search_saved_view(&saved.id, saved.revision, None, 200, 0)
        .is_err());
    drop(store);
    assert!(!std::fs::read(&path)
        .unwrap()
        .windows(7)
        .any(|w| w == b"Renamed"));
    let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    assert_eq!(store.list_saved_views().unwrap(), vec![renamed.clone()]);
    assert!(store.delete_saved_view("view", 1).is_err());
    store.delete_saved_view("view", renamed.revision).unwrap();
    assert!(store.save_saved_view(view()).is_err());
}

#[test]
fn full_result_account_membership_pages_and_mutation_refresh() {
    let store = fixture();
    let mut v = view();
    v.filters.is_read = Some(false);
    v.filters.tags = vec!["client".into()];
    let v = store.save_saved_view(v).unwrap();
    let expected = (1..=600)
        .rev()
        .filter(|i| i % 2 == 0 && i % 3 != 0 && i % 7 == 0)
        .map(|i| format!("m-{i:04}"))
        .collect::<Vec<_>>();
    let rows = search(&store, &v, 0, NOW);
    assert_eq!(ids(rows.clone()), expected);
    assert!(rows
        .iter()
        .all(|r| r.folder == "INBOX" && r.account_id == "a"));
    store
        .conn
        .lock()
        .unwrap()
        .execute("UPDATE messages SET is_read=1 WHERE id=?1", [&expected[0]])
        .unwrap();
    assert_eq!(ids(search(&store, &v, 0, NOW)), expected[1..]);
    let mut all = view();
    all.id = "all".into();
    all.accounts.clear();
    let all = store.save_saved_view(all).unwrap();
    let combined = (0..3)
        .flat_map(|p| ids(search(&store, &all, p * 200, NOW)))
        .collect::<Vec<_>>();
    assert_eq!(
        combined,
        (1..=600)
            .rev()
            .map(|i| format!("m-{i:04}"))
            .collect::<Vec<_>>()
    );
    store
        .conn
        .lock()
        .unwrap()
        .execute_batch("DELETE FROM accounts WHERE id='a'")
        .unwrap();
    assert_eq!(store.list_saved_views().unwrap().len(), 2);
    assert!(store
        .search_saved_view("view", v.revision, None, 200, 0)
        .is_err());
    assert!(store.save_saved_view(v.clone()).is_err());
    store.delete_saved_view("view", v.revision).unwrap();
}

#[test]
fn every_ranking_branch_filters_before_candidates_and_matches_singleton_oracle() {
    let store = fixture();
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let mut v = view();
        v.id = format!("{mode:?}");
        v.mode = mode;
        v.query = "invoice".into();
        v.filters.tags = vec!["client".into()];
        v.filters.is_starred = Some(true);
        let v = store.save_saved_view(v).unwrap();
        let first = search(&store, &v, 0, NOW);
        let actual = ids(first);
        assert_eq!(actual.len(), 21); // 28,56,...588
        assert!(actual
            .iter()
            .all(|id| id[2..].parse::<u32>().unwrap() % 28 == 0));
        let page = store
            .search_saved_view_at(&v.id, v.revision, Some(vec![1; 384]), (7, 7), NOW)
            .unwrap();
        assert_eq!(ids(page), actual[7..14]);
        // All equal positive vectors have cosine zero; canonical ID resolves semantic ties.
        if mode == SearchMode::Semantic {
            assert_eq!(
                actual,
                (1..=21)
                    .map(|i| format!("m-{:04}", i * 28))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn scope_and_effective_folder_overrides_preserve_canonical_identity() {
    let store = fixture();
    let mut v = view();
    v.scope = Some(MailboxScope::AccountView {
        account_id: "a".into(),
        view: MailboxView::Inbox,
    });
    let v = store.save_saved_view(v).unwrap();
    assert_eq!(search(&store, &v, 0, NOW).len(), 200);
    store
        .conn
        .lock()
        .unwrap()
        .execute_batch("INSERT INTO message_folder_overrides VALUES ('m-0600','Archive')")
        .unwrap();
    assert!(!ids(search(&store, &v, 0, NOW)).contains(&"m-0600".into()));
    let mut stale = v;
    stale.accounts = vec!["b".into()];
    assert!(store.save_saved_view(stale).is_err());
}

#[test]
fn parsed_sender_and_extraction_windows_fail_closed_at_utc_boundaries() {
    let mut f = ViewFilters {
        sender: Some("client@example.org".into()),
        sender_domain: Some("example.org".into()),
        extraction: Some(ExtractionKind::Boleto),
        upcoming_days: Some(30),
        ..Default::default()
    };
    let badge = r#"[{"FebrabanBoleto":{"digitable_line":"synthetic","amount_cents":null,"due_date_iso":"2026-11-02"}}]"#;
    assert!(facts_match(
        &f,
        "Name <client@example.org>",
        "",
        badge,
        "2026-10-03"
    ));
    assert!(!facts_match(
        &f,
        "Name <client@example.org>",
        "",
        badge,
        "2026-10-02"
    ));
    assert!(!facts_match(
        &f,
        "client@example.org <imposter@example.com>",
        "",
        badge,
        "2026-11-02"
    ));
    assert!(!facts_match(
        &f,
        "client@example.org,other@example.org",
        "",
        badge,
        "2026-11-02"
    ));
    assert!(!facts_match(
        &f,
        "client@example.org",
        "",
        "[]",
        "2026-11-02"
    ));
    f.due_from = Some("2026-11-03".into());
    assert!(!facts_match(
        &f,
        "client@example.org",
        "",
        badge,
        "2026-11-02"
    ));
    let store = fixture();
    let mut v = view();
    v.filters.upcoming_days = Some(30);
    v.filters.has_attachments = Some(true);
    let v = store.save_saved_view(v).unwrap();
    assert_eq!(search(&store, &v, 0, NOW).len(), 60);
    assert!(search(&store, &v, 0, NOW + 86400000).is_empty());
    assert_eq!(search(&store, &v, 0, NOW - 1).len(), 60);
}

#[test]
fn malformed_definitions_are_rejected_without_writes() {
    let store = fixture();
    let mut v = view();
    v.filters.due_from = Some("2026-02-30".into());
    assert!(store.save_saved_view(v).is_err());
    let mut v = view();
    v.accounts = vec!["a".into(), "a".into()];
    assert!(store.save_saved_view(v).is_err());
    let mut v = view();
    v.version = 2;
    assert!(store.save_saved_view(v).is_err());
    assert!(store.list_saved_views().unwrap().is_empty());
    assert!(store.search_saved_view("unknown", 1, None, 0, 0).is_err());
}

#[test]
fn recipient_filters_match_normalized_forwarded_addresses() {
    let f = ViewFilters {
        recipient: Some("alias@example.com".into()),
        recipient_domain: Some("example.com".into()),
        ..Default::default()
    };
    let to = "Reader <reader@example.org>, Alias <Alias@Example.com>";
    assert!(facts_match(&f, "a@example.org", to, "[]", "2026-11-02"));
    assert!(!facts_match(
        &f,
        "a@example.org",
        "reader@example.org",
        "[]",
        "2026-11-02"
    ));
    assert!(!facts_match(&f, "a@example.org", "", "[]", "2026-11-02"));
    let old: ViewFilters = serde_json::from_str(r#"{"tags":[],"is_read":null,"is_starred":null,"has_attachments":null,"sender":null,"sender_domain":null,"extraction":null,"due_from":null,"due_to":null,"upcoming_days":null}"#).unwrap();
    assert!(old.recipient.is_none() && old.recipient_domain.is_none());
    let store = fixture();
    let mut v = view();
    v.filters.recipient = Some("Bad Name <x@example.com>".into());
    assert!(store.save_saved_view(v).is_err());
}
