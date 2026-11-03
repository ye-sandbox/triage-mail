use storage::{
    now_epoch_ms, AccountRecord, DatabaseKey, FolderRecord, FolderSyncState, MessageRecord,
    SqliteMailStore,
};
use tempfile::tempdir;
use triage_ai::{DueDateStatus, ReminderTrigger, ScheduledReminderConfig, TriageBadge};

fn setup_store() -> (tempfile::TempDir, SqliteMailStore) {
    let dir = tempdir().expect("tempdir");
    let key = DatabaseKey::from_bytes([0x42; 32]);
    let store = SqliteMailStore::open(
        dir.path().join("test.db"),
        &key,
        storage::FtsTokenizerMode::Unicode61RemoveDiacritics,
    )
    .expect("store open");
    (dir, store)
}

fn create_account_and_folder(store: &SqliteMailStore, account_id: &str) {
    let now = now_epoch_ms();
    let acct = AccountRecord {
        id: account_id.into(),
        email: format!("{account_id}@example.com"),
        auth_type: "Password".into(),
        created_at_epoch_ms: now,
        updated_at_epoch_ms: now,
    };
    store
        .insert_account_connection(&acct, "{}")
        .expect("insert account");

    let folder = FolderRecord {
        account_id: account_id.into(),
        folder_name: "INBOX".into(),
        display_name: "Inbox".into(),
        delimiter: Some("/".into()),
        role: "inbox".into(),
        is_selectable: true,
        total_messages: Some(0),
        unread_messages: Some(0),
        updated_at_epoch_ms: now,
    };
    store
        .save_folders(account_id, &[folder])
        .expect("save folders");
}

fn make_boleto_message(
    id: &str,
    account_id: &str,
    uid: u32,
    received_ms: i64,
    is_read: bool,
    amount_cents: Option<u64>,
    due_date_iso: Option<&str>,
) -> MessageRecord {
    let badge = TriageBadge::FebrabanBoleto {
        digitable_line: "34191.79001 01043.510047 91020.150008 5 99990000010000".into(),
        amount_cents,
        due_date_iso: due_date_iso.map(Into::into),
    };
    MessageRecord {
        id: id.into(),
        account_id: account_id.into(),
        folder: "INBOX".into(),
        uid,
        message_id_header: Some(format!("<{id}@example.com>")),
        in_reply_to: None,
        references_header: None,
        subject: "Fatura de Energia".into(),
        sender: "cobranca@example.org".into(),
        recipients: "user@example.com".into(),
        received_at_epoch_ms: received_ms,
        is_read,
        has_attachments: false,
        category_badge: Some("Financial".into()),
        triage_badges: vec![badge],
        raw_headers: "".into(),
        text_body: Some("Segue sua fatura em anexo".into()),
        html_body: None,
        list_unsubscribe: None,
        list_unsubscribe_post: None,
        one_click_unsubscribe_supported: false,
        created_at_epoch_ms: received_ms,
        attachments: vec![],
        tags: vec![],
        is_starred: false,
        snooze_until_epoch_ms: None,
        provider_message_id: None,
    }
}

#[test]
fn test_reminder_configs_crud_and_revisions() {
    let (_dir, store) = setup_store();
    create_account_and_folder(&store, "acct1");

    let configs = store.list_reminder_configs().expect("list");
    assert!(configs.is_empty());

    let config = ScheduledReminderConfig {
        version: 1,
        id: "rem-due-1".into(),
        revision: 0,
        name: "Lembrete 1 dia antes".into(),
        enabled: true,
        trigger: ReminderTrigger::DueDate {
            days_before: 1,
            target_hour: 9,
            min_amount_cents: Some(5000),
        },
        destination_ids: vec!["local-desktop".into()],
        account_id: Some("acct1".into()),
    };

    let saved = store.save_reminder_config(config).expect("save");
    assert_eq!(saved.revision, 1);

    let listed = store.list_reminder_configs().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Lembrete 1 dia antes");

    // Optimistic lock check: saving with stale revision fails
    let stale = ScheduledReminderConfig {
        revision: 0,
        ..saved.clone()
    };
    assert!(store.save_reminder_config(stale).is_err());

    // Update with current revision succeeds
    let mut updated = saved.clone();
    updated.name = "Lembrete atualizado".into();
    let saved2 = store.save_reminder_config(updated).expect("update");
    assert_eq!(saved2.revision, 2);

    // Deleting with stale revision fails
    assert!(store.delete_reminder_config(&saved2.id, 1).is_err());

    // Deleting with correct revision succeeds
    store.delete_reminder_config(&saved2.id, 2).expect("delete");
    let after_delete = store.list_reminder_configs().expect("list");
    assert!(after_delete.is_empty());

    // Reusing deleted ID fails
    let reused = ScheduledReminderConfig {
        revision: 0,
        ..saved.clone()
    };
    assert!(store.save_reminder_config(reused).is_err());
}

#[test]
fn test_due_date_items_listing_and_dismissal() {
    let (_dir, store) = setup_store();
    create_account_and_folder(&store, "acct1");
    let state = FolderSyncState {
        account_id: "acct1".into(),
        folder_name: "INBOX".into(),
        uidvalidity: 1,
        uidnext: 200,
        highest_modseq: None,
    };

    let msg1 = make_boleto_message(
        "msg1",
        "acct1",
        101,
        1000,
        false,
        Some(15000),
        Some("2026-11-05"),
    );
    let msg2 = make_boleto_message(
        "msg2",
        "acct1",
        102,
        2000,
        true,
        Some(35000),
        Some("2026-11-02"),
    );
    let msg3 = make_boleto_message("msg3", "acct1", 103, 3000, false, None, None); // no due date

    store
        .commit_sync_batch(&[msg1, msg2, msg3], &[], &state, false)
        .expect("commit batch");

    let items = store.list_due_date_items(None).expect("list items");
    assert_eq!(items.len(), 2);
    // Ordered by due_date_iso ASC
    assert_eq!(items[0].message_id, "msg2");
    assert_eq!(items[0].due_date_iso, "2026-11-02");
    assert_eq!(items[0].status("2026-11-03"), DueDateStatus::Overdue);
    assert!(!items[0].is_dismissed);

    assert_eq!(items[1].message_id, "msg1");
    assert_eq!(items[1].due_date_iso, "2026-11-05");
    assert_eq!(items[1].status("2026-11-03"), DueDateStatus::Upcoming);

    // INVARIANT: Extraction never implies payment. Items start not dismissed and without any paid flag.
    assert!(!items[0].is_dismissed);
    assert!(!items[1].is_dismissed);

    // Explicit dismissal
    store.dismiss_due_date_item("msg1", 5000).expect("dismiss");
    let after_dismiss = store.list_due_date_items(None).expect("list");
    let msg1_item = after_dismiss
        .iter()
        .find(|i| i.message_id == "msg1")
        .unwrap();
    assert!(msg1_item.is_dismissed);
    assert_eq!(msg1_item.status("2026-11-03"), DueDateStatus::Dismissed);
}

#[test]
fn test_scheduled_reminders_scheduling_and_cancellation() {
    let (_dir, store) = setup_store();
    create_account_and_folder(&store, "acct1");
    let state = FolderSyncState {
        account_id: "acct1".into(),
        folder_name: "INBOX".into(),
        uidvalidity: 1,
        uidnext: 200,
        highest_modseq: None,
    };

    // Add reminder config for 1 day before due date at 09:00 UTC-3 (-10800)
    let config_due = ScheduledReminderConfig {
        version: 1,
        id: "rem-due".into(),
        revision: 0,
        name: "1 day before".into(),
        enabled: true,
        trigger: ReminderTrigger::DueDate {
            days_before: 1,
            target_hour: 9,
            min_amount_cents: Some(10000), // R$ 100.00
        },
        destination_ids: vec!["dest-local".into()],
        account_id: None,
    };
    store
        .save_reminder_config(config_due)
        .expect("save due config");

    // Add reminder config for unread interval: 24h
    let config_unread = ScheduledReminderConfig {
        version: 1,
        id: "rem-unread".into(),
        revision: 0,
        name: "Unread 24h".into(),
        enabled: true,
        trigger: ReminderTrigger::UnreadInterval {
            interval_hours: 24,
            min_amount_cents: None,
        },
        destination_ids: vec!["dest-local".into()],
        account_id: None,
    };
    store
        .save_reminder_config(config_unread)
        .expect("save unread config");

    let now = 1_700_000_000_000i64;
    let msg = make_boleto_message(
        "msg10",
        "acct1",
        110,
        now,
        false,
        Some(15000),
        Some("2026-11-05"),
    );
    store
        .commit_sync_batch(&[msg], &[], &state, false)
        .expect("commit batch");

    // Schedule reminders for message
    let scheduled_count = store
        .schedule_reminders_for_message("msg10", now, -10800)
        .expect("schedule");
    assert_eq!(scheduled_count, 2); // 1 DueDate + 1 UnreadInterval

    // Duplicate scheduling is suppressed by unique index
    let duplicate_count = store
        .schedule_reminders_for_message("msg10", now, -10800)
        .expect("duplicate schedule");
    assert_eq!(duplicate_count, 0);

    // Read-before-deadline cancellation: marking msg10 as read cancels unread reminder
    let canceled_count = store
        .cancel_reminders_for_read_messages(&["msg10".into()])
        .expect("cancel unread");
    assert_eq!(canceled_count, 1);

    // When claiming due reminders past due date, only valid un-canceled reminders survive
    let far_future = 2_000_000_000_000i64; // 2033, past the 2026 due date
    let due_jobs = store.claim_due_reminders(far_future, 10).expect("claim");
    assert_eq!(due_jobs.len(), 1);
    assert_eq!(due_jobs[0].trigger_type, "DueDate");

    // Finish job
    store
        .finish_reminder_job(&due_jobs[0].id, far_future)
        .expect("finish");
    let remaining = store.claim_due_reminders(far_future, 10).expect("claim");
    assert!(remaining.is_empty());
}
