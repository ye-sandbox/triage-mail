use super::*;
use mail_engine::{protocol::UidFlags, AttachmentMeta, UnsubscribeDescriptor};
use storage::{AccountRecord, DatabaseKey, MailRepository};
use triage_ai::rules::{Condition, Rule, RuleAction};

fn state() -> FolderSyncState {
    FolderSyncState {
        account_id: "fixture".into(),
        folder_name: "INBOX".into(),
        uidvalidity: 7,
        uidnext: 2,
        highest_modseq: Some(8),
    }
}
fn message(uid: u32) -> ParsedInboundMessage {
    ParsedInboundMessage {
        uid,
        modseq: None,
        provider_message_id: None,
        message_id_header: Some(format!("<synthetic-{uid}@example.org>")),
        in_reply_to: None,
        references: vec![],
        references_header: None,
        subject: "Synthetic notice".into(),
        sender: format!("sender-{uid}@example.org"),
        recipients: vec!["reader@example.com".into(), "other@example.net".into()],
        date_epoch_ms: Some(100),
        text_plain: Some("Synthetic body".into()),
        html_raw: Some("<p>Synthetic body</p>".into()),
        raw_headers: Some("X-Synthetic: yes".into()),
        unsubscribe: None,
        attachments: vec![],
    }
}
fn attachment() -> AttachmentMeta {
    AttachmentMeta {
        part_id: "2.1".into(),
        filename: "synthetic.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 123,
        content_id: Some("synthetic@example.org".into()),
        is_inline: true,
    }
}
fn batch(messages: Vec<ParsedInboundMessage>, next: u32) -> IncrementalSyncResult {
    IncrementalSyncResult {
        folder: "INBOX".into(),
        uidvalidity: 7,
        uidvalidity_reset: false,
        server_uidnext: next,
        new_last_uid: next.saturating_sub(1),
        highest_modseq: Some(8),
        new_messages: messages,
        flag_deltas: vec![],
    }
}
fn store(path: &std::path::Path) -> SqliteMailStore {
    let store = SqliteMailStore::open(path, &DatabaseKey::from_bytes([71; 32]), Default::default())
        .unwrap();
    store
        .insert_account(&AccountRecord {
            id: "fixture".into(),
            email: "reader@example.com".into(),
            auth_type: "synthetic".into(),
            created_at_epoch_ms: 0,
            updated_at_epoch_ms: 0,
        })
        .unwrap();
    store
}
fn sql(store: &SqliteMailStore, sql: &str) {
    store
        .secondary_connection()
        .unwrap()
        .lock()
        .unwrap()
        .execute_batch(sql)
        .unwrap();
}
fn count(store: &SqliteMailStore, table: &str) -> i64 {
    store
        .secondary_connection()
        .unwrap()
        .lock()
        .unwrap()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}
fn enable_arrival_features(store: &SqliteMailStore) {
    let mut preferences = store.review_preferences("fixture").unwrap();
    preferences.enabled = true;
    store.save_review_preferences(preferences).unwrap();
    store.backfill_sender_review().unwrap();
    store
        .save_rule(Rule {
            version: 1,
            id: "synthetic-rule".into(),
            revision: 0,
            name: "Synthetic arrivals".into(),
            enabled: true,
            position: 0,
            all: vec![Condition::Account("fixture".into())],
            any: vec![],
            except: vec![],
            actions: vec![RuleAction::Tag("arrival".into())],
            stop_processing: false,
        })
        .unwrap();
}

#[test]
fn conversion_preserves_identity_metadata_lazy_attachments_and_deterministic_triage() {
    let mut input = message(3);
    input.attachments.push(attachment());
    input.in_reply_to = Some("<parent@example.org>".into());
    input.references_header = input.in_reply_to.clone();
    input.unsubscribe = Some(UnsubscribeDescriptor {
        https_uri: Some("https://example.org/unsubscribe".into()),
        mailto_uri: None,
        supports_rfc8058_one_click: true,
        raw_list_unsubscribe: Some("<https://example.org/unsubscribe>".into()),
        raw_list_unsubscribe_post: Some("List-Unsubscribe=One-Click".into()),
    });
    let mut calls = 0;
    let record = message_record(input, &state(), &mut || {
        calls += 1;
        200
    })
    .unwrap();
    assert_eq!(record.id, r#"["fixture","INBOX",7,3]"#);
    assert_eq!(record.created_at_epoch_ms, 200);
    assert_eq!(record.received_at_epoch_ms, 100);
    assert_eq!(calls, 1); // A supplied Date must not consume a fallback clock reading.
    assert_eq!(record.category_badge.as_deref(), Some("Newsletter"));
    assert!(record.one_click_unsubscribe_supported);
    assert_eq!(
        record.list_unsubscribe_post.as_deref(),
        Some("List-Unsubscribe=One-Click")
    );
    assert_eq!(record.raw_headers, "X-Synthetic: yes");
    assert_eq!(record.html_body.as_deref(), Some("<p>Synthetic body</p>"));
    assert_eq!(record.recipients, "reader@example.com, other@example.net");
    assert_eq!(record.in_reply_to, record.references_header);
    assert!(record.has_attachments);
    assert_eq!(record.attachments[0].id, format!("{}:2.1", record.id));
    assert_eq!(record.attachments[0].message_id, record.id);
    assert_eq!(record.attachments[0].size_bytes, 123);
    assert_eq!(record.attachments[0].content_type, "application/pdf");
    assert!(record.attachments[0].is_inline);
    assert!(!record.is_read && !record.is_starred);
    assert!(record.tags.is_empty() && record.snooze_until_epoch_ms.is_none());
}

#[test]
fn missing_date_uses_separate_clock_reads_and_identity_escapes_folder_names() {
    let mut input = message(1);
    input.date_epoch_ms = None;
    input.raw_headers = None;
    let mut state = state();
    state.folder_name = "Quoted \"folder\"\\path".into();
    let mut clock = [101, 202].into_iter();
    let record = message_record(input, &state, &mut || clock.next().unwrap()).unwrap();
    assert_eq!(record.received_at_epoch_ms, 101);
    assert_eq!(record.created_at_epoch_ms, 202);
    assert_eq!(record.raw_headers, "");
    let identity: (String, String, u32, u32) = serde_json::from_str(&record.id).unwrap();
    assert_eq!(identity, (state.account_id, state.folder_name, 7, 1));
}

#[test]
fn invalid_message_inputs_fail_before_clock_or_partial_batch_commit() {
    assert_eq!(
        message_record(message(0), &state(), &mut || panic!("unexpected clock")).unwrap_err(),
        IngestError::InvalidRequest
    );
    let mut input = message(1);
    input.attachments.push(AttachmentMeta {
        size_bytes: u64::MAX,
        ..attachment()
    });
    assert_eq!(
        message_record(input, &state(), &mut || panic!("unexpected clock")).unwrap_err(),
        IngestError::InvalidRequest
    );
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir.path().join("mail.db"));
    assert_eq!(
        ingest_batch(
            &store,
            &state(),
            batch(vec![message(1), message(0)], 2),
            || 100
        )
        .unwrap_err(),
        IngestError::InvalidRequest
    );
    assert_eq!(count(&store, "messages"), 0);
    assert!(store
        .get_folder_sync_state("fixture", "INBOX")
        .unwrap()
        .is_none());
}

#[test]
fn failed_conversion_retains_frontiers_and_retry_does_not_treat_history_as_arrivals() {
    use storage::sender_review::ReviewState;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mail.db");
    let store = store(&path);
    enable_arrival_features(&store);
    assert_eq!(
        ingest_batch(&store, &state(), batch(vec![message(0)], 2), || 100).unwrap_err(),
        IngestError::InvalidRequest
    );
    assert_eq!(count(&store, "rule_baselines"), 1);
    assert_eq!(count(&store, "sender_review_frontiers"), 1);
    let history = ingest_batch(&store, &state(), batch(vec![message(1)], 100), || 100).unwrap();
    assert_eq!(
        store
            .review_metadata(&history[0].id)
            .unwrap()
            .unwrap()
            .state,
        ReviewState::Known
    );
    assert_eq!(count(&store, "rule_jobs"), 0);
    let next_state = FolderSyncState {
        uidnext: 3,
        ..state()
    };
    let arrival = ingest_batch(&store, &next_state, batch(vec![message(2)], 100), || 100).unwrap();
    assert_eq!(
        store
            .review_metadata(&arrival[0].id)
            .unwrap()
            .unwrap()
            .state,
        ReviewState::Pending
    );
    assert_eq!(count(&store, "rule_jobs"), 1);
    assert_eq!(count(&store, "pending_mutations"), 0); // Hooks have not executed.
    drop(store);
    assert_ne!(&std::fs::read(&path).unwrap()[..16], b"SQLite format 3\0");
    let reopened = SqliteMailStore::open(
        &path,
        &DatabaseKey::from_bytes([71; 32]),
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        reopened.get_folder_sync_state("fixture", "INBOX").unwrap(),
        Some(next_state)
    );
    assert_eq!(count(&reopened, "rule_jobs"), 1);
}

#[test]
fn duplicate_memberships_return_canonical_identity_and_normalize_server_flags() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir.path().join("mail.db"));
    let original = ingest_batch(&store, &state(), batch(vec![message(1)], 2), || 100).unwrap();
    let other = FolderSyncState {
        folder_name: "Archive".into(),
        uidnext: 11,
        ..state()
    };
    let mut duplicate = message(10);
    duplicate.message_id_header = message(1).message_id_header;
    let mut result = batch(vec![duplicate], 11);
    result.flag_deltas = vec![UidFlags {
        uid: 10,
        flags: vec!["\\sEeN".into(), "\\FLAGGED".into(), "\\Answered".into()],
    }];
    let canonical = ingest_batch(&store, &other, result, || 100).unwrap();
    assert_eq!(canonical[0].id, original[0].id);
    assert_eq!(count(&store, "messages"), 1);
    assert_eq!(count(&store, "message_folder_memberships"), 2);
    let stored = store.get_message_by_id(&original[0].id).unwrap().unwrap();
    assert!(stored.is_read && stored.is_starred);
    let mut empty = batch(vec![], 11);
    empty.flag_deltas = vec![UidFlags {
        uid: 10,
        flags: vec![],
    }];
    assert!(
        ingest_batch(&store, &other, empty, || panic!("empty batch clock"))
            .unwrap()
            .is_empty()
    );
    let stored = store.get_message_by_id(&original[0].id).unwrap().unwrap();
    assert!(!stored.is_read && !stored.is_starred);
}

#[test]
fn checkpoint_failure_rolls_back_messages_flags_jobs_and_reset_deletions() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir.path().join("mail.db"));
    enable_arrival_features(&store);
    let original = ingest_batch(&store, &state(), batch(vec![message(1)], 2), || 100).unwrap();
    sql(&store, "CREATE TRIGGER synthetic_failure BEFORE UPDATE ON folder_sync_state BEGIN SELECT RAISE(ABORT,'synthetic private fixture'); END;");
    for reset in [false, true] {
        let mut next = FolderSyncState {
            uidnext: 3,
            ..state()
        };
        if reset {
            next.uidvalidity = 8;
        }
        let mut result = batch(vec![message(2)], 3);
        result.uidvalidity_reset = reset;
        result.uidvalidity = next.uidvalidity;
        result.flag_deltas = vec![UidFlags {
            uid: 1,
            flags: vec!["\\Seen".into(), "\\Flagged".into()],
        }];
        let error = ingest_batch(&store, &next, result, || 100).unwrap_err();
        assert_eq!(error, IngestError::StorageUnavailable);
        assert_eq!(error.to_string(), "sync storage unavailable");
        assert_eq!(count(&store, "messages"), 1);
        assert_eq!(count(&store, "rule_jobs"), 0);
        assert_eq!(count(&store, "message_folder_memberships"), 1);
        assert_eq!(
            store.get_folder_sync_state("fixture", "INBOX").unwrap(),
            Some(state())
        );
        let record = store.get_message_by_id(&original[0].id).unwrap().unwrap();
        assert!(!record.is_read && !record.is_starred);
    }
    sql(&store, "DROP TRIGGER synthetic_failure;");
    let next = FolderSyncState {
        uidvalidity: 8,
        uidnext: 1,
        ..state()
    };
    let mut reset = batch(vec![], 3);
    reset.uidvalidity_reset = true;
    reset.uidvalidity = 8;
    ingest_batch(&store, &next, reset, || panic!("empty reset clock")).unwrap();
    assert_eq!(count(&store, "messages"), 0);
    assert_eq!(count(&store, "message_folder_memberships"), 0);
    assert_eq!(
        store.get_folder_sync_state("fixture", "INBOX").unwrap(),
        Some(next)
    );
}
