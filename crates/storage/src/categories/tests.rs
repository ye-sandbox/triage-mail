use super::*;
use crate::{DatabaseKey, SearchMode};
fn seed(store: &SqliteMailStore) {
    store.transaction(|tx| {
        tx.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.org','synthetic',0,0);
        WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<601)
        INSERT INTO messages(id,account_id,folder,uid,message_id_header,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms)
        SELECT printf('m-%04d',i),CASE WHEN i=601 THEN 'b' ELSE 'a' END,'INBOX',i,'<id-'||i||'@example.com>',CASE WHEN i%2=0 THEN 'Invoice' ELSE 'Notification' END,'Sender <sender@example.org>','reader@example.com',i,'',0 FROM n;
        INSERT INTO message_folder_memberships SELECT account_id,'INBOX',uid,id,0 FROM messages;").map_err(error)?;
        Ok(())
    }).unwrap();
}
fn fixture() -> SqliteMailStore {
    let store =
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([71; 32]), Default::default())
            .unwrap();
    seed(&store);
    store
}
#[test]
fn precedence_reset_isolation_and_revisions() {
    let store = fixture();
    let initial = store.category_metadata("m-0001").unwrap();
    assert_eq!(initial.effective, TriageCategory::Notifications);
    let value = SenderOverride {
        account_id: "a".into(),
        sender: "sender@example.org".into(),
        revision: 0,
        category: Some(TriageCategory::Shopping),
    };
    let saved = store.save_category_override(value.clone()).unwrap();
    assert!(store.save_category_override(value).is_err());
    assert_eq!(
        store.category_metadata("m-0001").unwrap().source,
        CategorySource::Sender
    );
    assert_eq!(
        store.category_metadata("m-0601").unwrap().effective,
        TriageCategory::Notifications
    );
    let manual = store
        .correct_category("m-0001", 0, Some(TriageCategory::Personal))
        .unwrap();
    assert_eq!(manual.source, CategorySource::Message);
    store
        .transaction(|tx| {
            tx.execute(
                "UPDATE messages SET subject='Password reset' WHERE id='m-0001'",
                [],
            )
            .map_err(error)?;
            Ok(())
        })
        .unwrap();
    let refreshed = store.category_metadata("m-0001").unwrap();
    assert_eq!(refreshed.automatic, TriageCategory::Security);
    assert_eq!(refreshed.effective, TriageCategory::Personal);
    assert!(store.correct_category("m-0001", 0, None).is_err());
    assert_eq!(
        store.correct_category("m-0001", 1, None).unwrap().effective,
        TriageCategory::Shopping
    );
    store
        .save_category_override(SenderOverride {
            category: None,
            ..saved
        })
        .unwrap();
    assert_eq!(
        store.category_metadata("m-0001").unwrap().effective,
        TriageCategory::Security
    );
    assert!(store
        .correct_category("m-0001", 0, Some(TriageCategory::Personal))
        .is_err());
}
#[test]
fn bounded_backfill_full_result_pages_and_saved_filter() {
    let store = fixture();
    assert_eq!(store.backfill_categories(200).unwrap(), 200);
    assert_eq!(store.backfill_categories(200).unwrap(), 200);
    assert_eq!(store.backfill_categories(200).unwrap(), 200);
    assert_eq!(store.backfill_categories(200).unwrap(), 1);
    assert_eq!(store.backfill_categories(200).unwrap(), 0);
    let mut query = HybridQuery {
        query_text: String::new(),
        mode: SearchMode::Lexical,
        query_embedding_int8: None,
        account_filter: Some("a".into()),
        limit: 200,
        offset: 0,
    };
    let first = store
        .search_category(&query, None, TriageCategory::Financial)
        .unwrap();
    assert_eq!(first.len(), 200);
    query.offset = 200;
    let second = store
        .search_category(&query, None, TriageCategory::Financial)
        .unwrap();
    assert_eq!(second.len(), 100);
    assert!(first.iter().all(|r| !second.iter().any(|s| s.id == r.id)));
    assert!(first
        .iter()
        .chain(&second)
        .all(|r| r.uid % 2 == 0 && r.category_badge.as_deref() == Some("Financial")));
    let old: crate::saved_views::ViewFilters=serde_json::from_str(r#"{"is_read":null,"is_starred":null,"has_attachments":null,"tags":[],"sender":null,"sender_domain":null,"extraction":null,"due_from":null,"due_to":null,"upcoming_days":null}"#).unwrap();
    assert_eq!(old.category, None);
}
#[test]
fn durable_identity_reopen_move_and_cache_reingestion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mail.db");
    let key = DatabaseKey::from_bytes([72; 32]);
    let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    seed(&store);
    store
        .correct_category("m-0001", 0, Some(TriageCategory::Shopping))
        .unwrap();
    store
        .transaction(|tx| {
            tx.execute(
                "UPDATE messages SET folder='Archive',uid=700 WHERE id='m-0001'",
                [],
            )
            .map_err(error)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        store.category_metadata("m-0001").unwrap().effective,
        TriageCategory::Shopping
    );
    store.transaction(|tx| {
        tx.execute("DELETE FROM messages WHERE id='m-0001'",[]).map_err(error)?;
        tx.execute_batch("INSERT INTO messages(id,account_id,folder,uid,message_id_header,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms) VALUES ('replacement','a','Archive',700,'id-1@example.com','Notification','sender@example.org','reader@example.com',1,'',0)").map_err(error)?;Ok(())
    }).unwrap();
    drop(store);
    let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    assert_eq!(
        store.category_metadata("replacement").unwrap().effective,
        TriageCategory::Shopping
    );
    store
        .transaction(|tx| {
            tx.execute(
                "UPDATE messages SET message_id_header=NULL WHERE id='m-0003'",
                [],
            )
            .map_err(error)?;
            Ok(())
        })
        .unwrap();
    assert!(
        !store
            .correct_category("m-0003", 0, Some(TriageCategory::Shopping))
            .unwrap()
            .durable
    );
    store
        .transaction(|tx| {
            tx.execute("DELETE FROM messages WHERE id='m-0003'", [])
                .map_err(error)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        store
            .transaction(|tx| tx
                .query_row(
                    "SELECT count(*) FROM category_corrections WHERE category='Shopping'",
                    [],
                    |r| r.get::<_, u32>(0)
                )
                .map_err(error))
            .unwrap(),
        1
    );
}
#[test]
fn malformed_senders_and_missing_accounts_fail_closed() {
    let store = fixture();
    for sender in [
        "Display Name",
        "a@example.com, b@example.org",
        "Display <a@example.com>",
    ] {
        assert!(store
            .save_category_override(SenderOverride {
                account_id: "a".into(),
                sender: sender.into(),
                revision: 0,
                category: Some(TriageCategory::Shopping)
            })
            .is_err());
    }
    assert!(store
        .save_category_override(SenderOverride {
            account_id: "missing".into(),
            sender: "sender@example.org".into(),
            revision: 0,
            category: None
        })
        .is_err());
}

#[test]
fn sync_upsert_preserves_correction_and_badges_without_outbox_work() {
    let store = fixture();
    store
        .correct_category("m-0002", 0, Some(TriageCategory::Personal))
        .unwrap();
    let mut record = store.get_message_by_id("m-0002").unwrap().unwrap();
    record.subject = "Password reset notification".into();
    record.triage_badges = vec![TriageBadge::OtpCode {
        code: "123456".into(),
    }];
    store.insert_message(&record).unwrap();
    let value = store.category_metadata("m-0002").unwrap();
    assert_eq!(value.effective, TriageCategory::Personal);
    assert_eq!(value.automatic, TriageCategory::Security);
    assert_eq!(
        store
            .get_message_by_id("m-0002")
            .unwrap()
            .unwrap()
            .triage_badges,
        record.triage_badges
    );
    let counts=store.transaction(|tx|tx.query_row("SELECT (SELECT count(*) FROM pending_mutations)+(SELECT count(*) FROM rule_jobs)+(SELECT count(*) FROM alert_jobs)+(SELECT count(*) FROM reminder_jobs)",[],|r|r.get::<_,u32>(0)).map_err(error)).unwrap();
    assert_eq!(counts, 0);
}

#[test]
fn semantic_hybrid_prefiltering_account_scope_and_saved_category() {
    let store = fixture();
    while store.backfill_categories(300).unwrap() > 0 {}
    store.transaction(|tx| {
        for i in 1..=601 {
            tx.execute("INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES (?1,?2,vec_int8(?3))",params![format!("m-{i:04}"),if i==601 {"b"} else {"a"},vec![1_u8;384]]).map_err(error)?;
        }
        Ok(())
    }).unwrap();
    let scope = MailboxScope::AccountView {
        account_id: "a".into(),
        view: crate::MailboxView::Starred,
    };
    for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
        let query = HybridQuery {
            query_text: "invoice".into(),
            mode,
            query_embedding_int8: Some(vec![1; 384]),
            account_filter: Some("a".into()),
            limit: 200,
            offset: 0,
        };
        let first = store
            .search_category(&query, None, TriageCategory::Financial)
            .unwrap();
        let second = store
            .search_category(
                &HybridQuery {
                    offset: 200,
                    ..query.clone()
                },
                None,
                TriageCategory::Financial,
            )
            .unwrap();
        assert_eq!(first.len(), 200);
        assert_eq!(second.len(), 100);
        let expected = (1..=300)
            .map(|i| format!("m-{:04}", i * 2))
            .collect::<Vec<_>>();
        assert_eq!(
            first
                .iter()
                .chain(&second)
                .map(|r| r.id.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(store
            .search_category(&query, Some(&scope), TriageCategory::Financial)
            .unwrap()
            .is_empty());
        let saved = store
            .save_saved_view(crate::saved_views::SavedView {
                version: 1,
                id: format!("{mode:?}"),
                revision: 0,
                name: "Finance".into(),
                query: query.query_text.clone(),
                mode,
                accounts: vec!["a".into()],
                scope: None,
                filters: crate::saved_views::ViewFilters {
                    category: Some(TriageCategory::Financial),
                    ..Default::default()
                },
            })
            .unwrap();
        assert_eq!(
            store
                .search_saved_view(&saved.id, saved.revision, Some(vec![1; 384]), 200, 0)
                .unwrap(),
            first
        );
    }
}

#[test]
fn interrupted_backfill_rolls_back_and_account_deletion_cleans_overrides() {
    let store = fixture();
    let failed: Result<(), StorageError> = store.transaction(|tx| {
        refresh(tx, "m-0001")?;
        Err(invalid())
    });
    assert!(failed.is_err());
    assert_eq!(
        store
            .transaction(|tx| tx
                .query_row("SELECT count(*) FROM message_categories", [], |r| r
                    .get::<_, u32>(0))
                .map_err(error))
            .unwrap(),
        0
    );
    assert_eq!(store.backfill_categories(1).unwrap(), 1);
    store
        .save_category_override(SenderOverride {
            account_id: "a".into(),
            sender: "sender@example.org".into(),
            revision: 0,
            category: Some(TriageCategory::Shopping),
        })
        .unwrap();
    assert_eq!(store.list_category_overrides(0).unwrap().len(), 1);
    store
        .transaction(|tx| {
            tx.execute("DELETE FROM accounts WHERE id='a'", [])
                .map_err(error)?;
            Ok(())
        })
        .unwrap();
    assert!(store.list_category_overrides(0).unwrap().is_empty());
}

#[test]
fn malformed_header_is_not_a_durable_identity_and_provider_id_wins() {
    let store = fixture();
    store.transaction(|tx| {
        tx.execute("UPDATE messages SET message_id_header='broken' WHERE id='m-0001'",[]).map_err(error)?;
        tx.execute("UPDATE messages SET message_id_header='broken',provider_message_id='provider-2' WHERE id='m-0002'",[]).map_err(error)?;
        Ok(())
    }).unwrap();
    assert!(!store.category_metadata("m-0001").unwrap().durable);
    assert!(store.category_metadata("m-0002").unwrap().durable);
}
