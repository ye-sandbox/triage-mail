use super::*;
use crate::{DatabaseKey, FolderSyncState, MailboxScope, MailboxView, SearchMode};
fn fixture() -> SqliteMailStore {
    let s = SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([92; 32]), Default::default())
        .unwrap();
    s.transaction(|tx| {tx.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.com','synthetic',0,0); INSERT INTO folders VALUES ('a','INBOX','Inbox','/','inbox',1,NULL,NULL,0);").map_err(error)?;Ok(())}).unwrap();
    s
}
fn insert(s: &SqliteMailStore, id: &str, uid: u32, sender: &str, eligible: bool) {
    s.transaction(|tx| {tx.execute("INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms) VALUES (?1,'a','INBOX',?2,'Notice',?3,'reader@example.com',?2,'',0)",params![id,uid,sender]).map_err(error)?;
 tx.execute("INSERT INTO message_folder_memberships VALUES ('a','INBOX',?1,?2,0)",params![uid,id]).map_err(error)?;
 if eligible {arrival(tx,id,&FolderSyncState {account_id:"a".into(),folder_name:"INBOX".into(),uidvalidity:7,uidnext:uid+1,highest_modseq:None})?;} else {observe(tx,id,false)?;} Ok(())}).unwrap();
}
fn enable(s: &SqliteMailStore) {
    let mut p = s.review_preferences("a").unwrap();
    p.enabled = true;
    p.hide_in_inbox = true;
    s.save_review_preferences(p).unwrap();
    s.backfill_sender_review().unwrap();
}
fn query() -> HybridQuery {
    HybridQuery {
        query_text: "".into(),
        query_embedding_int8: None,
        account_filter: None,
        mode: SearchMode::Lexical,
        limit: 200,
        offset: 0,
    }
}
#[test]
fn history_frontiers_decisions_and_inbox_only_filtering() {
    let s = fixture();
    insert(&s, "old", 1, "History <HISTORY@example.org>", false);
    enable(&s);
    s.establish_review_frontier("a", "INBOX", 7, 10).unwrap();
    s.establish_review_frontier("a", "INBOX", 7, 100).unwrap();
    insert(&s, "history", 9, "late@example.org", true);
    insert(&s, "new", 10, "New <NEW@example.org>", true);
    assert_eq!(
        s.review_metadata("history").unwrap().unwrap().state,
        ReviewState::Known
    );
    let pending = s.review_metadata("new").unwrap().unwrap();
    assert_eq!(pending.sender, "new@example.org");
    assert_eq!(pending.state, ReviewState::Pending);
    assert_eq!(
        s.search_sender_review(&query(), ReviewState::Pending)
            .unwrap()
            .len(),
        1
    );
    let scope = MailboxScope::AccountView {
        account_id: "a".into(),
        view: MailboxView::Inbox,
    };
    assert_eq!(s.search_scoped(&query(), &scope).unwrap().len(), 2);
    let accepted = s
        .save_sender_choice(SenderChoice {
            state: ReviewState::Accepted,
            ..pending.clone()
        })
        .unwrap();
    assert!(s.save_sender_choice(pending).is_err());
    assert_eq!(s.search_scoped(&query(), &scope).unwrap().len(), 3);
    let declined = s
        .save_sender_choice(SenderChoice {
            state: ReviewState::Declined,
            ..accepted
        })
        .unwrap();
    assert_eq!(
        s.search_sender_review(&query(), ReviewState::Declined)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        s.search_scoped(&query(), &MailboxScope::GlobalAllMail)
            .unwrap()
            .len(),
        3
    );
    s.transaction(|tx| {
        tx.execute("DELETE FROM messages WHERE id='new'", [])
            .map_err(error)?;
        Ok(())
    })
    .unwrap();
    insert(&s, "again", 11, "new@example.org", true);
    assert_eq!(s.review_metadata("again").unwrap(), Some(declined));
}
#[test]
fn disabled_initialization_unknown_and_reset_seed_conservatively() {
    let s = fixture();
    enable(&s);
    s.establish_review_frontier("a", "INBOX", 7, 0).unwrap();
    insert(&s, "unknown", 1, "one@example.org", true);
    assert_eq!(
        s.review_metadata("unknown").unwrap().unwrap().state,
        ReviewState::Known
    );
    s.establish_review_frontier("a", "INBOX", 7, 2).unwrap();
    insert(&s, "pending", 2, "two@example.org", true);
    assert_eq!(
        s.review_metadata("pending").unwrap().unwrap().state,
        ReviewState::Pending
    );
    s.establish_review_frontier("a", "INBOX", 8, 100).unwrap();
    insert(&s, "reset", 3, "three@example.org", true);
    assert_eq!(
        s.review_metadata("reset").unwrap().unwrap().state,
        ReviewState::Known
    );
    let mut p = s.review_preferences("a").unwrap();
    p.enabled = false;
    s.save_review_preferences(p.clone()).unwrap();
    assert!(s.save_review_preferences(p).is_err());
    insert(&s, "off", 4, "four@example.org", true);
    assert_eq!(
        s.review_metadata("off").unwrap().unwrap().state,
        ReviewState::Known
    );
    enable(&s);
    assert_eq!(
        s.review_metadata("pending").unwrap().unwrap().state,
        ReviewState::Pending
    );
    insert(&s, "invalid", 5, "Name only", false);
    assert_eq!(s.review_metadata("invalid").unwrap(), None);
    assert!(s.review_preferences("missing").is_err());
    assert!(s.list_sender_choices("b", 0).unwrap().is_empty());
}
#[test]
fn bounded_history_seed_survives_restart_and_does_not_overwrite_pending() {
    let s = fixture();
    s.transaction(|tx|{tx.execute_batch("WITH RECURSIVE n(i) AS(VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<601) INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms) SELECT 'seed-'||i,'a','INBOX',i,'History','seed@example.org','reader@example.com',i,'',0 FROM n").map_err(error)?;Ok(())}).unwrap();
    let mut p = s.review_preferences("a").unwrap();
    p.enabled = true;
    s.save_review_preferences(p).unwrap();
    assert_eq!(s.backfill_sender_review().unwrap(), 300);
    assert!(s.review_preferences("a").unwrap().initializing);
    assert_eq!(s.backfill_sender_review().unwrap(), 300);
    assert_eq!(s.backfill_sender_review().unwrap(), 1);
    assert!(!s.review_preferences("a").unwrap().initializing);
    assert_eq!(s.backfill_sender_review().unwrap(), 0);
    assert_eq!(s.list_sender_choices("a", 0).unwrap().len(), 1);
}

#[test]
fn exact_review_pages_filter_before_all_ranking_windows_and_explicit_views_bypass_hiding() {
    let s = fixture();
    enable(&s);
    s.establish_review_frontier("a", "INBOX", 7, 1).unwrap();
    for uid in 1..=220 {
        insert(
            &s,
            &format!("m-{uid:03}"),
            uid,
            if uid % 2 == 0 {
                "pending@example.org"
            } else {
                "known@example.org"
            },
            uid % 2 == 0,
        );
        s.insert_message_embedding(&format!("m-{uid:03}"), &vec![1; 384])
            .unwrap();
    }
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let mut q = query();
        q.limit = 50;
        q.mode = mode;
        if mode != SearchMode::Lexical {
            q.query_embedding_int8 = Some(vec![1; 384]);
            q.query_text = "Notice".into();
        }
        let mut ids = vec![];
        loop {
            let rows = s.search_sender_review(&q, ReviewState::Pending).unwrap();
            ids.extend(rows.iter().map(|r| r.id.clone()));
            if rows.len() < 50 {
                break;
            }
            q.offset += 50;
        }
        let expected = (1..=220)
            .filter(|i| i % 2 == 0)
            .map(|i| format!("m-{i:03}"))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), 110);
        assert_eq!(
            ids.into_iter().collect::<std::collections::HashSet<_>>(),
            expected
        );
    }
    let scope = MailboxScope::AccountView {
        account_id: "a".into(),
        view: MailboxView::Inbox,
    };
    let view = crate::saved_views::SavedView {
        version: 1,
        id: "all-inbox".into(),
        revision: 0,
        name: "Explicit inbox".into(),
        query: "".into(),
        mode: SearchMode::Lexical,
        accounts: vec![],
        scope: Some(scope.clone()),
        filters: Default::default(),
    };
    let view = s.save_saved_view(view).unwrap();
    assert_eq!(
        s.search_saved_view(&view.id, view.revision, None, 500, 0)
            .unwrap()
            .len(),
        220
    );
    assert_eq!(
        s.search_scoped(
            &HybridQuery {
                limit: 500,
                ..query()
            },
            &scope
        )
        .unwrap()
        .len(),
        110
    );
    assert!(s
        .search_sender_review(
            &HybridQuery {
                account_filter: Some("missing".into()),
                ..query()
            },
            ReviewState::Pending
        )
        .is_err());
}

#[test]
fn choices_and_partial_history_survive_encrypted_reopen_and_account_removal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.db");
    let key = DatabaseKey::from_bytes([93; 32]);
    {
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        s.transaction(|tx|{tx.execute_batch("INSERT INTO accounts VALUES ('a','reader@example.com','synthetic',0,0); WITH RECURSIVE n(i) AS(VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<301) INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms) SELECT 'm-'||i,'a','INBOX',i,'History','seed@example.org','reader@example.com',i,'',0 FROM n").map_err(error)?;Ok(())}).unwrap();
        let mut p = s.review_preferences("a").unwrap();
        p.enabled = true;
        s.save_review_preferences(p).unwrap();
        assert_eq!(s.backfill_sender_review().unwrap(), 300);
        let choice = s.list_sender_choices("a", 0).unwrap().remove(0);
        s.save_sender_choice(SenderChoice {
            state: ReviewState::Declined,
            ..choice
        })
        .unwrap();
    }
    let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    assert!(s.review_preferences("a").unwrap().initializing);
    s.backfill_sender_review().unwrap();
    assert!(!s.review_preferences("a").unwrap().initializing);
    assert_eq!(
        s.list_sender_choices("a", 0).unwrap()[0].state,
        ReviewState::Declined
    );
    s.transaction(|tx| {
        tx.execute("DELETE FROM accounts WHERE id='a'", [])
            .map_err(error)?;
        let count: u32 = tx
            .query_row("SELECT count(*) FROM sender_review_choices", [], |r| {
                r.get(0)
            })
            .map_err(error)?;
        assert_eq!(count, 0);
        Ok(())
    })
    .unwrap();
}

fn record(id: &str, uid: u32, sender: &str) -> crate::MessageRecord {
    crate::MessageRecord {
        id: id.into(),
        account_id: "a".into(),
        folder: "INBOX".into(),
        uid,
        message_id_header: Some(format!("{id}@example.org")),
        in_reply_to: None,
        references_header: None,
        subject: "Notice".into(),
        sender: sender.into(),
        recipients: "reader@example.com".into(),
        received_at_epoch_ms: i64::from(uid),
        is_read: false,
        has_attachments: false,
        category_badge: None,
        triage_badges: vec![],
        raw_headers: String::new(),
        text_body: None,
        html_body: None,
        list_unsubscribe: None,
        list_unsubscribe_post: None,
        one_click_unsubscribe_supported: false,
        created_at_epoch_ms: 0,
        attachments: vec![],
        tags: vec![],
        is_starred: false,
        snooze_until_epoch_ms: None,
        provider_message_id: None,
    }
}
#[test]
fn sync_straddles_frontier_atomically_and_duplicate_membership_never_requeues_review() {
    let s = fixture();
    enable(&s);
    s.establish_review_frontier("a", "INBOX", 7, 3).unwrap();
    let state = FolderSyncState {
        account_id: "a".into(),
        folder_name: "INBOX".into(),
        uidvalidity: 7,
        uidnext: 5,
        highest_modseq: None,
    };
    s.commit_sync_batch(
        &[
            record("old", 2, "old@example.org"),
            record("new", 3, "new@example.org"),
        ],
        &[],
        &state,
        false,
    )
    .unwrap();
    assert_eq!(
        s.review_metadata("old").unwrap().unwrap().state,
        ReviewState::Known
    );
    assert_eq!(
        s.review_metadata("new").unwrap().unwrap().state,
        ReviewState::Pending
    );
    let mut moved = record("copy", 70, "new@example.org");
    moved.folder = "Archive".into();
    moved.message_id_header = Some("new@example.org".into());
    moved.received_at_epoch_ms = 3;
    s.commit_sync_batch(
        &[moved],
        &[],
        &FolderSyncState {
            folder_name: "Archive".into(),
            ..state.clone()
        },
        false,
    )
    .unwrap();
    assert_eq!(
        s.review_metadata("new").unwrap().unwrap().state,
        ReviewState::Pending
    );
    let mut bad = record("bad", 5, "bad@example.org");
    bad.account_id = "missing".into();
    assert!(s
        .commit_sync_batch(
            &[record("rollback", 4, "rollback@example.org"), bad],
            &[],
            &state,
            false
        )
        .is_err());
    assert!(s.review_metadata("rollback").is_err());
    assert!(!s
        .list_sender_choices("a", 0)
        .unwrap()
        .iter()
        .any(|v| v.sender == "rollback@example.org"));
}
