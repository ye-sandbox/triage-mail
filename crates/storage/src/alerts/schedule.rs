//! Transactional scheduling over consent-filtered payloads. No network or wall clock.
use super::{destinations, error, insert_job};
use crate::{SqliteMailStore, StorageError};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use triage_ai::alerts::{schedule::*, Destination, MAX_AGE_MS};

#[derive(Serialize)]
pub struct ScheduleState {
    pub policy: SchedulePolicy,
    pub deferred: u32,
    pub summarized: u64,
    pub expired: u64,
    pub overflow: u64,
}
pub(crate) fn read_policy(
    conn: &Connection,
    d: &Destination,
) -> Result<SchedulePolicy, StorageError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT definition_json FROM alert_schedule_policies WHERE destination_id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    let mut policy = match json {
        Some(json) => serde_json::from_str(&json).map_err(error)?,
        None => SchedulePolicy::immediate(&d.id, d.revision),
    };
    policy.destination_revision = d.revision;
    Ok(policy)
}
pub(crate) fn revisions(conn: &Connection) -> Result<String, StorageError> {
    let mut stmt = conn
        .prepare("SELECT destination_id,revision FROM alert_schedule_policies")
        .map_err(error)?;
    let values = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))
        .map_err(error)?
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()
        .map_err(error)?;
    serde_json::to_string(&values).map_err(error)
}
pub(crate) struct ScheduledItem<'a> {
    pub event: &'a str,
    pub account: &'a str,
    pub message: &'a str,
    pub payload: &'a str,
    pub created: i64,
    pub urgent: bool,
    pub reminder: Option<&'a str>,
}
pub(crate) fn enqueue(
    tx: &Transaction<'_>,
    d: &Destination,
    item: ScheduledItem<'_>,
) -> Result<(), StorageError> {
    let policy = read_policy(tx, d)?;
    let bypass = item.urgent && policy.urgent_bypass;
    if bypass || (policy.cadence == Cadence::Immediate && policy.quiet.is_none()) {
        insert_job(
            tx,
            d,
            item.event,
            Some(item.account),
            Some(item.message),
            item.payload,
            item.created,
        )?;
        tx.execute(
            "UPDATE alert_jobs SET schedule_revision=?2,urgent=?3 WHERE id=?1",
            params![item.event, policy.revision, bypass],
        )
        .map_err(error)?;
        return Ok(());
    }
    let count: u32 = tx
        .query_row(
            "SELECT count(*) FROM alert_schedule_items WHERE destination_id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .map_err(error)?;
    if count >= 1000 {
        tx.execute(
            "UPDATE alert_schedule_policies SET overflow=overflow+1 WHERE destination_id=?1",
            [&d.id],
        )
        .map_err(error)?;
        return Ok(());
    }
    tx.execute("INSERT OR IGNORE INTO alert_schedule_items(id,destination_id,destination_revision,policy_revision,account_id,message_id,reminder_id,payload,created) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![item.event,d.id,d.revision,policy.revision,item.account,item.message,item.reminder,item.payload,item.created]).map_err(error)?;
    Ok(())
}
impl SqliteMailStore {
    pub fn alert_schedule(&self, id: &str) -> Result<ScheduleState, StorageError> {
        self.transaction(|tx| {
            let d = destinations(tx)?.into_iter().find(|d| d.id == id).ok_or_else(|| error("missing"))?;
            let policy = read_policy(tx, &d)?;
            let deferred = tx.query_row("SELECT count(*) FROM alert_schedule_items WHERE destination_id=?1 AND sealed_job IS NULL", [id], |r| r.get(0)).map_err(error)?;
            let (summarized,expired,overflow) = tx.query_row("SELECT summarized,expired,overflow FROM alert_schedule_policies WHERE destination_id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(error)?.unwrap_or((0,0,0));
            Ok(ScheduleState { policy,deferred,summarized,expired,overflow })
        })
    }
    /// INVARIANT: Policy confirmation binds to both policy and destination revisions.
    pub fn save_alert_schedule(
        &self,
        mut policy: SchedulePolicy,
        consent: bool,
        now: i64,
    ) -> Result<SchedulePolicy, StorageError> {
        if !policy.validate() || (policy.urgent_bypass && !consent) {
            return Err(error("invalid"));
        }
        self.transaction(|tx| {
            let d = destinations(tx)?.into_iter().find(|d| d.id == policy.destination_id && d.revision == policy.destination_revision).ok_or_else(|| error("stale"))?;
            if read_policy(tx,&d)?.revision != policy.revision { return Err(error("stale")); }
            policy.revision += 1;
            tx.execute("UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=?2 WHERE destination_id=?1 AND status='Pending' AND schedule_revision IS NOT NULL", params![d.id,now]).map_err(error)?;
            tx.execute("DELETE FROM alert_schedule_items WHERE destination_id=?1", [&d.id]).map_err(error)?;
            tx.execute("INSERT INTO alert_schedule_policies(destination_id,revision,definition_json) VALUES (?1,?2,?3) ON CONFLICT(destination_id) DO UPDATE SET revision=excluded.revision,definition_json=excluded.definition_json,last_slot=NULL", params![d.id,policy.revision,serde_json::to_string(&policy).map_err(error)?]).map_err(error)?;
            Ok(policy)
        })
    }
    /// Bound work per destination; all missed slots become one immutable delivery.
    pub fn process_alert_schedules(
        &self,
        now: i64,
        calendar: &dyn ScheduleCalendar,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            for d in scheduled_destinations(tx)? {
                tx.execute("UPDATE alert_schedule_cursor SET destination_id=?1 WHERE singleton=1", [&d.id]).map_err(error)?;
                let policy = read_policy(tx,&d)?;
                let last: Option<i64> = tx.query_row("SELECT last_slot FROM alert_schedule_policies WHERE destination_id=?1", [&d.id], |r| r.get(0)).optional().map_err(error)?.flatten();
                expire(tx,&d.id,now)?;
                // INVARIANT: Quiet hours also gate retries and previously sealed routine jobs.
                let end = quiet_end(&policy,now,calendar).ok_or_else(|| error("calendar"))?;
                if end > now {
                    tx.execute("UPDATE alert_jobs SET due=max(due,?2) WHERE destination_id=?1 AND status='Pending' AND urgent=0 AND schedule_revision IS NOT NULL", params![d.id,end]).map_err(error)?;
                    continue;
                }
                seal(tx,&d,&policy,last,now,calendar)?;
            }
            Ok(())
        })
    }
}
// INVARIANT: Persisted round-robin selection bounds each worker tick without starving
// destinations beyond the batch. This is a work budget, not a configuration limit.
fn scheduled_destinations(tx: &Transaction<'_>) -> Result<Vec<Destination>, StorageError> {
    let mut stmt = tx.prepare("SELECT d.definition_json FROM alert_destinations d JOIN alert_schedule_policies p ON p.destination_id=d.id WHERE json_extract(d.definition_json,'$.enabled')=1 ORDER BY (d.id <= (SELECT destination_id FROM alert_schedule_cursor WHERE singleton=1)),d.id LIMIT 32").map_err(error)?;
    let json = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    json.iter()
        .map(|value| serde_json::from_str(value).map_err(error))
        .collect()
}
fn expire(tx: &Transaction<'_>, id: &str, now: i64) -> Result<(), StorageError> {
    let removed = tx.execute("DELETE FROM alert_schedule_items WHERE destination_id=?1 AND sealed_job IS NULL AND created<=?2", params![id,now.saturating_sub(MAX_AGE_MS)]).map_err(error)?;
    tx.execute(
        "UPDATE alert_schedule_policies SET expired=expired+?2 WHERE destination_id=?1",
        params![id, removed],
    )
    .map_err(error)?;
    Ok(())
}
struct Item {
    id: String,
    payload: String,
    created: i64,
}
fn seal(
    tx: &Transaction<'_>,
    d: &Destination,
    policy: &SchedulePolicy,
    last: Option<i64>,
    now: i64,
    calendar: &dyn ScheduleCalendar,
) -> Result<(), StorageError> {
    // INVARIANT: Never replace an ambiguous retry payload or accumulate one delivery
    // per missed slot behind it. Later items stay coalescible until it terminates.
    let pending: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM alert_jobs WHERE destination_id=?1 AND status='Pending' AND schedule_revision IS NOT NULL AND message_id IS NULL)", [&d.id], |r| r.get(0)).map_err(error)?;
    if pending {
        return Ok(());
    }
    let mut stmt = tx.prepare("SELECT id,payload,created FROM alert_schedule_items WHERE destination_id=?1 AND destination_revision=?2 AND policy_revision=?3 AND sealed_job IS NULL ORDER BY created,id LIMIT 1000").map_err(error)?;
    let items = stmt
        .query_map(params![d.id, d.revision, policy.revision], |r| {
            Ok(Item {
                id: r.get(0)?,
                payload: r.get(1)?,
                created: r.get(2)?,
            })
        })
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    let mut ready = Vec::new();
    let mut maximum = last.unwrap_or(i64::MIN);
    for item in items {
        let (due, slot) =
            next_slot(policy, item.created, calendar).ok_or_else(|| error("calendar"))?;
        if due > now {
            continue;
        }
        if policy.cadence != Cadence::Immediate && last.is_some_and(|last| slot <= last) {
            tx.execute("DELETE FROM alert_schedule_items WHERE id=?1", [&item.id])
                .map_err(error)?;
            tx.execute(
                "UPDATE alert_schedule_policies SET expired=expired+1 WHERE destination_id=?1",
                [&d.id],
            )
            .map_err(error)?;
            continue;
        }
        maximum = maximum.max(slot);
        ready.push(item);
    }
    if ready.is_empty() {
        return Ok(());
    }
    use sha2::{Digest, Sha256};
    let event = format!(
        "digest-{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                &d.id,
                policy.revision,
                maximum,
                ready.iter().map(|v| &v.id).collect::<Vec<_>>()
            ))
            .map_err(error)?
        )
    );
    let oldest = ready.iter().map(|v| v.created).min().unwrap_or(now);
    let payload = digest(
        &event,
        now,
        &ready.iter().map(|v| v.payload.as_str()).collect::<Vec<_>>(),
    );
    insert_job(tx, d, &event, None, None, &payload, oldest)?;
    tx.execute(
        "UPDATE alert_jobs SET due=?2,schedule_revision=?3 WHERE id=?1",
        params![event, now, policy.revision],
    )
    .map_err(error)?;
    for item in &ready {
        tx.execute(
            "UPDATE alert_schedule_items SET sealed_job=?2,payload='' WHERE id=?1",
            params![item.id, event],
        )
        .map_err(error)?;
    }
    tx.execute("UPDATE alert_schedule_policies SET last_slot=?2,summarized=summarized+?3 WHERE destination_id=?1", params![d.id,maximum,ready.len()]).map_err(error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatabaseKey;
    use triage_ai::alerts::{DeliveryOutcome, DestinationKind};
    fn store() -> SqliteMailStore {
        let s =
            SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([41; 32]), Default::default())
                .unwrap();
        seed(&s);
        s
    }
    fn seed(s: &SqliteMailStore) {
        s.transaction(|tx| {
            tx.execute_batch("INSERT INTO accounts VALUES ('a','reader@example.com','synthetic',0,0);
                INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms)
                VALUES ('m','a','INBOX',1,'synthetic','sender@example.org','reader@example.com',0,'',0);").map_err(error)?; Ok(())
        }).unwrap();
        s.save_alert_destination(
            Destination {
                id: "d".into(),
                revision: 0,
                name: "Synthetic".into(),
                kind: DestinationKind::Webhook,
                enabled: true,
                fields: vec![],
                public_origin: Some("https://hooks.example.com".into()),
                has_credentials: false,
            },
            None,
            0,
        )
        .unwrap();
    }
    fn policy(s: &SqliteMailStore) -> SchedulePolicy {
        let mut p = s.alert_schedule("d").unwrap().policy;
        p.zone = ScheduleZone::Utc;
        p.cadence = Cadence::Hourly { minute: 0 };
        s.save_alert_schedule(p, false, 0).unwrap()
    }
    fn add(s: &SqliteMailStore, id: &str, created: i64, urgent: bool) {
        s.transaction(|tx| {
            let d = destinations(tx)?.remove(0);
            enqueue(
                tx,
                &d,
                ScheduledItem {
                    event: id,
                    account: "a",
                    message: "m",
                    payload: "{\"schema_version\":1}",
                    created,
                    urgent,
                    reminder: None,
                },
            )
        })
        .unwrap();
    }
    fn count(s: &SqliteMailStore, table: &str) -> u32 {
        s.conn
            .lock()
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    #[test]
    fn resume_coalesces_slots_atomic_retry_identity_and_rollback() {
        let s = store();
        policy(&s);
        for n in 0..10 {
            add(&s, &format!("e{n}"), n * 60 * MINUTE_MS + 1, false);
        }
        s.process_alert_schedules(10 * 60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        let job = s.claim_alert(10 * 60 * MINUTE_MS).unwrap().unwrap();
        let value: serde_json::Value = serde_json::from_str(&job.payload).unwrap();
        assert_eq!(value["total"], 10);
        assert_eq!(job.created, 1);
        s.finish_alert(&job, DeliveryOutcome::Retry, 0, 0, 10 * 60 * MINUTE_MS)
            .unwrap();
        s.process_alert_schedules(10 * 60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        let retry = s
            .claim_alert(10 * 60 * MINUTE_MS + 30_000)
            .unwrap()
            .unwrap();
        assert_eq!(retry.id, job.id);
        assert_eq!(retry.payload, job.payload);
        s.finish_alert(
            &retry,
            DeliveryOutcome::Accepted,
            0,
            0,
            10 * 60 * MINUTE_MS + 30_000,
        )
        .unwrap();
        assert_eq!(count(&s, "alert_schedule_items"), 0);
        add(&s, "rollback", 1, false);
        s.process_alert_schedules(10 * 60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        assert_eq!(s.alert_schedule("d").unwrap().expired, 1);
    }
    #[test]
    fn quiet_urgency_and_manual_test_do_not_flush_pending_work() {
        let s = store();
        let mut p = policy(&s);
        p.quiet = Some(QuietHours { start: 0, end: 420 });
        p.urgent_bypass = true;
        assert!(s.save_alert_schedule(p.clone(), false, 0).is_err());
        s.save_alert_schedule(p, true, 0).unwrap();
        add(&s, "routine", 1, false);
        add(&s, "urgent", 1, true);
        s.enqueue_alert_test("d", "explicit-test", 1).unwrap();
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(s.alert_schedule("d").unwrap().deferred, 1);
        let job = s.claim_alert(60 * MINUTE_MS).unwrap().unwrap();
        assert!(job.id == "urgent" || job.id == "explicit-test");
        s.finish_alert(&job, DeliveryOutcome::Accepted, 0, 0, 60 * MINUTE_MS)
            .unwrap();
        let second = s.claim_alert(60 * MINUTE_MS + 10_000).unwrap().unwrap();
        assert_ne!(second.id, job.id);
        s.finish_alert(
            &second,
            DeliveryOutcome::Accepted,
            0,
            0,
            60 * MINUTE_MS + 10_000,
        )
        .unwrap();
        assert!(s.claim_alert(60 * MINUTE_MS + 20_000).unwrap().is_none());
        s.process_alert_schedules(420 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert!(s
            .claim_alert(420 * MINUTE_MS)
            .unwrap()
            .unwrap()
            .id
            .starts_with("digest-"));
    }
    #[test]
    fn deleting_a_constituent_cancels_sealed_summary_and_clears_payloads() {
        let s = store();
        policy(&s);
        add(&s, "e", 1, false);
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        s.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM messages WHERE id='m'", [])
            .unwrap();
        assert!(s.claim_alert(60 * MINUTE_MS).unwrap().is_none());
        assert_eq!(s.alert_history().unwrap()[0].status, "Canceled");
        assert_eq!(count(&s, "alert_schedule_items"), 0);
        let payload: String = s
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT payload FROM alert_jobs", [], |r| r.get(0))
            .unwrap();
        assert!(payload.is_empty());
    }
    #[test]
    fn revisions_cancel_pending_and_age_is_not_reset_by_summary() {
        let s = store();
        let p = policy(&s);
        add(&s, "e", 1, false);
        let saved = s.save_alert_schedule(p.clone(), false, 2).unwrap();
        assert!(s.save_alert_schedule(p, false, 3).is_err());
        assert_eq!(count(&s, "alert_schedule_items"), 0);
        add(&s, "old", 1, false);
        add(&s, "new", MAX_AGE_MS - 1, false);
        s.process_alert_schedules(MAX_AGE_MS + 1, &UtcCalendar)
            .unwrap();
        assert_eq!(s.alert_schedule("d").unwrap().expired, 1);
        assert_eq!(s.alert_schedule("d").unwrap().deferred, 0);
        assert_eq!(
            s.claim_alert(MAX_AGE_MS + 1).unwrap().unwrap().created,
            MAX_AGE_MS - 1
        );
        s.save_alert_schedule(saved, false, MAX_AGE_MS + 2).unwrap();
        assert_eq!(count(&s, "alert_schedule_items"), 0);
    }
    #[test]
    fn encrypted_restart_and_concurrent_sealing_preserve_one_event() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("schedule.db");
        let key = DatabaseKey::from_bytes([42; 32]);
        {
            let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
            seed(&s);
            policy(&s);
            add(&s, "e", 1, false);
        }
        let s =
            std::sync::Arc::new(SqliteMailStore::open(&path, &key, Default::default()).unwrap());
        let threads = (0..2)
            .map(|_| {
                let s = s.clone();
                std::thread::spawn(move || {
                    s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(count(&s, "alert_jobs"), 1);
        let job = s.claim_alert(60 * MINUTE_MS).unwrap().unwrap();
        let event = job.id.clone();
        drop(s);
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        assert!(s.claim_alert(60 * MINUTE_MS + 29_999).unwrap().is_none());
        let retry = s.claim_alert(60 * MINUTE_MS + 30_000).unwrap().unwrap();
        assert_eq!(retry.id, event);
        assert_eq!(retry.payload, job.payload);
        assert_eq!(retry.attempts, 2);
        assert!(!std::fs::read(path)
            .unwrap()
            .windows(16)
            .any(|w| w == b"SQLite format 3\0"));
    }
    #[test]
    fn seal_failure_rolls_back_membership_and_watermark() {
        let s = store();
        policy(&s);
        add(&s, "e", 1, false);
        s.conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_seal BEFORE INSERT ON alert_jobs BEGIN SELECT RAISE(ABORT,'synthetic failure'); END").unwrap();
        assert!(s
            .process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .is_err());
        assert_eq!(s.alert_schedule("d").unwrap().deferred, 1);
        assert_eq!(s.alert_schedule("d").unwrap().summarized, 0);
        s.conn
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_seal")
            .unwrap();
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
    }
    #[test]
    fn read_cancels_unread_reminders_already_waiting_in_a_summary() {
        let s = store();
        policy(&s);
        s.transaction(|tx| {
            tx.execute_batch("INSERT INTO reminder_jobs(id,message_id,account_id,trigger_type,due_epoch_ms,destination_ids,status,created_at_epoch_ms) VALUES ('rem','m','a','UnreadInterval',0,'[\"d\"]','Dispatched',0)").map_err(error)?;
            let d=destinations(tx)?.remove(0);
            enqueue(tx,&d,ScheduledItem {event:"rem-rem-d",account:"a",message:"m",payload:"{}",created:1,urgent:false,reminder:Some("rem")})
        }).unwrap();
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        s.conn
            .lock()
            .unwrap()
            .execute("UPDATE messages SET is_read=1 WHERE id='m'", [])
            .unwrap();
        assert!(s.claim_alert(60 * MINUTE_MS).unwrap().is_none());
        assert_eq!(count(&s, "alert_schedule_items"), 0);
        assert_eq!(s.alert_history().unwrap()[0].status, "Canceled");
    }
    #[test]
    fn destination_batches_are_fair_and_actual_lease_checks_quiet_hours() {
        let s = store();
        let mut p = policy(&s);
        p.cadence = Cadence::Immediate;
        s.save_alert_schedule(p, false, 0).unwrap();
        for n in 0..33 {
            let d = s
                .save_alert_destination(
                    Destination {
                        id: format!("x{n:02}"),
                        revision: 0,
                        name: "Synthetic".into(),
                        kind: DestinationKind::Local,
                        enabled: true,
                        fields: vec![],
                        public_origin: None,
                        has_credentials: false,
                    },
                    None,
                    0,
                )
                .unwrap();
            let mut p = SchedulePolicy::immediate(&d.id, d.revision);
            p.zone = ScheduleZone::Utc;
            p.cadence = Cadence::Hourly { minute: 0 };
            s.save_alert_schedule(p, false, 0).unwrap();
            s.transaction(|tx| {
                enqueue(
                    tx,
                    &d,
                    ScheduledItem {
                        event: &format!("e{n}"),
                        account: "a",
                        message: "m",
                        payload: "{}",
                        created: 1,
                        urgent: false,
                        reminder: None,
                    },
                )
            })
            .unwrap();
        }
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 31); // Immediate destination consumes one work slot.
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 33);
        // A retry must be gated even when its destination was not just staged.
        let mut p = s.alert_schedule("d").unwrap().policy;
        p.quiet = Some(QuietHours {
            start: 120,
            end: 180,
        });
        s.save_alert_schedule(p, false, 0).unwrap();
        s.transaction(|tx| {
            let d = destinations(tx)?.into_iter().find(|d| d.id == "d").unwrap();
            insert_job(tx, &d, "actual-candidate", Some("a"), Some("m"), "{}", 0)?;
            tx.execute(
                "UPDATE alert_jobs SET schedule_revision=1 WHERE id='actual-candidate'",
                [],
            )
            .map_err(error)?;
            tx.execute("DELETE FROM alert_jobs WHERE destination_id!='d'", [])
                .map_err(error)?;
            Ok(())
        })
        .unwrap();
        assert!(s.claim_alert(120 * MINUTE_MS).unwrap().is_none());
        assert!(s.claim_alert(180 * MINUTE_MS).unwrap().is_some());
    }
    #[test]
    fn overflow_is_counted_without_extra_jobs() {
        let s = store();
        policy(&s);
        for n in 0..1002 {
            add(&s, &format!("e{n}"), 1, false);
        }
        let state = s.alert_schedule("d").unwrap();
        assert_eq!(state.deferred, 1000);
        assert_eq!(state.overflow, 2);
        s.process_alert_schedules(60 * MINUTE_MS, &UtcCalendar)
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        let job = s.claim_alert(60 * MINUTE_MS).unwrap().unwrap();
        assert!(job.payload.len() <= triage_ai::alerts::MAX_PAYLOAD_BYTES);
        s.finish_alert(&job, DeliveryOutcome::Accepted, 0, 0, 60 * MINUTE_MS)
            .unwrap();
        assert_eq!(count(&s, "alert_schedule_items"), 0);
    }
}
