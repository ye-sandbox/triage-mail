use super::*;
use crate::DatabaseKey;
const PASS: &[u8] = b"synthetic backup passphrase";
fn store() -> SqliteMailStore {
    SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([7; 32]), Default::default()).unwrap()
}
fn seed(store: &SqliteMailStore) {
    store.conn.lock().unwrap().execute_batch("INSERT INTO accounts VALUES ('a','reader@example.com','AppPassword',0,0);
        INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms,text_body)
        VALUES ('m','a','INBOX',1,'Synthetic subject','sender@example.org','reader@example.com',0,'',0,'cached body');
        INSERT INTO account_connections VALUES ('a','synthetic endpoint excluded');
        INSERT INTO pending_mutations(message_id,payload_json) VALUES ('m','synthetic pending mutation');").unwrap();
}
#[test]
fn roundtrip_excludes_connections_work_and_source_key() {
    let source = store();
    seed(&source);
    let bytes = source.write_backup(Vec::new(), PASS).unwrap();
    assert!(!bytes.windows(11).any(|w| w == b"cached body"));
    let target = store();
    target.read_backup(bytes.as_slice(), PASS).unwrap();
    let conn = target.conn.lock().unwrap();
    assert_eq!(
        conn.query_row("SELECT text_body FROM messages", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "cached body"
    );
    for table in [
        "account_connections",
        "pending_mutations",
        "rule_jobs",
        "alert_jobs",
        "reminder_jobs",
        "alert_schedule_items",
        "conversation_previews",
    ] {
        assert_eq!(
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn failure_rolls_back_and_occupied_target_is_preserved() {
    let source = store();
    seed(&source);
    let bytes = source.write_backup(Vec::new(), PASS).unwrap();
    let target = store();
    assert!(target.read_backup(&bytes[..bytes.len() - 1], PASS).is_err());
    assert_eq!(
        target
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM accounts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    seed(&target);
    assert!(target.read_backup(bytes.as_slice(), PASS).is_err());
    assert_eq!(
        target
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn snapshot_includes_uncheckpointed_wal_and_excludes_later_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.db");
    let key = DatabaseKey::from_bytes([9; 32]);
    let source = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    source
        .conn
        .lock()
        .unwrap()
        .execute_batch("PRAGMA wal_autocheckpoint=0")
        .unwrap();
    seed(&source);
    let connection = crate::connection::open_encrypted_connection(&path, &key).unwrap();
    struct ConcurrentWriter {
        bytes: Vec<u8>,
        conn: Connection,
        changed: bool,
    }
    impl Write for ConcurrentWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            // Header writes precede the read transaction; frame writes follow it.
            if self.bytes.len() >= 24 && !self.changed {
                self.conn
                    .execute("UPDATE messages SET subject='later commit'", [])
                    .unwrap();
                self.changed = true;
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let writer = source
        .write_backup(
            ConcurrentWriter {
                bytes: Vec::new(),
                conn: connection,
                changed: false,
            },
            PASS,
        )
        .unwrap();
    assert!(writer.changed);
    let target = store();
    target.read_backup(writer.bytes.as_slice(), PASS).unwrap();
    assert_eq!(
        target
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT subject FROM messages", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Synthetic subject"
    );
}

#[test]
fn automation_is_disabled_and_destination_secret_references_are_absent() {
    use triage_ai::{
        alerts::{Destination, DestinationKind},
        reminders::{ReminderTrigger, ScheduledReminderConfig},
        rules::{Condition, Rule, RuleAction},
    };
    let source = store();
    seed(&source);
    let destination = Destination {
        id: "destination".into(),
        revision: 1,
        name: "Synthetic local destination".into(),
        kind: DestinationKind::Local,
        enabled: true,
        fields: vec![],
        public_origin: None,
        has_credentials: true,
    };
    source.conn.lock().unwrap().execute("INSERT INTO alert_destinations VALUES ('destination',1,?1,'synthetic-vault-reference',0)", [serde_json::to_string(&destination).unwrap()]).unwrap();
    source
        .save_rule(Rule {
            version: 1,
            id: "rule".into(),
            revision: 0,
            name: "Synthetic rule".into(),
            enabled: true,
            position: 0,
            all: vec![Condition::Account("a".into())],
            any: vec![],
            except: vec![],
            actions: vec![RuleAction::Alert("destination".into())],
            stop_processing: false,
        })
        .unwrap();
    source
        .save_reminder_config(ScheduledReminderConfig {
            version: 1,
            id: "reminder".into(),
            revision: 0,
            name: "Synthetic reminder".into(),
            enabled: true,
            trigger: ReminderTrigger::UnreadInterval {
                interval_hours: 1,
                min_amount_cents: None,
            },
            destination_ids: vec!["destination".into()],
            account_id: Some("a".into()),
        })
        .unwrap();
    let bytes = source.write_backup(Vec::new(), PASS).unwrap();
    let mut reader = BackupReader::new(bytes.as_slice(), PASS).unwrap();
    while let Some(record) = reader.record().unwrap() {
        assert!(!String::from_utf8_lossy(&record).contains("synthetic-vault-reference"));
    }
    let target = store();
    target.read_backup(bytes.as_slice(), PASS).unwrap();
    assert!(!target.list_rules().unwrap()[0].enabled);
    assert!(!target.list_reminder_configs().unwrap()[0].enabled);
    let destination = &target.list_alert_destinations().unwrap()[0];
    assert!(!destination.enabled);
    assert!(!destination.has_credentials);
    assert_eq!(destination.public_origin, None);
}
#[test]
fn unsupported_authenticated_manifest_is_rejected_without_writes() {
    let mut writer = BackupWriter::new(Vec::new(), PASS).unwrap();
    writer
        .record(
            &json(&Manifest {
                version: 99,
                schema: BACKUP_SCHEMA_VERSION,
                tokenizer: Default::default(),
            })
            .unwrap(),
        )
        .unwrap();
    let bytes = writer.finish().unwrap();
    let target = store();
    assert!(target.read_backup(bytes.as_slice(), PASS).is_err());
}
