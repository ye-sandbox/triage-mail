//! Encrypted rule definitions, simulation and durable post-ingestion evaluation.
use crate::{desktop::LocalMutation, FolderSyncState, SqliteMailStore, StorageError};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use triage_ai::rules::{
    evaluate, sender_address, Condition, MessageFacts, Rule, RuleAction, RulePlan, MAX_RULES,
    MAX_SIMULATION_MESSAGES,
};

fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("rule storage operation failed".into())
}
fn invalid() -> StorageError {
    StorageError::Query("invalid or stale rule request".into())
}

pub(crate) fn read_rules(conn: &Connection) -> Result<Vec<Rule>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT definition_json FROM local_rules ORDER BY position,id")
        .map_err(error)?;
    let jsons = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    jsons
        .iter()
        .map(|s| serde_json::from_str(s).map_err(error))
        .collect()
}

pub(crate) fn facts(conn: &Connection, id: &str) -> Result<MessageFacts, StorageError> {
    let (mut f, badges): (MessageFacts, String) = conn
        .query_row(
            "SELECT account_id,sender,subject,text_body,triage_badges FROM messages WHERE id=?1",
            [id],
            |r| {
                let sender: String = r.get(1)?;
                Ok((
                    MessageFacts {
                        account_id: r.get(0)?,
                        sender_address: sender_address(&sender),
                        subject: r.get(2)?,
                        body: r.get(3)?,
                        ..Default::default()
                    },
                    r.get(4)?,
                ))
            },
        )
        .map_err(error)?;
    let mut stmt = conn
        .prepare("SELECT filename,content_type FROM attachments WHERE message_id=?1 ORDER BY id")
        .map_err(error)?;
    f.attachments = stmt
        .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(error)?
        .collect::<Result<_, _>>()
        .map_err(error)?;
    for badge in serde_json::from_str::<Vec<triage_ai::TriageBadge>>(&badges).map_err(error)? {
        if let triage_ai::TriageBadge::FebrabanBoleto {
            amount_cents,
            due_date_iso,
            ..
        } = badge
        {
            f.amounts_cents.extend(amount_cents);
            f.due_dates.extend(due_date_iso);
        }
    }
    Ok(f)
}

impl SqliteMailStore {
    pub fn list_rules(&self) -> Result<Vec<Rule>, StorageError> {
        self.transaction(|tx| read_rules(tx))
    }

    /// Revision zero creates; updates compare-and-swap the last observed revision.
    pub fn save_rule(&self, mut rule: Rule) -> Result<Rule, StorageError> {
        if !rule.validate() {
            return Err(invalid());
        }
        self.transaction(|tx| {
            // INVARIANT: A deleted ID cannot reuse an old revision (ABA stale-write race).
            let deleted: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM rule_deleted_ids WHERE id=?1)", [&rule.id], |r| r.get(0)).map_err(error)?;
            if deleted { return Err(invalid()); }
            let current: Option<u32> = tx.query_row("SELECT revision FROM local_rules WHERE id=?1", [&rule.id], |r| r.get(0)).optional().map_err(error)?;
            if current.unwrap_or(0) != rule.revision { return Err(invalid()); }
            if current.is_none() && read_rules(tx)?.len() >= MAX_RULES { return Err(invalid()); }
            for c in rule.all.iter().chain(&rule.any).chain(&rule.except) {
                if let Condition::Account(id) = c {
                    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",[id],|r| r.get(0)).map_err(error)?;
                    if !exists { return Err(invalid()); }
                }
            }
            for action in &rule.actions {
                if let RuleAction::Alert(id) | RuleAction::UrgentAlert(id) = action {
                    if !crate::alerts::destinations(tx)?.iter().any(|d| d.id == *id) { return Err(invalid()); }
                }
            }
            rule.revision += 1;
            let json = serde_json::to_string(&rule).map_err(error)?;
            tx.execute("INSERT INTO local_rules VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,position=excluded.position,definition_json=excluded.definition_json",params![rule.id,rule.revision,rule.position,json]).map_err(error)?;
            Ok(rule)
        })
    }

    pub fn delete_rule(&self, id: &str, revision: u32) -> Result<(), StorageError> {
        self.transaction(|tx| {
            if tx
                .execute(
                    "DELETE FROM local_rules WHERE id=?1 AND revision=?2",
                    params![id, revision],
                )
                .map_err(error)?
                != 1
            {
                return Err(invalid());
            }
            tx.execute("INSERT INTO rule_deleted_ids VALUES (?1)", [id])
                .map_err(error)?;
            Ok(())
        })
    }

    /// Reorder is atomic and rejects a stale/incomplete list of rule revisions.
    pub fn reorder_rules(&self, revisions: &[(String, u32)]) -> Result<(), StorageError> {
        if revisions.len() > MAX_RULES {
            return Err(invalid());
        }
        self.transaction(|tx| {
            let mut rules = read_rules(tx)?;
            if rules.len() != revisions.len() {
                return Err(invalid());
            }
            let mut seen = std::collections::HashSet::new();
            for (position, (id, revision)) in revisions.iter().enumerate() {
                if !seen.insert(id) {
                    return Err(invalid());
                }
                let rule = rules
                    .iter_mut()
                    .find(|r| r.id == *id && r.revision == *revision)
                    .ok_or_else(invalid)?;
                if rule.revision >= i32::MAX as u32 - 1 {
                    return Err(invalid());
                }
                rule.position = position as u32;
                rule.revision += 1;
                tx.execute(
                    "UPDATE local_rules SET revision=?1,position=?2,definition_json=?3 WHERE id=?4",
                    params![
                        rule.revision,
                        rule.position,
                        serde_json::to_string(rule).map_err(error)?,
                        id
                    ],
                )
                .map_err(error)?;
            }
            Ok(())
        })
    }

    /// A candidate draft can replace its saved rule for preview; never persisted.
    pub fn simulate_rules(
        &self,
        ids: &[String],
        draft: Option<Rule>,
    ) -> Result<Vec<(String, RulePlan)>, StorageError> {
        if ids.is_empty()
            || ids.len() > MAX_SIMULATION_MESSAGES
            || draft.as_ref().is_some_and(|r| !r.validate())
        {
            return Err(invalid());
        }
        self.transaction(|tx| {
            let mut rules = read_rules(tx)?;
            if let Some(draft) = draft {
                rules.retain(|r| r.id != draft.id);
                rules.push(draft);
            }
            ids.iter()
                .map(|id| Ok((id.clone(), evaluate(&rules, &facts(tx, id)?))))
                .collect()
        })
    }

    pub fn rule_draft_from_message(&self, id: &str, rule_id: String) -> Result<Rule, StorageError> {
        self.transaction(|tx| {
            let f = facts(tx, id)?;
            Ok(Rule {
                version: 1,
                id: rule_id,
                revision: 0,
                name: String::new(),
                enabled: false,
                position: read_rules(tx)?.len() as u32,
                all: vec![
                    Condition::Account(f.account_id),
                    Condition::Sender(f.sender_address.ok_or_else(invalid)?),
                ],
                any: vec![],
                except: vec![],
                actions: vec![RuleAction::Star],
                stop_processing: false,
            })
        })
    }

    /// Called after SELECT and before ingestion, even on empty/reset batches.
    /// INVARIANT: The first server UID frontier survives interruption of history imports.
    pub fn establish_rule_baseline(
        &self,
        account: &str,
        folder: &str,
        validity: u32,
        server_uidnext: u32,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute("INSERT INTO rule_baselines VALUES (?1,?2,?3,?4) ON CONFLICT(account_id,folder) DO UPDATE SET uidvalidity=excluded.uidvalidity,frontier=excluded.frontier WHERE rule_baselines.uidvalidity!=excluded.uidvalidity",params![account,folder,validity,if server_uidnext == 0 {u32::MAX} else {server_uidnext.saturating_sub(1)}]).map_err(error)?;
            Ok(())
        })
    }

    /// Headless adapter supplies only the existing remote payload serialization.
    /// Marker, local actions and remote entries commit together or all roll back.
    /// Returns changed account IDs for decoupled refresh events.
    pub fn process_rule_jobs(
        &self,
        limit: usize,
        remote: impl Fn(&str, &str, &str, u32, u32, &RuleAction) -> Result<String, StorageError>,
    ) -> Result<Vec<String>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx.prepare(
                "SELECT message_id,rules_json FROM rule_jobs WHERE processed=0 ORDER BY message_id LIMIT ?1"
            ).map_err(error)?;
            let jobs = stmt.query_map([limit.min(100) as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }).map_err(error)?.collect::<Result<Vec<_>, _>>().map_err(error)?;
            let current = read_rules(tx)?;
            let mut changed = Vec::new();
            for (id, json) in jobs {
                let scheduled: Vec<Rule> = serde_json::from_str(&json).map_err(error)?;
                // INVARIANT: Edits invalidate pending revisions and never replay old mail.
                let active: Vec<_> = scheduled.into_iter().filter(|r| {
                    current.iter().any(|c| c.id == r.id && c.revision == r.revision && c.enabled)
                }).collect();
                let message_facts = facts(tx, &id)?;
                let plan = evaluate(&active, &message_facts);
                apply_plan(tx, &id, &plan, &remote)?;
                tx.execute("UPDATE rule_jobs SET processed=1,rules_json='[]' WHERE message_id=?1", [&id]).map_err(error)?;
                if !plan.actions.is_empty() && !changed.contains(&message_facts.account_id) {
                    changed.push(message_facts.account_id);
                }
            }
            Ok(changed)
        })
    }
}

pub(crate) fn enqueue(
    tx: &Transaction<'_>,
    message: &crate::MessageRecord,
    state: &FolderSyncState,
) -> Result<(), StorageError> {
    let eligible: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM rule_baselines WHERE account_id=?1 AND folder=?2 AND uidvalidity=?3 AND frontier<?4)",params![state.account_id,state.folder_name,state.uidvalidity,message.uid],|r|r.get(0)).map_err(error)?;
    if eligible {
        // INVARIANT: Move-without-COPYUID can delete the source cache; identity tombstones
        // outlive messages to prevent a destination resync from running the rule again.
        let identity = if let Some(id) = &message.provider_message_id {
            format!("provider:{id}")
        } else if let Some(id) = &message.message_id_header {
            format!("header:{id}:{}", message.received_at_epoch_ms)
        } else {
            format!(
                "fallback:{}:{}:{}",
                message.raw_headers,
                message.subject,
                message.text_body.as_deref().unwrap_or_default()
            )
        };
        let key = format!("{:x}", Sha256::digest(identity.as_bytes()));
        if tx
            .execute(
                "INSERT OR IGNORE INTO rule_events VALUES (?1,?2)",
                params![message.account_id, key],
            )
            .map_err(error)?
            == 0
        {
            return Ok(());
        }
        let rules = read_rules(tx)?;
        // INVARIANT: Freeze rule revisions at ingestion, so later edits cannot act on old mail.
        tx.execute(
            "INSERT OR IGNORE INTO rule_jobs(message_id,rules_json,alert_revisions,event_key,schedule_revisions) VALUES (?1,?2,?3,?4,?5)",
            params![message.id, serde_json::to_string(&rules).map_err(error)?, crate::alerts::revisions(tx)?, key, crate::alerts::schedule::revisions(tx)?],
        )
        .map_err(error)?;
    }
    Ok(())
}

fn apply_plan(
    tx: &Transaction<'_>,
    id: &str,
    plan: &RulePlan,
    remote: &impl Fn(&str, &str, &str, u32, u32, &RuleAction) -> Result<String, StorageError>,
) -> Result<(), StorageError> {
    let (account,folder,uid,validity): (String,String,u32,u32) = tx.query_row("SELECT m.account_id,m.folder,m.uid,s.uidvalidity FROM messages m JOIN folder_sync_state s ON s.account_id=m.account_id AND s.folder_name=m.folder WHERE m.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(error)?;
    for action in &plan.actions {
        if let RuleAction::Alert(destination) | RuleAction::UrgentAlert(destination) = action {
            let urgent = matches!(action, RuleAction::UrgentAlert(_));
            // INVARIANT: Evaluate the complete ordered plan before destination deduplication.
            if !urgent
                && plan
                    .actions
                    .contains(&RuleAction::UrgentAlert(destination.clone()))
            {
                continue;
            }
            crate::alerts::enqueue_auto(tx, id, destination, crate::now_epoch_ms(), urgent)?;
            continue;
        }
        let local = match action {
            RuleAction::Tag(t) => LocalMutation::Tag(t),
            RuleAction::Star => LocalMutation::Star(true),
            RuleAction::Archive => LocalMutation::Folder("Archive"),
            RuleAction::Alert(_) | RuleAction::UrgentAlert(_) => {
                unreachable!("alert handled above")
            }
        };
        SqliteMailStore::apply_mutation_in_transaction(tx, id, &local)?;
        if !matches!(action, RuleAction::Tag(_)) {
            let payload = remote(id, &account, &folder, uid, validity, action)?;
            tx.execute(
                "INSERT INTO pending_mutations(message_id,payload_json) VALUES (?1,?2)",
                params![id, payload],
            )
            .map_err(error)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatabaseKey, MailRepository, MessageRecord};
    fn store() -> SqliteMailStore {
        let s =
            SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([91; 32]), Default::default())
                .unwrap();
        s.conn.lock().unwrap().execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.org','synthetic',0,0)").unwrap();
        s
    }
    fn rule() -> Rule {
        Rule {
            version: 1,
            id: "r".into(),
            revision: 0,
            name: "Synthetic".into(),
            enabled: true,
            position: 0,
            all: vec![Condition::Account("a".into())],
            any: vec![],
            except: vec![],
            actions: vec![
                RuleAction::Star,
                RuleAction::Archive,
                RuleAction::Tag("bill".into()),
            ],
            stop_processing: false,
        }
    }
    fn message(id: &str, uid: u32) -> MessageRecord {
        MessageRecord {
            id: id.into(),
            account_id: "a".into(),
            folder: "INBOX".into(),
            uid,
            message_id_header: Some(format!("{id}@example.com")),
            in_reply_to: None,
            references_header: None,
            subject: "Bill".into(),
            sender: "Billing <billing@example.com>".into(),
            recipients: "reader@example.com".into(),
            received_at_epoch_ms: 100,
            is_read: false,
            has_attachments: false,
            category_badge: None,
            triage_badges: vec![],
            raw_headers: String::new(),
            text_body: Some("synthetic".into()),
            html_body: None,
            list_unsubscribe: None,
            list_unsubscribe_post: None,
            one_click_unsubscribe_supported: false,
            created_at_epoch_ms: 0,
            attachments: vec![],
            tags: vec![],
            is_starred: false,
            snooze_until_epoch_ms: Some(i64::MAX),
            provider_message_id: None,
        }
    }
    fn state(uidnext: u32) -> FolderSyncState {
        FolderSyncState {
            account_id: "a".into(),
            folder_name: "INBOX".into(),
            uidvalidity: 7,
            uidnext,
            highest_modseq: None,
        }
    }
    fn count(s: &SqliteMailStore, table: &str) -> i64 {
        s.conn
            .lock()
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn payload(
        _: &str,
        _: &str,
        _: &str,
        _: u32,
        _: u32,
        action: &RuleAction,
    ) -> Result<String, StorageError> {
        Ok(serde_json::to_string(action).unwrap())
    }

    fn alert_destination() -> triage_ai::alerts::Destination {
        triage_ai::alerts::Destination {
            id: "d".into(),
            revision: 0,
            name: "Synthetic".into(),
            kind: triage_ai::alerts::DestinationKind::Webhook,
            enabled: true,
            fields: vec![triage_ai::alerts::ExportField::Subject],
            public_origin: Some("https://example.com".into()),
            has_credentials: false,
        }
    }
    #[test]
    fn urgent_rule_precedence_and_policy_snapshots_are_consent_bound() {
        let s = store();
        s.save_alert_destination(alert_destination(), None, 0)
            .unwrap();
        let mut p = s.alert_schedule("d").unwrap().policy;
        p.urgent_bypass = true;
        p.cadence = triage_ai::alerts::schedule::Cadence::Daily { minute: 540 };
        s.save_alert_schedule(p, true, 0).unwrap();
        let mut r = rule();
        r.actions = vec![
            RuleAction::Alert("d".into()),
            RuleAction::UrgentAlert("d".into()),
        ];
        s.save_rule(r).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
        s.commit_sync_batch(&[message("one", 1)], &[], &state(2), false)
            .unwrap();
        s.process_rule_jobs(100, payload).unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        assert_eq!(count(&s, "alert_schedule_items"), 0);
        let urgent: bool = s
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT urgent FROM alert_jobs", [], |r| r.get(0))
            .unwrap();
        assert!(urgent);
        s.commit_sync_batch(&[message("two", 2)], &[], &state(3), false)
            .unwrap();
        let p = s.alert_schedule("d").unwrap().policy;
        s.save_alert_schedule(p, true, 0).unwrap();
        s.process_rule_jobs(100, payload).unwrap();
        assert_eq!(s.alert_history().unwrap().len(), 1);
        assert_eq!(s.alert_history().unwrap()[0].status, "Canceled");
    }
    #[test]
    fn alerts_share_atomic_rule_completion_and_never_replay_history() {
        let s = store();
        s.save_alert_destination(alert_destination(), None, 0)
            .unwrap();
        let mut r = rule();
        r.actions.push(RuleAction::Alert("d".into()));
        s.save_rule(r).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 2).unwrap();
        s.commit_sync_batch(
            &[message("old", 1), message("new", 2)],
            &[],
            &state(3),
            false,
        )
        .unwrap();
        let failure = |_: &str, _: &str, _: &str, _: u32, _: u32, _: &RuleAction| {
            Err(StorageError::Query("synthetic failure".into()))
        };
        assert!(s.process_rule_jobs(100, failure).is_err());
        assert_eq!(count(&s, "alert_jobs"), 0);
        assert_eq!(count(&s, "pending_mutations"), 0);
        s.process_rule_jobs(100, payload).unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        s.process_rule_jobs(100, payload).unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        let job = s.claim_alert(crate::now_epoch_ms()).unwrap().unwrap();
        assert!(job.payload.contains("Bill"));
        assert!(!job.payload.contains("body"));
        assert!(s.simulate_rules(&["old".into()], None).unwrap()[0]
            .1
            .actions
            .contains(&RuleAction::Alert("d".into())));
        assert_eq!(count(&s, "alert_jobs"), 1);
        s.replay_alerts(&["old".into()], "d", "explicit", crate::now_epoch_ms())
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 2);
        assert_eq!(count(&s, "pending_mutations"), 2);
    }
    #[test]
    fn edited_destinations_invalidate_ingestion_snapshots_and_account_removal_cleans_jobs() {
        let s = store();
        let d = s
            .save_alert_destination(alert_destination(), None, 0)
            .unwrap();
        let mut r = rule();
        r.actions = vec![RuleAction::Alert("d".into())];
        s.save_rule(r).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
        s.commit_sync_batch(&[message("one", 1)], &[], &state(2), false)
            .unwrap();
        s.save_alert_destination(d, None, 0).unwrap();
        s.process_rule_jobs(100, payload).unwrap();
        assert_eq!(count(&s, "alert_jobs"), 0);
        s.commit_sync_batch(&[message("two", 2)], &[], &state(3), false)
            .unwrap();
        s.process_rule_jobs(100, payload).unwrap();
        assert_eq!(count(&s, "alert_jobs"), 1);
        s.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM accounts WHERE id='a'", [])
            .unwrap();
        assert_eq!(count(&s, "alert_jobs"), 0);
        assert_eq!(count(&s, "alert_dedup"), 0);
    }

    #[test]
    fn encrypted_restart_resumes_pending_jobs_without_reapplying_processed_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.db");
        let key = DatabaseKey::from_bytes([92; 32]);
        {
            let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
            s.conn
                .lock()
                .unwrap()
                .execute_batch(
                    "INSERT INTO accounts VALUES ('a','reader@example.com','synthetic',0,0)",
                )
                .unwrap();
            s.save_rule(rule()).unwrap();
            s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
            s.commit_sync_batch(&[message("new", 1)], &[], &state(2), false)
                .unwrap();
        }
        {
            let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
            assert_eq!(s.process_rule_jobs(100, payload).unwrap(), ["a"]);
            assert_eq!(count(&s, "pending_mutations"), 2);
        }
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        assert!(s.process_rule_jobs(100, payload).unwrap().is_empty());
        assert_eq!(count(&s, "pending_mutations"), 2);
        assert!(SqliteMailStore::open(
            &path,
            &DatabaseKey::from_bytes([93; 32]),
            Default::default()
        )
        .is_err());
    }

    #[test]
    fn history_baseline_survives_batches_and_only_new_canonical_arrivals_enqueue() {
        let s = store();
        s.save_rule(rule()).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 601).unwrap();
        s.commit_sync_batch(&[message("one", 1)], &[], &state(2), false)
            .unwrap();
        // A resumed SELECT with a higher frontier must not reset the initial frontier.
        s.establish_rule_baseline("a", "INBOX", 7, 700).unwrap();
        s.commit_sync_batch(&[message("old", 600)], &[], &state(601), false)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 0);
        s.commit_sync_batch(&[message("new", 601)], &[], &state(602), false)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 1);
        let mut duplicate = message("another-folder", 900);
        duplicate.folder = "Other".into();
        duplicate.message_id_header = message("new", 601).message_id_header;
        let other = FolderSyncState {
            folder_name: "Other".into(),
            ..state(901)
        };
        s.establish_rule_baseline("a", "Other", 7, 1).unwrap();
        s.commit_sync_batch(&[duplicate], &[], &other, false)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 1);
        s.commit_sync_batch(&[message("new", 601)], &[], &state(602), false)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 1);
    }

    #[test]
    fn simulation_is_pure_and_execution_marker_changes_outbox_are_atomic() {
        let s = store();
        s.save_rule(rule()).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
        s.commit_sync_batch(&[message("new", 1)], &[], &state(2), false)
            .unwrap();
        let before = s.get_message_by_id("new").unwrap().unwrap();
        let result = s.simulate_rules(&["new".into()], None).unwrap();
        assert_eq!(result[0].1.actions.len(), 3);
        assert_eq!(s.get_message_by_id("new").unwrap().unwrap(), before);
        assert_eq!(count(&s, "pending_mutations"), 0);
        assert!(s
            .process_rule_jobs(100, |_, _, _, _, _, _| Err(invalid()))
            .is_err());
        assert_eq!(s.get_message_by_id("new").unwrap().unwrap(), before);
        assert_eq!(count(&s, "pending_mutations"), 0);
        assert_eq!(s.process_rule_jobs(100, payload).unwrap(), ["a"]);
        let after = s.get_message_by_id("new").unwrap().unwrap();
        assert!(after.is_starred);
        assert_eq!(after.tags, ["bill"]);
        assert_eq!(after.snooze_until_epoch_ms, None);
        assert_eq!(
            s.folder_override("new").unwrap().as_deref(),
            Some("Archive")
        );
        assert_eq!(after.folder, "INBOX");
        assert_eq!(after.uid, 1);
        assert_eq!(count(&s, "pending_mutations"), 2);
        assert!(s.process_rule_jobs(100, payload).unwrap().is_empty());
        assert_eq!(count(&s, "pending_mutations"), 2);
    }

    #[test]
    fn edits_disable_pending_revisions_and_never_replay_existing_mail() {
        let s = store();
        let mut r = s.save_rule(rule()).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
        s.commit_sync_batch(&[message("new", 1)], &[], &state(2), false)
            .unwrap();
        let stale = r.clone();
        r.enabled = false;
        let r = s.save_rule(r).unwrap();
        assert!(s.save_rule(stale).is_err());
        assert!(s.delete_rule("r", 1).is_err());
        assert!(s.process_rule_jobs(100, payload).unwrap().is_empty());
        let mut r = r;
        r.enabled = true;
        s.save_rule(r).unwrap();
        assert!(s.process_rule_jobs(100, payload).unwrap().is_empty());
        let mut bad = rule();
        bad.id = "bad".into();
        bad.all = vec![Condition::Account("missing".into())];
        assert!(s.save_rule(bad).is_err());
        let current = s.list_rules().unwrap().remove(0);
        s.delete_rule(&current.id, current.revision).unwrap();
        assert!(s.save_rule(rule()).is_err());
    }

    #[test]
    fn reset_and_unknown_frontiers_fail_closed_and_tombstones_survive_cache_deletion() {
        let s = store();
        s.save_rule(rule()).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
        let msg = message("new", 1);
        s.commit_sync_batch(std::slice::from_ref(&msg), &[], &state(2), false)
            .unwrap();
        s.process_rule_jobs(100, payload).unwrap();
        s.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM messages WHERE id='new'", [])
            .unwrap();
        let mut moved = msg;
        moved.id = "moved".into();
        moved.uid = 500;
        s.commit_sync_batch(&[moved], &[], &state(501), false)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 0);
        s.establish_rule_baseline("a", "INBOX", 8, 900).unwrap();
        let reset = FolderSyncState {
            uidvalidity: 8,
            ..state(1)
        };
        s.commit_sync_batch(&[], &[], &reset, true).unwrap();
        let reset = FolderSyncState {
            uidnext: 701,
            ..reset
        };
        s.commit_sync_batch(&[message("history", 700)], &[], &reset, false)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 0);
        s.establish_rule_baseline("a", "INBOX", 9, 0).unwrap();
        let unknown = FolderSyncState {
            uidvalidity: 9,
            ..state(1001)
        };
        s.commit_sync_batch(&[message("unknown", 1000)], &[], &unknown, true)
            .unwrap();
        assert_eq!(count(&s, "rule_jobs"), 0);
    }

    #[test]
    fn ingestion_rolls_back_jobs_and_account_bound_simulation_and_reorder_are_validated() {
        let s = store();
        let r = s.save_rule(rule()).unwrap();
        s.establish_rule_baseline("a", "INBOX", 7, 1).unwrap();
        let mut bad = message("bad", 2);
        bad.attachments.push(crate::AttachmentRecord {
            id: "bad-attachment".into(),
            message_id: "missing".into(),
            part_id: "2".into(),
            filename: "synthetic.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: 1,
            content_id: None,
            is_inline: false,
        });
        assert!(s
            .commit_sync_batch(&[message("one", 1), bad], &[], &state(3), false)
            .is_err());
        assert_eq!(count(&s, "rule_jobs"), 0);
        assert_eq!(count(&s, "messages"), 0);
        assert!(s.get_folder_sync_state("a", "INBOX").unwrap().is_none());
        assert!(s.simulate_rules(&[], None).is_err());
        assert!(s.simulate_rules(&["missing".into()], None).is_err());
        assert!(s.reorder_rules(&[]).is_err());
        s.reorder_rules(&[(r.id.clone(), r.revision)]).unwrap();
        assert!(s.reorder_rules(&[(r.id, r.revision)]).is_err());
        let mut b = message("b", 1);
        b.account_id = "b".into();
        s.commit_sync_batch(
            &[b],
            &[],
            &FolderSyncState {
                account_id: "b".into(),
                ..state(2)
            },
            false,
        )
        .unwrap();
        assert!(s.simulate_rules(&["b".into()], None).unwrap()[0]
            .1
            .actions
            .is_empty());
    }
}
