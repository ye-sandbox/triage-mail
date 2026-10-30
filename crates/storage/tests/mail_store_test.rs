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
