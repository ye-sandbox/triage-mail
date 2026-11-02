//! SQLCipher delivery queue. Secrets and network I/O never enter this module.
use crate::{SqliteMailStore, StorageError};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use triage_ai::alerts::{DeliveryOutcome, Destination, MAX_AGE_MS};

fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("alert storage operation failed".into())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryHistory {
    pub event_id: String,
    pub destination_id: String,
    pub status: String,
    pub attempts: u32,
    pub created: i64,
    pub finished: Option<i64>,
}
/// Internal adapter request; deliberately not serializable or Debug (contains mail payload).
pub struct DeliveryJob {
    pub id: String,
    pub destination: Destination,
    pub secret_key: Option<String>,
    pub payload: String,
    pub attempts: u32,
    pub created: i64,
}
pub(crate) fn destinations(conn: &Connection) -> Result<Vec<Destination>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT definition_json FROM alert_destinations ORDER BY id")
        .map_err(error)?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(error)?;
    rows.map(|r| serde_json::from_str(&r.map_err(error)?).map_err(error))
        .collect()
}
pub(crate) fn revisions(conn: &Connection) -> Result<String, StorageError> {
    let map: std::collections::BTreeMap<_, _> = destinations(conn)?
        .into_iter()
        .filter(|d| d.enabled)
        .map(|d| (d.id, d.revision))
        .collect();
    serde_json::to_string(&map).map_err(error)
}
impl SqliteMailStore {
    pub fn list_alert_destinations(&self) -> Result<Vec<Destination>, StorageError> {
        self.transaction(|tx| destinations(tx))
    }
    pub fn alert_secret_key(&self, id: &str) -> Result<Option<String>, StorageError> {
        self.transaction(|tx| {
            tx.query_row(
                "SELECT secret_key FROM alert_destinations WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(error)
        })
    }
    /// Register intent BEFORE vault write; unreferenced revisions remain discoverable after crashes.
    pub fn register_alert_secret(&self, key: &str) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute("INSERT INTO alert_secret_intents VALUES (?1)", [key])
                .map_err(error)?;
            Ok(())
        })
    }
    pub fn orphan_alert_secrets(&self) -> Result<Vec<String>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx.prepare("SELECT secret_key FROM alert_secret_intents WHERE secret_key NOT IN (SELECT secret_key FROM alert_destinations WHERE secret_key IS NOT NULL)").map_err(error)?;
            let rows=stmt.query_map([],|r|r.get(0)).map_err(error)?.collect::<Result<_,_>>().map_err(error)?; Ok(rows)
        })
    }
    pub fn forget_alert_secret(&self, key: &str) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute(
                "DELETE FROM alert_secret_intents WHERE secret_key=?1",
                [key],
            )
            .map_err(error)?;
            Ok(())
        })
    }
    pub fn save_alert_destination(
        &self,
        mut d: Destination,
        key: Option<&str>,
        now: i64,
    ) -> Result<Destination, StorageError> {
        if !d.validate() {
            return Err(error("invalid"));
        }
        self.transaction(|tx| {
            let deleted:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM alert_deleted_ids WHERE id=?1)",[&d.id],|r|r.get(0)).map_err(error)?;
            let current:Option<u32>=tx.query_row("SELECT revision FROM alert_destinations WHERE id=?1",[&d.id],|r|r.get(0)).optional().map_err(error)?;
            if deleted || current.unwrap_or(0)!=d.revision { return Err(error("stale")); }
            d.revision+=1;
            tx.execute("INSERT INTO alert_destinations(id,revision,definition_json,secret_key) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,definition_json=excluded.definition_json,secret_key=excluded.secret_key",params![d.id,d.revision,serde_json::to_string(&d).map_err(error)?,key]).map_err(error)?;
            tx.execute("UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=?2 WHERE destination_id=?1 AND status='Pending'",params![d.id,now]).map_err(error)?;
            Ok(d)
        })
    }
    pub fn delete_alert_destination(&self, id: &str, revision: u32) -> Result<(), StorageError> {
        self.transaction(|tx| {
            if tx
                .execute(
                    "DELETE FROM alert_destinations WHERE id=?1 AND revision=?2",
                    params![id, revision],
                )
                .map_err(error)?
                != 1
            {
                return Err(error("stale"));
            }
            tx.execute("INSERT INTO alert_deleted_ids VALUES (?1)", [id])
                .map_err(error)?;
            Ok(())
        })
    }
    pub fn enqueue_alert_test(
        &self,
        destination: &str,
        event: &str,
        now: i64,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            let d=destinations(tx)?.into_iter().find(|d| d.id==destination && d.enabled).ok_or_else(||error("disabled"))?;
            let payload=serde_json::json!({"schema_version":1,"event_id":event,"timestamp_epoch_ms":now,"test":true}).to_string();
            insert_job(tx,&d,event,None,None,&payload,now)
        })
    }
    /// Replay does not evaluate mutation actions or modify automatic eligibility/deduplication.
    pub fn replay_alerts(
        &self,
        ids: &[String],
        destination: &str,
        replay: &str,
        now: i64,
    ) -> Result<(), StorageError> {
        if ids.is_empty() || ids.len() > 100 {
            return Err(error("selection"));
        }
        self.transaction(|tx| {
            let d = destinations(tx)?
                .into_iter()
                .find(|d| d.id == destination && d.enabled)
                .ok_or_else(|| error("disabled"))?;
            for (index, id) in ids.iter().enumerate() {
                let facts = crate::rules::facts(tx, id)?;
                let event = format!("{replay}-{index}");
                let payload = triage_ai::alerts::payload(&event, now, &d.fields, &facts);
                insert_job(
                    tx,
                    &d,
                    &event,
                    Some(&facts.account_id),
                    Some(id),
                    &payload,
                    now,
                )?;
            }
            Ok(())
        })
    }
    pub fn alert_history(&self) -> Result<Vec<DeliveryHistory>, StorageError> {
        self.transaction(|tx| {
            let mut stmt=tx.prepare("SELECT id,destination_id,status,attempts,created,finished FROM alert_jobs ORDER BY created DESC,id LIMIT 200").map_err(error)?;
            let rows=stmt.query_map([],|r|Ok(DeliveryHistory {event_id:r.get(0)?,destination_id:r.get(1)?,status:r.get(2)?,attempts:r.get(3)?,created:r.get(4)?,finished:r.get(5)?})).map_err(error)?.collect::<Result<_,_>>().map_err(error)?; Ok(rows)
        })
    }
    /// Transactional lease and persistent rate gate; request I/O occurs after this commits.
    pub fn claim_alert(&self, now: i64) -> Result<Option<DeliveryJob>, StorageError> {
        self.transaction(|tx| {
            tx.execute("UPDATE alert_jobs SET status='Expired',payload='',finished=?1,lease_until=0 WHERE status='Pending' AND (created<=?2 OR (attempts>=8 AND lease_until<=?1))",params![now,now.saturating_sub(MAX_AGE_MS)]).map_err(error)?;
            prune(tx,now)?;
            let id:Option<String>=tx.query_row("SELECT j.id FROM alert_jobs j JOIN alert_destinations d ON d.id=j.destination_id AND d.revision=j.revision WHERE j.status='Pending' AND j.due<=?1 AND j.lease_until<=?1 AND d.next_start<=?1 AND NOT EXISTS(SELECT 1 FROM alert_jobs other WHERE other.destination_id=j.destination_id AND other.status='Pending' AND other.lease_until>?1) ORDER BY j.due,j.id LIMIT 1",[now],|r|r.get(0)).optional().map_err(error)?;
            let Some(id)=id else {return Ok(None)};
            let (json,key,payload,attempts,created):(String,Option<String>,String,u32,i64)=tx.query_row("SELECT d.definition_json,d.secret_key,j.payload,j.attempts,j.created FROM alert_jobs j JOIN alert_destinations d ON d.id=j.destination_id WHERE j.id=?1",[&id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(error)?;
            let destination:Destination=serde_json::from_str(&json).map_err(error)?;
            if !destination.enabled {return Ok(None);}
            tx.execute("UPDATE alert_jobs SET attempts=attempts+1,lease_until=?2 WHERE id=?1",params![id,now+30_000]).map_err(error)?;
            tx.execute("UPDATE alert_destinations SET next_start=?2 WHERE id=?1",params![destination.id,now+10_000]).map_err(error)?;
            Ok(Some(DeliveryJob {id,destination,secret_key:key,payload,attempts:attempts+1,created}))
        })
    }
    pub fn finish_alert(
        &self,
        job: &DeliveryJob,
        outcome: DeliveryOutcome,
        retry_after_ms: i64,
        jitter_ms: i64,
        now: i64,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            let retry=outcome==DeliveryOutcome::Retry && job.attempts<8 && now<job.created.saturating_add(MAX_AGE_MS);
            let backoff=(30_000i64 * (1i64 << job.attempts.saturating_sub(1).min(7))).min(3_600_000);
            let due=now.saturating_add((backoff+jitter_ms.clamp(0,5000)).max(retry_after_ms.clamp(0,3_600_000)));
            let status=if retry {"Pending"} else {match outcome {DeliveryOutcome::Accepted=>"Accepted",DeliveryOutcome::Unavailable=>"Unavailable",_=>"Failed"}};
            // INVARIANT: A stale lease completion cannot revive canceled or superseded work.
            let changed = tx.execute("UPDATE alert_jobs SET status=?2,due=?3,lease_until=0,payload=CASE WHEN ?2='Pending' THEN payload ELSE '' END,finished=CASE WHEN ?2='Pending' THEN NULL ELSE ?4 END WHERE id=?1 AND attempts=?5 AND revision=?6 AND status='Pending'",params![job.id,status,due,now,job.attempts,job.destination.revision]).map_err(error)?;
            // INVARIANT: Provider rate limits can cover shared bots/webhooks across destinations.
            // Conservatively pause the entire provider kind, without storing a secret fingerprint.
            if changed == 1 && outcome == DeliveryOutcome::Retry && matches!(job.destination.kind,
                triage_ai::alerts::DestinationKind::Telegram | triage_ai::alerts::DestinationKind::Discord) {
                tx.execute("UPDATE alert_destinations SET next_start=max(next_start,?2) WHERE json_extract(definition_json,'$.kind')=?1",
                    params![match job.destination.kind {triage_ai::alerts::DestinationKind::Telegram=>"Telegram",_=>"Discord"},due]).map_err(error)?;
            }
            Ok(())
        })
    }
}
fn prune(tx: &Transaction<'_>, now: i64) -> Result<(), StorageError> {
    tx.execute(
        "DELETE FROM alert_jobs WHERE status!='Pending' AND finished<?1",
        [now.saturating_sub(30 * MAX_AGE_MS)],
    )
    .map_err(error)?;
    tx.execute("DELETE FROM alert_jobs WHERE status!='Pending' AND id NOT IN (SELECT id FROM (SELECT id,row_number() OVER (PARTITION BY destination_id ORDER BY created DESC,id) AS n FROM alert_jobs WHERE status!='Pending') WHERE n<=1000)",[]).map_err(error)?;
    Ok(())
}
fn insert_job(
    tx: &Transaction<'_>,
    d: &Destination,
    event: &str,
    account: Option<&str>,
    message: Option<&str>,
    payload: &str,
    now: i64,
) -> Result<(), StorageError> {
    tx.execute("INSERT INTO alert_jobs(id,destination_id,revision,account_id,message_id,payload,created,due) VALUES (?1,?2,?3,?4,?5,?6,?7,?7)",params![event,d.id,d.revision,account,message,payload,now]).map_err(error)?;
    Ok(())
}
pub(crate) fn enqueue_auto(
    tx: &Transaction<'_>,
    id: &str,
    destination: &str,
    now: i64,
) -> Result<(), StorageError> {
    let (snapshot, key): (String, String) = tx
        .query_row(
            "SELECT alert_revisions,event_key FROM rule_jobs WHERE message_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(error)?;
    let revisions: std::collections::BTreeMap<String, u32> =
        serde_json::from_str(&snapshot).map_err(error)?;
    let Some(d) = destinations(tx)?
        .into_iter()
        .find(|d| d.id == destination && d.enabled && revisions.get(&d.id) == Some(&d.revision))
    else {
        return Ok(());
    };
    if key.is_empty() {
        return Ok(());
    }
    let facts = crate::rules::facts(tx, id)?;
    if tx
        .execute(
            "INSERT OR IGNORE INTO alert_dedup VALUES (?1,?2,?3)",
            params![facts.account_id, d.id, key],
        )
        .map_err(error)?
        == 0
    {
        return Ok(());
    }
    use sha2::{Digest, Sha256};
    let event = format!(
        "auto-{:x}",
        Sha256::digest(serde_json::to_vec(&(&facts.account_id, &d.id, &key)).map_err(error)?)
    );
    let payload = triage_ai::alerts::payload(&event, now, &d.fields, &facts);
    insert_job(
        tx,
        &d,
        &event,
        Some(&facts.account_id),
        Some(id),
        &payload,
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatabaseKey;
    use triage_ai::alerts::DestinationKind;
    fn store() -> SqliteMailStore {
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([32; 32]), Default::default())
            .unwrap()
    }
    pub(super) fn destination() -> Destination {
        Destination {
            id: "dest".into(),
            revision: 0,
            name: "Synthetic".into(),
            kind: DestinationKind::Local,
            enabled: true,
            fields: vec![],
            public_origin: None,
            has_credentials: false,
        }
    }
    #[test]
    fn restart_lease_rate_gate_retries_and_history_are_durable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("alerts.db");
        let key = DatabaseKey::from_bytes([33; 32]);
        {
            let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
            s.save_alert_destination(destination(), None, 0).unwrap();
            s.enqueue_alert_test("dest", "test-a", 0).unwrap();
            s.enqueue_alert_test("dest", "test-b", 0).unwrap();
            assert_eq!(s.claim_alert(0).unwrap().unwrap().attempts, 1);
            assert!(s.claim_alert(20_000).unwrap().is_none());
        }
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        let job = s.claim_alert(30_000).unwrap().unwrap();
        assert_eq!(job.id, "test-a");
        assert_eq!(job.attempts, 2);
        s.finish_alert(&job, DeliveryOutcome::Retry, 120_000, 0, 30_000)
            .unwrap();
        assert!(s.claim_alert(39_999).unwrap().is_none());
        let other = s.claim_alert(40_000).unwrap().unwrap();
        assert_eq!(other.id, "test-b");
        s.finish_alert(&other, DeliveryOutcome::Accepted, 0, 0, 40_000)
            .unwrap();
        assert!(s.claim_alert(149_999).unwrap().is_none());
        let again = s.claim_alert(150_000).unwrap().unwrap();
        assert_eq!(again.id, "test-a");
        assert_eq!(again.payload, job.payload);
        s.finish_alert(&again, DeliveryOutcome::Accepted, 0, 0, 150_000)
            .unwrap();
        let history = serde_json::to_string(&s.alert_history().unwrap()).unwrap();
        assert!(!history.contains("payload"));
        assert!(!history.contains("schema_version"));
        let payload: String = s
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT payload FROM alert_jobs WHERE id='test-a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(payload.is_empty());
    }
    #[test]
    fn disable_cancels_and_stale_completion_never_revives_work() {
        let s = store();
        let mut d = s.save_alert_destination(destination(), None, 0).unwrap();
        s.enqueue_alert_test("dest", "a", 0).unwrap();
        let job = s.claim_alert(0).unwrap().unwrap();
        d.enabled = false;
        let d = s.save_alert_destination(d, None, 1).unwrap();
        s.finish_alert(&job, DeliveryOutcome::Retry, 0, 0, 2)
            .unwrap();
        assert_eq!(s.alert_history().unwrap()[0].status, "Canceled");
        assert!(s.enqueue_alert_test("dest", "b", 3).is_err());
        let mut enabled = d;
        enabled.enabled = true;
        s.save_alert_destination(enabled, None, 4).unwrap();
        assert!(s.claim_alert(99_999).unwrap().is_none());
        assert!(s.save_alert_destination(destination(), None, 5).is_err());
    }
    #[test]
    fn attempts_and_age_are_bounded_and_finished_history_prunes() {
        let s = store();
        s.save_alert_destination(destination(), None, 0).unwrap();
        s.enqueue_alert_test("dest", "a", 0).unwrap();
        let mut now = 0;
        for expected in 1..=8 {
            let j = s.claim_alert(now).unwrap().unwrap();
            assert_eq!(j.attempts, expected);
            s.finish_alert(&j, DeliveryOutcome::Retry, 0, 0, now)
                .unwrap();
            now += 3_600_000;
        }
        assert_eq!(s.alert_history().unwrap()[0].status, "Failed");
        s.enqueue_alert_test("dest", "b", 0).unwrap();
        assert!(s.claim_alert(MAX_AGE_MS).unwrap().is_none());
        assert!(s
            .alert_history()
            .unwrap()
            .iter()
            .any(|h| h.status == "Expired"));
        s.claim_alert(32 * MAX_AGE_MS).unwrap();
        assert!(s.alert_history().unwrap().is_empty());
    }
    #[test]
    fn staged_secrets_are_discoverable_until_reference_commits() {
        let s = store();
        s.register_alert_secret("new").unwrap();
        assert_eq!(s.orphan_alert_secrets().unwrap(), ["new"]);
        s.save_alert_destination(destination(), Some("new"), 0)
            .unwrap();
        assert!(s.orphan_alert_secrets().unwrap().is_empty());
        s.delete_alert_destination("dest", 1).unwrap();
        assert_eq!(s.orphan_alert_secrets().unwrap(), ["new"]);
        s.forget_alert_secret("new").unwrap();
        assert!(s.orphan_alert_secrets().unwrap().is_empty());
        assert!(s.save_alert_destination(destination(), None, 0).is_err());
    }
    #[test]
    fn connector_kinds_reopen_and_provider_cooldown_blocks_other_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connector.db");
        let key = DatabaseKey::from_bytes([39; 32]);
        {
            let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
            for (id, kind) in [
                ("a", DestinationKind::Telegram),
                ("b", DestinationKind::Telegram),
                ("c", DestinationKind::Discord),
            ] {
                let mut d = destination();
                d.id = id.into();
                d.kind = kind;
                s.save_alert_destination(d, Some("opaque-secret"), 0)
                    .unwrap();
                s.enqueue_alert_test(id, id, 0).unwrap();
            }
            let job = s.claim_alert(0).unwrap().unwrap();
            assert_eq!(job.id, "a");
            s.finish_alert(&job, DeliveryOutcome::Retry, 120_000, 0, 0)
                .unwrap();
        }
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        assert_eq!(s.list_alert_destinations().unwrap().len(), 3);
        let discord = s.claim_alert(10_000).unwrap().unwrap();
        assert_eq!(discord.id, "c");
        s.finish_alert(&discord, DeliveryOutcome::Accepted, 0, 0, 10_000)
            .unwrap();
        assert!(s.claim_alert(119_999).unwrap().is_none());
        let retry = s.claim_alert(120_000).unwrap().unwrap();
        assert_eq!(retry.id, "b");
        assert_eq!(retry.destination.kind, DestinationKind::Telegram);
        let mut d = retry.destination.clone();
        d.enabled = false;
        s.save_alert_destination(d, Some("opaque-secret"), 120_001)
            .unwrap();
        s.finish_alert(&retry, DeliveryOutcome::Retry, 999_999, 0, 120_002)
            .unwrap();
        assert_eq!(s.claim_alert(130_000).unwrap().unwrap().id, "a");
    }
}
