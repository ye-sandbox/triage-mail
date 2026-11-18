use std::sync::atomic::Ordering;

use tempfile::tempdir;
use triage_ai::TriageCategory;

use crate::desktop::LocalMutation;
use crate::ram_vector_index::{RamVectorIndex, RamVectorState, RAM_VECTOR_DIM};
use crate::saved_views::{SavedView, ViewFilters};
use crate::sender_review::{ReviewPreferences, ReviewState, SenderChoice};
use crate::{
    AccountRecord, DatabaseKey, FolderRecord, FolderSyncState, HybridQuery, MailboxScope,
    MailboxView, MessageRecord, SearchEngine, SearchMode, SqliteMailStore,
};

fn vec_unit(seed: i8, idx: usize) -> Vec<i8> {
    let mut v = vec![0i8; RAM_VECTOR_DIM];
    v[idx % RAM_VECTOR_DIM] = seed;
    v[(idx + 7) % RAM_VECTOR_DIM] = seed.saturating_div(2).max(1);
    v
}

fn vec_zero() -> Vec<i8> {
    vec![0i8; RAM_VECTOR_DIM]
}

fn blob_of(v: &[i8]) -> Vec<u8> {
    v.iter().map(|&x| x as u8).collect()
}

fn store_mem() -> SqliteMailStore {
    SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([77; 32]), Default::default()).unwrap()
}

fn seed_account(store: &SqliteMailStore, id: &str, email: &str) {
    store
        .insert_account(&AccountRecord {
            id: id.into(),
            email: email.into(),
            auth_type: "synthetic".into(),
            created_at_epoch_ms: 0,
            updated_at_epoch_ms: 0,
        })
        .unwrap();
    store
        .save_folders(
            id,
            &[
                FolderRecord {
                    account_id: id.into(),
                    folder_name: "INBOX".into(),
                    display_name: "INBOX".into(),
                    delimiter: Some("/".into()),
                    role: "inbox".into(),
                    is_selectable: true,
                    total_messages: None,
                    unread_messages: None,
                    updated_at_epoch_ms: 0,
                },
                FolderRecord {
                    account_id: id.into(),
                    folder_name: "Archive".into(),
                    display_name: "Archive".into(),
                    delimiter: Some("/".into()),
                    role: "archive".into(),
                    is_selectable: true,
                    total_messages: None,
                    unread_messages: None,
                    updated_at_epoch_ms: 0,
                },
                FolderRecord {
                    account_id: id.into(),
                    folder_name: "Custom".into(),
                    display_name: "Custom".into(),
                    delimiter: Some("/".into()),
                    role: "custom".into(),
                    is_selectable: true,
                    total_messages: None,
                    unread_messages: None,
                    updated_at_epoch_ms: 0,
                },
            ],
        )
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn insert_msg(
    store: &SqliteMailStore,
    id: &str,
    account: &str,
    folder: &str,
    uid: u32,
    subject: &str,
    sender: &str,
    received_ms: i64,
    emb: Option<&[i8]>,
) {
    store
        .insert_message(&MessageRecord {
            id: id.into(),
            account_id: account.into(),
            provider_message_id: None,
            folder: folder.into(),
            uid,
            message_id_header: Some(format!("<{id}@example.org>")),
            in_reply_to: None,
            references_header: None,
            subject: subject.into(),
            sender: sender.into(),
            recipients: "reader@example.com".into(),
            received_at_epoch_ms: received_ms,
            is_read: false,
            has_attachments: false,
            category_badge: None,
            triage_badges: vec![],
            raw_headers: String::new(),
            text_body: Some(format!("body {subject}")),
            html_body: None,
            list_unsubscribe: None,
            list_unsubscribe_post: None,
            one_click_unsubscribe_supported: false,
            created_at_epoch_ms: received_ms,
            attachments: vec![],
            tags: vec![],
            is_starred: false,
            snooze_until_epoch_ms: None,
        })
        .unwrap();
    if let Some(vec) = emb {
        store.insert_message_embedding(id, vec).unwrap();
    }
}

fn oracle_vs_ram<T, F>(store: &SqliteMailStore, mut run: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&SqliteMailStore) -> T,
{
    store.clear_ram_index();
    assert!(!store.is_ram_index_ready());
    let expected = run(store);
    store.hydrate_ram_index().unwrap();
    assert!(store.is_ram_index_ready());
    let actual = run(store);
    assert_eq!(
        actual, expected,
        "RAM index diverged from sqlite-vec oracle"
    );
}

#[test]
fn ram_index_matches_sqlite_oracle_across_all_scopes_ties_and_zero_norms() {
    let store = store_mem();
    seed_account(&store, "acc-a", "a@example.com");
    seed_account(&store, "acc-b", "b@example.org");

    store
        .save_review_preferences(ReviewPreferences {
            account_id: "acc-a".into(),
            revision: 0,
            enabled: true,
            hide_in_inbox: true,
            initializing: false,
        })
        .unwrap();

    // Seed mixed embeddings: zero-norm, identical tie cluster (>100 messages), distinct vectors, missing vectors.
    let tie_vec = vec_unit(40, 3);
    for i in 0..120 {
        let id = format!("tie-{i:03}");
        insert_msg(
            &store,
            &id,
            "acc-a",
            "INBOX",
            (i + 1) as u32,
            if i % 3 == 0 {
                "invoice quarterly notice"
            } else {
                "general announcement"
            },
            if i % 5 == 0 {
                "pending-sender@example.org"
            } else {
                "known-sender@example.org"
            },
            10_000 + i64::from(i),
            Some(&tie_vec),
        );
    }

    // Zero-norm vector rows (k-021 NULL cosine distance -> sorted first)
    let zero = vec_zero();
    insert_msg(
        &store,
        "z-01",
        "acc-a",
        "INBOX",
        500,
        "invoice zero vector",
        "known-sender@example.org",
        20_000,
        Some(&zero),
    );
    insert_msg(
        &store,
        "z-02",
        "acc-b",
        "INBOX",
        501,
        "invoice zero b",
        "other@example.org",
        20_001,
        Some(&zero),
    );
    // Unembedded message
    insert_msg(
        &store,
        "no-vec",
        "acc-a",
        "INBOX",
        502,
        "invoice no embedding",
        "known-sender@example.org",
        20_002,
        None,
    );
    // Starred & Snoozed (active vs expired) & Overridden & Duplicate memberships
    let distinct = vec_unit(90, 12);
    insert_msg(
        &store,
        "m-starred",
        "acc-a",
        "Archive",
        600,
        "invoice starred archive",
        "known-sender@example.org",
        21_000,
        Some(&distinct),
    );
    insert_msg(
        &store,
        "m-snoozed-active",
        "acc-a",
        "INBOX",
        601,
        "invoice snoozed active",
        "known-sender@example.org",
        21_001,
        Some(&distinct),
    );
    insert_msg(
        &store,
        "m-snoozed-expired",
        "acc-a",
        "INBOX",
        602,
        "invoice snoozed expired",
        "known-sender@example.org",
        21_002,
        Some(&distinct),
    );
    insert_msg(
        &store,
        "m-overridden",
        "acc-a",
        "INBOX",
        603,
        "invoice overridden to custom",
        "known-sender@example.org",
        21_003,
        Some(&distinct),
    );

    store
        .apply_local_mutations(&[
            ("m-starred".into(), LocalMutation::Star(true), None),
            (
                "m-snoozed-active".into(),
                LocalMutation::Snooze(Some(50_000)),
                None,
            ),
            (
                "m-snoozed-expired".into(),
                LocalMutation::Snooze(Some(30_000)),
                None,
            ),
            ("m-overridden".into(), LocalMutation::Folder("Custom"), None),
        ])
        .unwrap();

    // Add duplicate folder membership + orphan vec_messages row + categories + review choices
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO message_folder_memberships VALUES ('acc-a','INBOX',999,'m-starred',0)",
            [],
        )
        .unwrap();
        // Orphan vector without messages row must never enter RAM index or match
        conn.execute(
            "INSERT INTO vec_messages(message_id, embedding_int8) VALUES ('orphan-id', vec_int8(?1))",
            [blob_of(&distinct)],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sender_review_choices VALUES ('acc-a','pending-sender@example.org',1,'Pending')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sender_review_choices VALUES ('acc-a','known-sender@example.org',1,'Accepted')",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT OR IGNORE INTO sender_review_messages SELECT id, account_id, sender FROM messages WHERE account_id='acc-a';",
        )
        .unwrap();
    }
    while store.backfill_categories(200).unwrap() > 0 {}
    for cid in ["m-starred", "tie-000", "z-01"] {
        store
            .correct_category(cid, 0, Some(TriageCategory::Financial))
            .unwrap();
    }

    // Tag one message and create complex + pure SavedViews
    store
        .apply_local_mutations(&[("tie-001".into(), LocalMutation::Tag("vip"), None)])
        .unwrap();
    let saved = store
        .save_saved_view(SavedView {
            version: 1,
            id: "sv-complex".into(),
            revision: 0,
            name: "Complex".into(),
            query: "invoice".into(),
            mode: SearchMode::Hybrid,
            accounts: vec!["acc-a".into()],
            scope: Some(MailboxScope::AccountView {
                account_id: "acc-a".into(),
                view: MailboxView::Inbox,
            }),
            filters: ViewFilters {
                tags: vec!["vip".into()],
                ..Default::default()
            },
        })
        .unwrap();
    let saved_pure = store
        .save_saved_view(SavedView {
            version: 1,
            id: "sv-pure".into(),
            revision: 0,
            name: "PureInbox".into(),
            query: "invoice".into(),
            mode: SearchMode::Hybrid,
            accounts: vec![],
            scope: Some(MailboxScope::AccountView {
                account_id: "acc-a".into(),
                view: MailboxView::Inbox,
            }),
            filters: ViewFilters::default(),
        })
        .unwrap();

    let query_vec = vec_unit(50, 3);
    let now_ms = 40_000;

    // 1. Global & Account-filtered Semantic / Hybrid / Deep Ties
    for (mode, offset, acct) in [
        (SearchMode::Semantic, 0, None),
        (SearchMode::Semantic, 95, Some("acc-a".to_string())),
        (SearchMode::Hybrid, 0, None),
        (SearchMode::Hybrid, 15, Some("acc-a".to_string())),
    ] {
        let q = HybridQuery {
            query_text: "invoice".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: acct,
            mode,
            limit: 25,
            offset,
        };
        oracle_vs_ram(&store, |s| {
            s.search(&q)
                .unwrap()
                .into_iter()
                .map(|r| r.id)
                .collect::<Vec<_>>()
        });
    }

    // 2. Zero-norm query vector parity (k-021)
    let q_zero = HybridQuery {
        query_text: String::new(),
        query_embedding_int8: Some(zero.clone()),
        account_filter: None,
        mode: SearchMode::Semantic,
        limit: 30,
        offset: 0,
    };
    oracle_vs_ram(&store, |s| {
        s.search(&q_zero)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>()
    });

    // 3. Scoped searches: Inbox (with snooze boundary + INBOX_VISIBLE), Starred, Snoozed, Custom folder
    for view in [
        MailboxView::Inbox,
        MailboxView::Starred,
        MailboxView::Snoozed,
        MailboxView::Folder("Custom".into()),
    ] {
        for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
            let q = HybridQuery {
                query_text: "invoice".into(),
                query_embedding_int8: Some(query_vec.clone()),
                account_filter: None,
                mode,
                limit: 30,
                offset: 0,
            };
            let scope = MailboxScope::AccountView {
                account_id: "acc-a".into(),
                view: view.clone(),
            };
            oracle_vs_ram(&store, |s| {
                s.search_scoped_at(&q, &scope, now_ms)
                    .unwrap()
                    .into_iter()
                    .map(|r| r.id)
                    .collect::<Vec<_>>()
            });
        }
    }

    // 4. Category search parity
    for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
        let q = HybridQuery {
            query_text: "invoice".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: None,
            mode,
            limit: 20,
            offset: 0,
        };
        oracle_vs_ram(&store, |s| {
            s.search_category(
                &q,
                Some(&MailboxScope::AccountView {
                    account_id: "acc-a".into(),
                    view: MailboxView::Inbox,
                }),
                TriageCategory::Financial,
            )
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>()
        });
    }

    // 5. Sender Review search parity (Pending includes pending-sender hidden from Inbox)
    for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
        let q = HybridQuery {
            query_text: "invoice".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: Some("acc-a".into()),
            mode,
            limit: 20,
            offset: 0,
        };
        oracle_vs_ram(&store, |s| {
            s.search_sender_review(&q, ReviewState::Pending)
                .unwrap()
                .into_iter()
                .map(|r| r.id)
                .collect::<Vec<_>>()
        });
    }

    // 6. Saved Views (complex rowid fallback AND pure-RAM apply_review_hide=false)
    oracle_vs_ram(&store, |s| {
        s.search_saved_view_at(
            &saved.id,
            saved.revision,
            Some(query_vec.clone()),
            (20, 0),
            now_ms,
        )
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>()
    });
    oracle_vs_ram(&store, |s| {
        s.search_saved_view_at(
            &saved_pure.id,
            saved_pure.revision,
            Some(query_vec.clone()),
            (40, 0),
            now_ms,
        )
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>()
    });

    // 7. Conversation search parity (Messages, Category, Review)
    while store.backfill_conversations().unwrap() > 0 {}
    for source in [
        crate::conversations::ConversationSource::Messages,
        crate::conversations::ConversationSource::Category(TriageCategory::Financial),
        crate::conversations::ConversationSource::Review(ReviewState::Pending),
    ] {
        for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
            let q = HybridQuery {
                query_text: "invoice".into(),
                query_embedding_int8: Some(query_vec.clone()),
                account_filter: Some("acc-a".into()),
                mode,
                limit: 25,
                offset: 0,
            };
            let ctx = crate::conversations::ConversationQuery {
                source: source.clone(),
                scope: matches!(source, crate::conversations::ConversationSource::Messages).then(
                    || MailboxScope::AccountView {
                        account_id: "acc-a".into(),
                        view: MailboxView::Inbox,
                    },
                ),
                generation: None,
            };
            oracle_vs_ram(&store, |s| {
                s.search_conversations(&q, &ctx)
                    .unwrap()
                    .rows
                    .into_iter()
                    .map(|r| (r.conversation_id, r.representative.id, r.matching_count))
                    .collect::<Vec<_>>()
            });
        }
    }
}

#[test]
fn incremental_writes_keep_ram_ready_and_sync_accurately() {
    let store = store_mem();
    seed_account(&store, "acc-a", "a@example.com");
    let vec_a = vec_unit(80, 1);
    let vec_b = vec_unit(20, 2);

    insert_msg(
        &store,
        "m1",
        "acc-a",
        "INBOX",
        1,
        "hello",
        "sender@example.org",
        1000,
        Some(&vec_a),
    );
    store.hydrate_ram_index().unwrap();
    assert!(store.is_ram_index_ready());
    assert_eq!(store.ram_index_len(), Some(1));

    // Non-vector background write (conversations/alerts/reminders) keeps RAM hot!
    let _ = store.backfill_conversations().unwrap();
    assert!(store.is_ram_index_ready());

    // Insert new message + embedding via managed API -> updates RAM incrementally!
    insert_msg(
        &store,
        "m2",
        "acc-a",
        "INBOX",
        2,
        "world",
        "sender@example.org",
        2000,
        Some(&vec_b),
    );
    assert!(store.is_ram_index_ready());
    assert_eq!(store.ram_index_len(), Some(2));

    // Local mutations: Star + Folder override
    store
        .apply_local_mutations(&[
            ("m2".into(), LocalMutation::Star(true), None),
            ("m1".into(), LocalMutation::Folder("Archive"), None),
        ])
        .unwrap();
    assert!(store.is_ram_index_ready());

    let q_star = HybridQuery {
        query_text: String::new(),
        query_embedding_int8: Some(vec_a.clone()),
        account_filter: None,
        mode: SearchMode::Semantic,
        limit: 10,
        offset: 0,
    };
    let starred = store
        .search_scoped(
            &q_star,
            &MailboxScope::AccountView {
                account_id: "acc-a".into(),
                view: MailboxView::Starred,
            },
        )
        .unwrap();
    assert_eq!(starred.len(), 1);
    assert_eq!(starred[0].id, "m2");

    // Category correction updates RAM category immediately
    store.category_metadata("m2").unwrap();
    store
        .correct_category("m2", 0, Some(TriageCategory::Security))
        .unwrap();
    assert!(store.is_ram_index_ready());
    let sec = store
        .search_category(&q_star, None, TriageCategory::Security)
        .unwrap();
    assert_eq!(sec.len(), 1);
    assert_eq!(sec[0].id, "m2");

    // Sender review choice updates RAM visibility immediately
    store
        .save_review_preferences(ReviewPreferences {
            account_id: "acc-a".into(),
            revision: 0,
            enabled: true,
            hide_in_inbox: true,
            initializing: false,
        })
        .unwrap();
    {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO sender_review_messages VALUES ('m2','acc-a','sender@example.org')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO sender_review_choices VALUES ('acc-a','sender@example.org',1,'Pending')",
            [],
        )
        .unwrap();
        // Raw SQL mutation guarantees sqlite3_total_changes() increments out-of-band.
        conn.execute("UPDATE messages SET subject='raw edit' WHERE id='m2'", [])
            .unwrap();
    }
    // Raw SQL above bumped total_changes -> search wipes stale index and falls back to vec0
    assert!(store.search(&q_star).is_ok());
    assert!(!store.is_ram_index_ready());
    store.hydrate_ram_index().unwrap();
    assert!(store.is_ram_index_ready());

    store
        .save_sender_choice(SenderChoice {
            account_id: "acc-a".into(),
            sender: "sender@example.org".into(),
            revision: 1,
            state: ReviewState::Declined,
        })
        .unwrap();
    assert!(store.is_ram_index_ready());
    let inbox = store
        .search_scoped(
            &q_star,
            &MailboxScope::AccountView {
                account_id: "acc-a".into(),
                view: MailboxView::Inbox,
            },
        )
        .unwrap();
    assert!(inbox.is_empty(), "Declined sender must be hidden in Inbox");

    // complete_remote_move(None) deletes message & removes slot from RAM
    store
        .apply_local_mutations(&[(
            "m1".into(),
            LocalMutation::Folder("Archive"),
            Some("{\"op\":\"move\"}".into()),
        )])
        .unwrap();
    let outbox_id: i64 = store
        .conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM pending_mutations WHERE message_id='m1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    store
        .complete_remote_move(
            outbox_id,
            "m1",
            &FolderSyncState {
                account_id: "acc-a".into(),
                folder_name: "Archive".into(),
                uidvalidity: 1,
                uidnext: 10,
                highest_modseq: None,
            },
            None,
        )
        .unwrap();
    assert!(store.is_ram_index_ready());
    assert_eq!(store.ram_index_len(), Some(1));
    let global = store.search(&q_star).unwrap();
    assert_eq!(
        global.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec!["m2"]
    );

    // Rebuilding embedding generation wipes RAM index
    store
        .begin_embedding_generation("new-gen-v2", true)
        .unwrap();
    assert_eq!(store.ram_index_len(), Some(0));
}

#[test]
fn detects_external_connection_writes_and_hydrating_falls_back_to_vec0() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("wal_sync.db");
    let key = DatabaseKey::from_bytes([88; 32]);
    let store = SqliteMailStore::open(&db_path, &key, Default::default()).unwrap();
    seed_account(&store, "acc-a", "a@example.com");

    let vec_a = vec_unit(70, 5);
    insert_msg(
        &store,
        "m1",
        "acc-a",
        "INBOX",
        1,
        "initial",
        "s@example.org",
        1000,
        Some(&vec_a),
    );
    store.hydrate_ram_index().unwrap();
    assert!(store.is_ram_index_ready());

    // Open an external SQLite connection with the same key and delete m1 out-of-band.
    let ext = SqliteMailStore::open(&db_path, &key, Default::default()).unwrap();
    {
        let conn = ext.conn.lock().unwrap();
        conn.execute("DELETE FROM vec_messages WHERE message_id='m1'", [])
            .unwrap();
        conn.execute("DELETE FROM messages WHERE id='m1'", [])
            .unwrap();
    }

    let q = HybridQuery {
        query_text: String::new(),
        query_embedding_int8: Some(vec_a.clone()),
        account_filter: None,
        mode: SearchMode::Semantic,
        limit: 10,
        offset: 0,
    };
    let rows = store.search(&q).unwrap();
    assert!(
        rows.is_empty(),
        "stale RAM index must be discarded on data_version change"
    );
    assert!(!store.is_ram_index_ready());

    // Simulate concurrent Hydrating state: search falls back to vec0 transparently without error/blocking.
    insert_msg(
        &store,
        "m2",
        "acc-a",
        "INBOX",
        2,
        "hydrating fallback",
        "s@example.org",
        2000,
        Some(&vec_a),
    );
    {
        let mut guard = store.ram_vectors.write().unwrap();
        *guard = RamVectorState::Hydrating;
    }
    let fallback_rows = store.search(&q).unwrap();
    assert_eq!(fallback_rows.len(), 1);
    assert_eq!(fallback_rows[0].id, "m2");
}

#[test]
fn zeroize_clears_vectors_on_clear_and_swap_remove() {
    let mut index = RamVectorIndex::new();
    let v1 = vec![42i8; RAM_VECTOR_DIM];
    let v2 = vec![-17i8; RAM_VECTOR_DIM];
    index
        .upsert_slot(
            1,
            "m1",
            "acc-a",
            &v1,
            false,
            None,
            None,
            None,
            false,
            &["INBOX".into()],
        )
        .unwrap();
    index
        .upsert_slot(
            2,
            "m2",
            "acc-a",
            &v2,
            false,
            None,
            None,
            None,
            false,
            &["INBOX".into()],
        )
        .unwrap();
    assert_eq!(index.len(), 2);
    assert!(index.raw_vectors_slice().contains(&42));

    index.remove_slot("m1");
    assert_eq!(index.len(), 1);
    assert_eq!(index.raw_vectors_slice().len(), RAM_VECTOR_DIM);
    assert!(index.raw_vectors_slice().iter().all(|&x| x == -17));

    index.clear();
    assert!(index.is_empty());
    assert!(index.raw_vectors_slice().is_empty());

    // Also verify clear_ram_index bumps wipe epoch and clears store state
    let store = store_mem();
    seed_account(&store, "acc-a", "a@example.com");
    insert_msg(
        &store,
        "m1",
        "acc-a",
        "INBOX",
        1,
        "sub",
        "s@example.org",
        100,
        Some(&v1),
    );
    store.hydrate_ram_index().unwrap();
    assert!(store.is_ram_index_ready());
    let prev_wipe = store.ram_wipe_epoch.load(Ordering::Acquire);
    store.clear_ram_index();
    assert!(!store.is_ram_index_ready());
    assert_eq!(store.ram_index_len(), None);
    assert_eq!(store.ram_wipe_epoch.load(Ordering::Acquire), prev_wipe + 1);
}
