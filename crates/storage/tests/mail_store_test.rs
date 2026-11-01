use std::time::Instant;

use storage::{
    AccountRecord, AttachmentRecord, DatabaseKey, FolderSyncState, FtsTokenizerMode, HybridQuery,
    MailRepository, MessageRecord, SearchEngine, SearchMode, SqliteMailStore,
};

fn create_sample_account(id: &str, email: &str) -> AccountRecord {
    AccountRecord {
        id: id.to_string(),
        email: email.to_string(),
        auth_type: "app_password".to_string(),
        created_at_epoch_ms: 1700000000000,
        updated_at_epoch_ms: 1700000000000,
    }
}

fn create_sample_message(
    id: &str,
    account_id: &str,
    folder: &str,
    uid: u32,
    subject: &str,
    body: &str,
) -> MessageRecord {
    MessageRecord {
        id: id.to_string(),
        account_id: account_id.to_string(),
        folder: folder.to_string(),
        uid,
        message_id_header: Some(format!("<{id}@example.com>")),
        in_reply_to: None,
        references_header: None,
        subject: subject.to_string(),
        sender: "billing@example.org".to_string(),
        recipients: "user@example.com".to_string(),
        received_at_epoch_ms: 1700000100000,
        is_read: false,
        has_attachments: true,
        category_badge: Some("finance".to_string()),
        triage_badges: vec![],
        raw_headers: "From: billing@example.org\r\nSubject: Test\r\n".to_string(),
        text_body: Some(body.to_string()),
        html_body: Some(format!("<p>{body}</p>")),
        list_unsubscribe: Some("<https://example.org/unsub>".to_string()),
        list_unsubscribe_post: Some("List-Unsubscribe=One-Click".to_string()),
        one_click_unsubscribe_supported: true,
        created_at_epoch_ms: 1700000100000,
        attachments: vec![AttachmentRecord {
            id: format!("att_{id}"),
            message_id: id.to_string(),
            part_id: "2".to_string(),
            filename: "invoice.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 42000,
            content_id: None,
            is_inline: false,
        }],
        tags: vec!["boleto".to_string(), "priority".to_string()],
    }
}

#[test]
fn test_folder_sync_state_roundtrip() {
    let key = DatabaseKey::from_bytes([11u8; 32]);
    let store =
        SqliteMailStore::open_in_memory(&key, FtsTokenizerMode::Unicode61RemoveDiacritics).unwrap();

    let account = create_sample_account("acc_1", "user1@example.com");
    store.insert_account(&account).unwrap();

    let initial = store.get_folder_sync_state("acc_1", "INBOX").unwrap();
    assert_eq!(initial, None);

    let state = FolderSyncState {
        account_id: "acc_1".to_string(),
        folder_name: "INBOX".to_string(),
        uidvalidity: 12345,
        uidnext: 6789,
        highest_modseq: Some(99999),
    };
    store.upsert_folder_sync_state(&state).unwrap();

    let fetched = store.get_folder_sync_state("acc_1", "INBOX").unwrap();
    assert_eq!(fetched, Some(state.clone()));

    // Test upsert update
    let updated = FolderSyncState {
        uidnext: 6790,
        highest_modseq: Some(100005),
        ..state
    };
    store.upsert_folder_sync_state(&updated).unwrap();

    let fetched2 = store.get_folder_sync_state("acc_1", "INBOX").unwrap();
    assert_eq!(fetched2, Some(updated));
}

#[test]
fn test_message_persistence_and_retrieval() {
    let key = DatabaseKey::from_bytes([22u8; 32]);
    let store =
        SqliteMailStore::open_in_memory(&key, FtsTokenizerMode::Unicode61RemoveDiacritics).unwrap();

    let account = create_sample_account("acc_1", "user1@example.com");
    store.insert_account(&account).unwrap();

    let msg = create_sample_message(
        "msg_1",
        "acc_1",
        "INBOX",
        101,
        "Sua fatura chegou",
        "Segue em anexo o boleto para pagamento no valor de R$ 150,00.",
    );
    store.insert_message(&msg).unwrap();

    let fetched = store
        .get_message_by_id("msg_1")
        .unwrap()
        .expect("msg found");
    assert_eq!(fetched.id, "msg_1");
    assert_eq!(fetched.subject, "Sua fatura chegou");
    assert_eq!(fetched.attachments.len(), 1);
    assert_eq!(fetched.attachments[0].filename, "invoice.pdf");
    assert_eq!(
        fetched.tags,
        vec!["boleto".to_string(), "priority".to_string()]
    );
    assert!(!fetched.is_read);

    // Update read flag
    store.update_message_is_read("msg_1", true).unwrap();
    let updated = store.get_message_by_id("msg_1").unwrap().unwrap();
    assert!(updated.is_read);
}

#[test]
fn test_triage_badges_persistence_and_summary_hydration() {
    let key = DatabaseKey::from_bytes([23u8; 32]);
    let store =
        SqliteMailStore::open_in_memory(&key, FtsTokenizerMode::Unicode61RemoveDiacritics).unwrap();

    let account = create_sample_account("acc_triage", "triage@example.com");
    store.insert_account(&account).unwrap();

    let mut msg = create_sample_message(
        "msg_triage",
        "acc_triage",
        "INBOX",
        102,
        "Security Code",
        "Your verification code is 654321.",
    );
    msg.category_badge = Some("Security".to_string());
    msg.triage_badges = vec![triage_ai::TriageBadge::OtpCode {
        code: "654321".to_string(),
    }];
    store.insert_message(&msg).unwrap();

    let fetched = store
        .get_message_by_id("msg_triage")
        .unwrap()
        .expect("msg found");
    assert_eq!(fetched.category_badge.as_deref(), Some("Security"));
    assert_eq!(
        fetched.triage_badges,
        vec![triage_ai::TriageBadge::OtpCode {
            code: "654321".to_string(),
        }]
    );

    // Verify row summary in search results has triage_badges populated
    let results = store
        .search(&HybridQuery {
            query_text: "verification".to_string(),
            query_embedding_int8: None,
            account_filter: Some("acc_triage".to_string()),
            mode: SearchMode::Lexical,
            limit: 10,
            offset: 0,
        })
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].triage_badges,
        vec![triage_ai::TriageBadge::OtpCode {
            code: "654321".to_string(),
        }]
    );
}

#[test]
fn test_lexical_fts5_diacritics_and_account_filtering() {
    let key = DatabaseKey::from_bytes([33u8; 32]);
    let store =
        SqliteMailStore::open_in_memory(&key, FtsTokenizerMode::Unicode61RemoveDiacritics).unwrap();

    let acc1 = create_sample_account("acc_1", "user1@example.com");
    let acc2 = create_sample_account("acc_2", "user2@example.com");
    store.insert_account(&acc1).unwrap();
    store.insert_account(&acc2).unwrap();

    // Acc 1 message with Portuguese accents: "cobranca" vs "cobrança"
    let msg1 = create_sample_message(
        "msg_1",
        "acc_1",
        "INBOX",
        1,
        "Aviso de Cobrança",
        "Prezado cliente, seu boleto bancário está disponível para liquidação.",
    );
    // Acc 2 message
    let msg2 = create_sample_message(
        "msg_2",
        "acc_2",
        "INBOX",
        2,
        "Outra Cobrança",
        "Boleto vencendo amanhã.",
    );
    store.insert_message(&msg1).unwrap();
    store.insert_message(&msg2).unwrap();

    // 1. Search for "cobranca" without diacritics - should match "Cobrança"
    let query_diacritics = HybridQuery {
        query_text: "cobranca".to_string(),
        query_embedding_int8: None,
        account_filter: None,
        mode: SearchMode::Lexical,
        limit: 10,
        offset: 0,
    };
    let results = store.search(&query_diacritics).unwrap();
    assert_eq!(results.len(), 2);

    // 2. Search with account_filter = "acc_1"
    let query_filter = HybridQuery {
        query_text: "boleto".to_string(),
        query_embedding_int8: None,
        account_filter: Some("acc_1".to_string()),
        mode: SearchMode::Lexical,
        limit: 10,
        offset: 0,
    };
    let filtered_results = store.search(&query_filter).unwrap();
    assert_eq!(filtered_results.len(), 1);
    assert_eq!(filtered_results[0].id, "msg_1");
}

#[test]
fn test_semantic_and_hybrid_sub_15ms_search() {
    let key = DatabaseKey::from_bytes([44u8; 32]);
    let store =
        SqliteMailStore::open_in_memory(&key, FtsTokenizerMode::Unicode61RemoveDiacritics).unwrap();

    let account = create_sample_account("acc_1", "user1@example.com");
    store.insert_account(&account).unwrap();

    // Insert 5 sample messages
    for i in 1..=5 {
        let msg = create_sample_message(
            &format!("msg_{i}"),
            "acc_1",
            "INBOX",
            i,
            &format!("Notificação {i}"),
            &format!("Conteúdo do email de teste {i} com detalhes fiscais."),
        );
        store.insert_message(&msg).unwrap();

        // 384-dimensional int8 embedding with varying components
        let mut embedding = vec![0i8; 384];
        embedding[0] = (i * 20) as i8;
        embedding[1] = 50;
        store
            .insert_message_embedding(&format!("msg_{i}"), &embedding)
            .unwrap();
    }

    // Semantic query targeting vector close to msg_3 (embedding[0] = 60)
    let mut query_emb = vec![0i8; 384];
    query_emb[0] = 58;
    query_emb[1] = 50;

    let semantic_query = HybridQuery {
        query_text: "".to_string(),
        query_embedding_int8: Some(query_emb.clone()),
        account_filter: Some("acc_1".to_string()),
        mode: SearchMode::Semantic,
        limit: 3,
        offset: 0,
    };

    let start_sem = Instant::now();
    let semantic_results = store.search(&semantic_query).unwrap();
    let sem_elapsed = start_sem.elapsed();

    assert_eq!(semantic_results.len(), 3);
    assert_eq!(
        semantic_results[0].id, "msg_3",
        "msg_3 should be nearest neighbor"
    );
    assert!(
        sem_elapsed.as_millis() < 15,
        "Semantic search must complete under 15ms, took {:?}",
        sem_elapsed
    );

    // Hybrid query combining text and vector
    let hybrid_query = HybridQuery {
        query_text: "detalhes".to_string(),
        query_embedding_int8: Some(query_emb),
        account_filter: Some("acc_1".to_string()),
        mode: SearchMode::Hybrid,
        limit: 5,
        offset: 0,
    };

    let start_hyb = Instant::now();
    let hybrid_results = store.search(&hybrid_query).unwrap();
    let hyb_elapsed = start_hyb.elapsed();

    assert_eq!(hybrid_results.len(), 5);
    assert!(
        hyb_elapsed.as_millis() < 15,
        "Hybrid search must complete under 15ms, took {:?}",
        hyb_elapsed
    );
}

#[test]
fn persisted_triage_metadata_rehydrates_ranked_pages_after_reopen() {
    use storage::desktop::LocalMutation;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("metadata.db");
    let key = DatabaseKey::from_bytes([63; 32]);
    let tokenizer = FtsTokenizerMode::Unicode61RemoveDiacritics;
    let store = SqliteMailStore::open(&path, &key, tokenizer).unwrap();
    store
        .insert_account(&create_sample_account("local", "reader@example.com"))
        .unwrap();
    let mut embedding = vec![0i8; 384];
    embedding[0] = 100;
    for index in 1..=4 {
        let mut message = create_sample_message(
            &format!("saved-{index}"),
            "local",
            "INBOX",
            index,
            "Invoice",
            "Invoice details",
        );
        message.tags.clear();
        store.insert_message(&message).unwrap();
        let mut vector = embedding.clone();
        vector[1] = (index * 10) as i8;
        store
            .insert_message_embedding(&message.id, &vector)
            .unwrap();
    }
    let queries: Vec<_> = [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ]
    .into_iter()
    .map(|mode| HybridQuery {
        query_text: "Invoice".into(),
        query_embedding_int8: Some(embedding.clone()),
        account_filter: Some("local".into()),
        mode,
        limit: 2,
        offset: 1,
    })
    .collect();
    let before: Vec<_> = queries.iter().map(|q| store.search(q).unwrap()).collect();
    for index in 1..=3 {
        let id = format!("saved-{index}");
        store
            .apply_local_mutations(&[
                (id.clone(), LocalMutation::Folder("Archive"), None),
                (id.clone(), LocalMutation::Tag("Zulu, with comma"), None),
                (id.clone(), LocalMutation::Tag("Alpha"), None),
                (id, LocalMutation::Tag("Alpha"), None),
            ])
            .unwrap();
    }
    drop(store);
    let reopened = SqliteMailStore::open(&path, &key, tokenizer).unwrap();
    for (query, original) in queries.iter().zip(before) {
        let rows = reopened.search(query).unwrap();
        assert_eq!(
            rows.iter().map(|r| &r.id).collect::<Vec<_>>(),
            original.iter().map(|r| &r.id).collect::<Vec<_>>()
        );
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.folder, "INBOX");
            assert_eq!(
                row.uid,
                row.id
                    .strip_prefix("saved-")
                    .unwrap()
                    .parse::<u32>()
                    .unwrap()
            );
            if row.id == "saved-4" {
                assert!(row.tags.is_empty());
                assert_eq!(row.folder_override, None);
            } else {
                assert_eq!(row.tags, vec!["Alpha", "Zulu, with comma"]);
                assert_eq!(row.folder_override.as_deref(), Some("Archive"));
            }
        }
    }
}

#[test]
fn blank_inbox_pages_are_stable_filtered_and_persisted() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inbox.db");
    let key = DatabaseKey::from_bytes([76; 32]);
    let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    for account in ["a", "b"] {
        store
            .insert_account(&create_sample_account(
                account,
                &format!("{account}@example.com"),
            ))
            .unwrap();
    }
    for index in (0..6).rev() {
        let mut message = create_sample_message(
            &format!("inbox-{index}"),
            if index % 2 == 0 { "a" } else { "b" },
            "INBOX",
            index + 1,
            "Stored message",
            "Stored body",
        );
        message.received_at_epoch_ms = i64::from(index / 2);
        store.insert_message(&message).unwrap();
    }
    store
        .apply_local_mutations(&[(
            "inbox-4".into(),
            storage::desktop::LocalMutation::Folder("Archive"),
            None,
        )])
        .unwrap();
    drop(store);
    let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    let query = HybridQuery {
        query_text: " \n\t".into(),
        query_embedding_int8: None,
        account_filter: None,
        mode: SearchMode::Lexical,
        limit: 2,
        offset: 0,
    };
    let mut ids = Vec::new();
    for offset in [0, 2, 4, 6] {
        let rows = store
            .search(&HybridQuery {
                offset,
                ..query.clone()
            })
            .unwrap();
        if offset == 0 {
            assert_eq!(rows[0].folder, "INBOX");
            assert_eq!(rows[0].folder_override.as_deref(), Some("Archive"));
            assert_eq!(rows[0].tags, ["boleto", "priority"]);
        }
        ids.extend(rows.into_iter().map(|row| row.id));
    }
    assert_eq!(
        ids,
        ["inbox-4", "inbox-5", "inbox-2", "inbox-3", "inbox-0", "inbox-1"]
    );
    let filtered = store
        .search(&HybridQuery {
            query_text: String::new(),
            account_filter: Some("a".into()),
            offset: 1,
            ..query.clone()
        })
        .unwrap();
    assert_eq!(
        filtered.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["inbox-2", "inbox-0"]
    );
    assert!(store
        .search(&HybridQuery {
            offset: usize::MAX,
            ..query
        })
        .is_err());
}

#[test]
fn semantic_pages_match_exact_account_neighbors_with_missing_vectors_and_ties() {
    let key = DatabaseKey::from_bytes([77; 32]);
    let store = SqliteMailStore::open_in_memory(&key, Default::default()).unwrap();
    for account in ["a", "b", "empty"] {
        store
            .insert_account(&create_sample_account(
                account,
                &format!("{account}@example.com"),
            ))
            .unwrap();
    }
    let mut reference = Vec::new();
    for index in (0..36u32).rev() {
        let account = if index < 12 { "a" } else { "b" };
        let id = format!("vector-{index:02}");
        store
            .insert_message(&create_sample_message(
                &id,
                account,
                "INBOX",
                index + 1,
                "shared",
                "shared",
            ))
            .unwrap();
        if index == 35 {
            continue;
        } // Eligible mail need not have an embedding.
        let mut vector = vec![0i8; 384];
        vector[0] = 100;
        vector[1] = if account == "a" { 0 } else { (index / 3) as i8 };
        store.insert_message_embedding(&id, &vector).unwrap();
        if account == "b" {
            let distance = 1.0 - 100.0 / (10000.0 + f64::from(vector[1]).powi(2)).sqrt();
            reference.push((distance, id));
        }
    }
    reference.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut embedding = vec![0; 384];
    embedding[0] = 100;
    let query = HybridQuery {
        query_text: String::new(),
        query_embedding_int8: Some(embedding),
        account_filter: Some("b".into()),
        mode: SearchMode::Semantic,
        limit: 4,
        offset: 0,
    };
    for offset in [0, 4, 8, 20, 24, 4096] {
        let rows = store
            .search(&HybridQuery {
                offset,
                ..query.clone()
            })
            .unwrap();
        let expected: Vec<_> = reference
            .iter()
            .skip(offset)
            .take(4)
            .map(|(_, id)| id.as_str())
            .collect();
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            expected
        );
    }
    assert!(store
        .search(&HybridQuery {
            account_filter: Some("empty".into()),
            ..query.clone()
        })
        .unwrap()
        .is_empty());
    let tie_query = HybridQuery {
        account_filter: Some("a".into()),
        ..query.clone()
    };
    for offset in [0, 4, 8] {
        let rows = store
            .search(&HybridQuery {
                offset,
                ..tie_query.clone()
            })
            .unwrap();
        assert_eq!(rows[0].id, format!("vector-{offset:02}"));
        assert_eq!(rows.len(), 4);
    }
    // INVARIANT: Unfiltered vec0 pages must include the whole boundary tie before sorting IDs.
    let unfiltered = HybridQuery {
        account_filter: None,
        ..query.clone()
    };
    for offset in [0, 4, 8] {
        let rows = store
            .search(&HybridQuery {
                offset,
                ..unfiltered.clone()
            })
            .unwrap();
        assert_eq!(
            rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            (offset..offset + 4)
                .map(|id| format!("vector-{id:02}"))
                .collect::<Vec<_>>()
        );
    }
    store
        .insert_message_embedding("orphan", &unfiltered.query_embedding_int8.clone().unwrap())
        .unwrap();
    let rows = store.search(&unfiltered).unwrap();
    assert_eq!(rows[0].id, "vector-00");
    assert_eq!(rows.len(), 4);
    let hybrid = HybridQuery {
        mode: SearchMode::Hybrid,
        query_text: "shared".into(),
        ..query
    };
    let first = store.search(&hybrid).unwrap();
    for _ in 0..5 {
        assert_eq!(store.search(&hybrid).unwrap(), first);
    }
}

#[test]
fn semantic_large_ties_and_deep_pages_survive_vec0_k_limit() {
    let key = DatabaseKey::from_bytes([78; 32]);
    let store = SqliteMailStore::open_in_memory(&key, Default::default()).unwrap();
    store
        .insert_account(&create_sample_account("a", "reader@example.com"))
        .unwrap();
    let embedding = vec![3; 384];
    for index in (0..4100u32).rev() {
        let id = format!("tie-{index:04}");
        let mut message = create_sample_message(&id, "a", "INBOX", index + 1, "shared", "shared");
        message.attachments.clear();
        message.tags.clear();
        store.insert_message(&message).unwrap();
        store.insert_message_embedding(&id, &embedding).unwrap();
    }
    let query = HybridQuery {
        query_text: String::new(),
        query_embedding_int8: Some(embedding),
        account_filter: None,
        mode: SearchMode::Semantic,
        limit: 4,
        offset: 0,
    };
    for account in [None, Some("a".to_string())] {
        for offset in [0, 4000, 4096] {
            let rows = store
                .search(&HybridQuery {
                    offset,
                    account_filter: account.clone(),
                    ..query.clone()
                })
                .unwrap();
            assert_eq!(
                rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
                (offset..offset + 4)
                    .map(|id| format!("tie-{id:04}"))
                    .collect::<Vec<_>>()
            );
        }
    }
}
