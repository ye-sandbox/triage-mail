use crate::{DatabaseKey, HybridQuery, SearchEngine, SearchMode, SqliteMailStore};
fn store() -> SqliteMailStore {
    let s = SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([81; 32]), Default::default())
        .unwrap();
    s.transaction(|tx| {
        tx.execute_batch("INSERT INTO accounts VALUES('a','reader@example.com','synthetic',0,0);
            INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms)
            VALUES('one','a','INBOX',1,'Invoice','sender@example.org','reader@example.com',1,'',0),('two','a','INBOX',2,'Parcel','sender@example.org','reader@example.com',2,'',0);
            INSERT INTO message_folder_memberships SELECT account_id,folder,uid,id,0 FROM messages;").map_err(|e| crate::StorageError::Query(e.to_string()))?;
        Ok(())
    }).unwrap();
    s
}
fn query(mode: SearchMode) -> HybridQuery {
    HybridQuery {
        query_text: "Invoice".into(),
        mode,
        query_embedding_int8: if mode == SearchMode::Lexical {
            None
        } else {
            Some(vec![1; 384])
        },
        account_filter: None,
        limit: 10,
        offset: 0,
    }
}
#[test]
fn generation_cas_hydration_delete_failure_and_restart() {
    let s = store();
    s.insert_message_embedding("one", &[1; 384]).unwrap();
    s.begin_embedding_generation(triage_ai::onnx::GENERATION, false)
        .unwrap();
    assert_eq!(s.embedding_progress().unwrap().pending, 2);
    assert!(s.insert_message_embedding("one", &[1; 384]).is_err());
    assert!(s.search(&query(SearchMode::Semantic)).is_err());
    assert_eq!(s.search(&query(SearchMode::Lexical)).unwrap().len(), 1);
    let jobs = s.embedding_work(triage_ai::onnx::GENERATION, 2).unwrap();
    s.begin_embedding_generation(triage_ai::onnx::GENERATION, false)
        .unwrap(); // restart keeps the epoch
    assert!(s
        .finish_embedding(triage_ai::onnx::GENERATION, &jobs[0], Some(&[1; 384]))
        .unwrap());
    s.transaction(|tx| {
        tx.execute(
            "UPDATE messages SET text_body='hydrated' WHERE id='two'",
            [],
        )
        .map_err(|e| crate::StorageError::Query(e.to_string()))?;
        Ok(())
    })
    .unwrap();
    assert!(!s
        .finish_embedding(triage_ai::onnx::GENERATION, &jobs[1], Some(&[1; 384]))
        .unwrap());
    let job = s
        .embedding_work(triage_ai::onnx::GENERATION, 1)
        .unwrap()
        .remove(0);
    assert!(s
        .finish_embedding(triage_ai::onnx::GENERATION, &job, None)
        .unwrap());
    assert_eq!(s.embedding_progress().unwrap().failed, 1);
    assert!(s
        .embedding_work(triage_ai::onnx::GENERATION, 2)
        .unwrap()
        .is_empty());
    s.begin_embedding_generation(triage_ai::onnx::GENERATION, true)
        .unwrap();
    assert!(!s
        .finish_embedding(triage_ai::onnx::GENERATION, &job, Some(&[1; 384]))
        .unwrap());
    assert_eq!(s.embedding_progress().unwrap().failed, 0);
    let jobs = s.embedding_work(triage_ai::onnx::GENERATION, 2).unwrap();
    s.transaction(|tx| {
        tx.execute("DELETE FROM messages WHERE id='two'", [])
            .map_err(|e| crate::StorageError::Query(e.to_string()))?;
        Ok(())
    })
    .unwrap();
    assert!(!s
        .finish_embedding(triage_ai::onnx::GENERATION, &jobs[1], Some(&[1; 384]))
        .unwrap());
    assert!(s
        .finish_embedding(triage_ai::onnx::GENERATION, &jobs[0], Some(&[1; 384]))
        .unwrap());
    assert_eq!(s.embedding_progress().unwrap().pending, 0);
    assert_eq!(s.search(&query(SearchMode::Semantic)).unwrap().len(), 1);
    s.begin_embedding_generation("learned-v2", false).unwrap();
    assert!(!s
        .finish_embedding(triage_ai::onnx::GENERATION, &jobs[0], Some(&[1; 384]))
        .unwrap());
    assert_eq!(s.embedding_progress().unwrap().pending, 1);
    let other = s.embedding_work("learned-v2", 1).unwrap().remove(0);
    assert!(s
        .finish_embedding("learned-v2", &other, Some(&[1; 384]))
        .unwrap());
    assert_eq!(s.embedding_progress().unwrap().pending, 0);
    assert!(!s.embedding_ready().unwrap());
    assert!(s.search(&query(SearchMode::Semantic)).is_err());
}
#[test]
fn filtered_search_and_backup_cannot_reuse_partial_or_old_vectors() {
    use crate::{
        conversations::{ConversationQuery, ConversationSource},
        saved_views::SavedView,
        sender_review::ReviewState,
        MailboxScope, MailboxView,
    };
    let s = store();
    let saved = s
        .save_saved_view(SavedView {
            version: 1,
            id: "view".into(),
            revision: 0,
            name: "Invoice".into(),
            query: "Invoice".into(),
            mode: SearchMode::Semantic,
            accounts: vec![],
            scope: None,
            filters: Default::default(),
        })
        .unwrap();
    while !s.conversation_progress().unwrap().ready {
        s.backfill_conversations().unwrap();
    }
    s.begin_embedding_generation(triage_ai::onnx::GENERATION, false)
        .unwrap();
    for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
        let q = query(mode);
        assert!(s
            .search_scoped(
                &q,
                &MailboxScope::AccountView {
                    account_id: "a".into(),
                    view: MailboxView::Inbox
                }
            )
            .is_err());
        assert!(s
            .search_category(&q, None, triage_ai::TriageCategory::Financial)
            .is_err());
        assert!(s.search_sender_review(&q, ReviewState::Pending).is_err());
        assert!(s
            .search_conversations(
                &q,
                &ConversationQuery {
                    source: ConversationSource::Messages,
                    scope: None,
                    generation: None
                }
            )
            .is_err());
    }
    assert!(s
        .search_saved_view(&saved.id, saved.revision, Some(vec![1; 384]), 10, 0)
        .is_err());
    for job in s.embedding_work(triage_ai::onnx::GENERATION, 2).unwrap() {
        s.finish_embedding(triage_ai::onnx::GENERATION, &job, Some(&[1; 384]))
            .unwrap();
    }
    let backup = s
        .write_backup(Vec::new(), b"synthetic backup passphrase")
        .unwrap();
    let restored =
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([82; 32]), Default::default())
            .unwrap();
    restored
        .read_backup(backup.as_slice(), b"synthetic backup passphrase")
        .unwrap();
    restored
        .begin_embedding_generation(triage_ai::onnx::GENERATION, false)
        .unwrap();
    assert_eq!(restored.embedding_progress().unwrap().indexed, 0);
    assert_eq!(restored.embedding_progress().unwrap().pending, 2);
    assert!(restored.search(&query(SearchMode::Semantic)).is_err());
}

#[test]
fn encrypted_reopen_retains_progress_and_rejects_another_writer_stale_result() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("synthetic.db");
    let key = DatabaseKey::from_bytes([83; 32]);
    let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    let backup = store()
        .write_backup(Vec::new(), b"synthetic backup passphrase")
        .unwrap();
    s.read_backup(backup.as_slice(), b"synthetic backup passphrase")
        .unwrap();
    s.begin_embedding_generation(triage_ai::onnx::GENERATION, false)
        .unwrap();
    let jobs = s.embedding_work(triage_ai::onnx::GENERATION, 2).unwrap();
    s.finish_embedding(triage_ai::onnx::GENERATION, &jobs[0], Some(&[1; 384]))
        .unwrap();
    drop(s);
    let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    s.begin_embedding_generation(triage_ai::onnx::GENERATION, false)
        .unwrap();
    assert_eq!(s.embedding_progress().unwrap().indexed, 1);
    assert_eq!(s.embedding_progress().unwrap().pending, 1);
    let other = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    other
        .transaction(|tx| {
            tx.execute(
                "UPDATE messages SET subject='Replaced' WHERE id=?1",
                [&jobs[1].message_id],
            )
            .map_err(|e| crate::StorageError::Query(e.to_string()))?;
            Ok(())
        })
        .unwrap();
    assert!(!s
        .finish_embedding(triage_ai::onnx::GENERATION, &jobs[1], Some(&[1; 384]))
        .unwrap());
    assert!(SqliteMailStore::open(
        &path,
        &DatabaseKey::from_bytes([84; 32]),
        Default::default()
    )
    .is_err());
}
