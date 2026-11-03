//! Encrypted persistence and queries for due-date items and scheduled reminders.
use crate::{SqliteMailStore, StorageError};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use triage_ai::{
    due_date_trigger_epoch_ms, rules::valid_date, unread_interval_trigger_epoch_ms, DueDateItem,
    ReminderTrigger, ScheduledReminderConfig, TriageBadge, MAX_REMINDER_CONFIGS,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderJobRecord {
    pub id: String,
    pub config_id: Option<String>,
    pub message_id: String,
    pub account_id: String,
    pub trigger_type: String,
    pub due_epoch_ms: i64,
    pub destination_ids: Vec<String>,
    pub status: String,
    pub dispatched_at_epoch_ms: Option<i64>,
    pub created_at_epoch_ms: i64,
}

fn error(e: impl std::fmt::Display) -> StorageError {
    StorageError::Query(format!("reminder operation failed: {e}"))
}

fn invalid() -> StorageError {
    StorageError::InvalidSearchQuery("invalid or stale reminder configuration".into())
}

impl SqliteMailStore {
    /// Lists all persisted scheduled reminder configurations sorted by name.
    pub fn list_reminder_configs(&self) -> Result<Vec<ScheduledReminderConfig>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT definition_json FROM reminder_configs
                     ORDER BY json_extract(definition_json, '$.name') COLLATE NOCASE, id",
                )
                .map_err(error)?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(error)?;
            rows.iter()
                .map(|s| serde_json::from_str(s).map_err(error))
                .collect()
        })
    }

    /// Saves or updates a scheduled reminder configuration with optimistic revision checking.
    pub fn save_reminder_config(
        &self,
        mut config: ScheduledReminderConfig,
    ) -> Result<ScheduledReminderConfig, StorageError> {
        if !config.validate() {
            return Err(invalid());
        }
        self.transaction(|tx| {
            if let Some(acct) = &config.account_id {
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1)",
                        [acct],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if !exists {
                    return Err(invalid());
                }
            }

            let current: Option<(u32, String)> = tx
                .query_row(
                    "SELECT revision, definition_json FROM reminder_configs WHERE id = ?1",
                    [&config.id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(error)?;

            if let Some((rev, _)) = current {
                if rev != config.revision {
                    return Err(invalid());
                }
                config.revision += 1;
                let json = serde_json::to_string(&config).map_err(error)?;
                tx.execute(
                    "UPDATE reminder_configs SET revision = ?1, definition_json = ?2 WHERE id = ?3",
                    params![config.revision, json, config.id],
                )
                .map_err(error)?;
            } else {
                if config.revision != 0 {
                    return Err(invalid());
                }
                let count: usize = tx
                    .query_row("SELECT COUNT(*) FROM reminder_configs", [], |r| r.get(0))
                    .map_err(error)?;
                if count >= MAX_REMINDER_CONFIGS {
                    return Err(invalid());
                }
                let was_deleted: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM reminder_config_deleted_ids WHERE id = ?1)",
                        [&config.id],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if was_deleted {
                    return Err(invalid());
                }
                config.revision = 1;
                let json = serde_json::to_string(&config).map_err(error)?;
                tx.execute(
                    "INSERT INTO reminder_configs (id, revision, definition_json) VALUES (?1, ?2, ?3)",
                    params![config.id, config.revision, json],
                )
                .map_err(error)?;
            }
            Ok(config)
        })
    }

    /// Deletes a reminder configuration and cancels its pending jobs.
    pub fn delete_reminder_config(&self, id: &str, revision: u32) -> Result<(), StorageError> {
        self.transaction(|tx| {
            let current_rev: Option<u32> = tx
                .query_row(
                    "SELECT revision FROM reminder_configs WHERE id = ?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(error)?;
            let Some(rev) = current_rev else {
                return Err(invalid());
            };
            if rev != revision {
                return Err(invalid());
            }
            tx.execute("DELETE FROM reminder_configs WHERE id = ?1", [id])
                .map_err(error)?;
            tx.execute(
                "INSERT OR IGNORE INTO reminder_config_deleted_ids (id) VALUES (?1)",
                [id],
            )
            .map_err(error)?;
            tx.execute(
                "UPDATE reminder_jobs SET status = 'Canceled' WHERE config_id = ?1 AND status = 'Pending'",
                [id],
            )
            .map_err(error)?;
            Ok(())
        })
    }

    /// Lists extracted due-date items across messages, optionally filtered by account.
    pub fn list_due_date_items(
        &self,
        account_id: Option<&str>,
    ) -> Result<Vec<DueDateItem>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT m.id, m.account_id, m.subject, m.sender, m.received_at_epoch_ms, m.is_read,
                            m.triage_badges,
                            (d.dismissed_at_epoch_ms IS NOT NULL) AS is_dismissed
                     FROM messages m
                     LEFT JOIN due_date_dismissals d ON d.message_id = m.id
                     WHERE (?1 IS NULL OR m.account_id = ?1)
                       AND m.triage_badges LIKE '%FebrabanBoleto%'
                     ORDER BY m.received_at_epoch_ms DESC",
                )
                .map_err(error)?;

            let rows = stmt
                .query_map([account_id], |row| {
                    let id: String = row.get(0)?;
                    let acct: String = row.get(1)?;
                    let subject: String = row.get(2)?;
                    let sender: String = row.get(3)?;
                    let received: i64 = row.get(4)?;
                    let is_read: bool = row.get(5)?;
                    let badges_json: String = row.get(6)?;
                    let is_dismissed: bool = row.get(7)?;
                    Ok((
                        id,
                        acct,
                        subject,
                        sender,
                        received,
                        is_read,
                        badges_json,
                        is_dismissed,
                    ))
                })
                .map_err(error)?;

            let mut items = Vec::new();
            for r in rows {
                let (id, acct, subject, sender, received, is_read, badges_json, is_dismissed) =
                    r.map_err(error)?;
                let Ok(badges) = serde_json::from_str::<Vec<TriageBadge>>(&badges_json) else {
                    continue;
                };
                for b in badges {
                    if let TriageBadge::FebrabanBoleto {
                        digitable_line,
                        amount_cents,
                        due_date_iso: Some(due_date),
                    } = b
                    {
                        if valid_date(&due_date) {
                            items.push(DueDateItem {
                                message_id: id.clone(),
                                account_id: acct.clone(),
                                subject: subject.clone(),
                                sender: sender.clone(),
                                received_at_epoch_ms: received,
                                is_read,
                                digitable_line,
                                amount_cents,
                                due_date_iso: due_date,
                                is_dismissed,
                            });
                        }
                    }
                }
            }
            // INVARIANT: Primary sort by due date ascending, tie-breaker by received date descending
            items.sort_by(|a, b| {
                a.due_date_iso
                    .cmp(&b.due_date_iso)
                    .then_with(|| b.received_at_epoch_ms.cmp(&a.received_at_epoch_ms))
            });
            Ok(items)
        })
    }

    /// Explicitly dismisses due-date reminders for a message.
    pub fn dismiss_due_date_item(
        &self,
        message_id: &str,
        now_epoch_ms: i64,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute(
                "INSERT INTO due_date_dismissals (message_id, dismissed_at_epoch_ms)
                 VALUES (?1, ?2)
                 ON CONFLICT(message_id) DO UPDATE SET dismissed_at_epoch_ms = ?2",
                params![message_id, now_epoch_ms],
            )
            .map_err(error)?;
            tx.execute(
                "UPDATE reminder_jobs
                 SET status = 'Dismissed'
                 WHERE message_id = ?1 AND status = 'Pending'",
                params![message_id],
            )
            .map_err(error)?;
            Ok(())
        })
    }

    /// Cancels unread reminder jobs for messages that have been read.
    pub fn cancel_reminders_for_read_messages(
        &self,
        message_ids: &[String],
    ) -> Result<usize, StorageError> {
        if message_ids.is_empty() {
            return Ok(0);
        }
        self.transaction(|tx| {
            let mut count = 0;
            let mut stmt = tx
                .prepare(
                    "UPDATE reminder_jobs
                     SET status = 'Canceled'
                     WHERE message_id = ?1
                       AND trigger_type = 'UnreadInterval'
                       AND status = 'Pending'",
                )
                .map_err(error)?;
            for id in message_ids {
                count += stmt.execute([id]).map_err(error)?;
            }
            Ok(count)
        })
    }

    /// Evaluates active reminder configs against a message and enqueues reminder jobs.
    pub fn schedule_reminders_for_message(
        &self,
        message_id: &str,
        now_epoch_ms: i64,
        tz_offset_seconds: i32,
    ) -> Result<usize, StorageError> {
        self.transaction(|tx| {
            let msg: Option<(String, i64, bool, String)> = tx
                .query_row(
                    "SELECT account_id, received_at_epoch_ms, is_read, triage_badges
                     FROM messages WHERE id = ?1",
                    [message_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()
                .map_err(error)?;

            let Some((account_id, received_at_epoch_ms, is_read, badges_json)) = msg else {
                return Ok(0);
            };

            let is_dismissed: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM due_date_dismissals WHERE message_id = ?1)",
                    [message_id],
                    |r| r.get(0),
                )
                .map_err(error)?;
            if is_dismissed {
                return Ok(0);
            }

            let configs = {
                let mut stmt = tx
                    .prepare("SELECT definition_json FROM reminder_configs")
                    .map_err(error)?;
                let rows = stmt
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(error)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(error)?;
                rows.into_iter()
                    .filter_map(|s| serde_json::from_str::<ScheduledReminderConfig>(&s).ok())
                    .filter(|c| c.enabled)
                    .collect::<Vec<_>>()
            };

            let badges: Vec<TriageBadge> = serde_json::from_str(&badges_json).unwrap_or_default();
            let mut inserted = 0;

            for config in configs {
                if let Some(acct) = &config.account_id {
                    if acct != &account_id {
                        continue;
                    }
                }
                match config.trigger {
                    ReminderTrigger::DueDate {
                        days_before,
                        target_hour,
                        min_amount_cents,
                    } => {
                        for b in &badges {
                            if let TriageBadge::FebrabanBoleto {
                                amount_cents,
                                due_date_iso: Some(due_date),
                                ..
                            } = b
                            {
                                if !valid_date(due_date) {
                                    continue;
                                }
                                if let Some(min) = min_amount_cents {
                                    if amount_cents.is_none_or(|amt| amt < min) {
                                        continue;
                                    }
                                }
                                if let Some(due_epoch_ms) = due_date_trigger_epoch_ms(
                                    due_date,
                                    days_before,
                                    target_hour,
                                    tz_offset_seconds,
                                ) {
                                    let job_id =
                                        format!("rem-{}-{}-{}", message_id, config.id, due_epoch_ms);
                                    let dest_json = serde_json::to_string(&config.destination_ids)
                                        .unwrap_or_default();
                                    let count = tx
                                        .execute(
                                            "INSERT OR IGNORE INTO reminder_jobs
                                             (id, config_id, message_id, account_id, trigger_type, due_epoch_ms, destination_ids, status, created_at_epoch_ms)
                                             VALUES (?1, ?2, ?3, ?4, 'DueDate', ?5, ?6, 'Pending', ?7)",
                                            params![
                                                job_id,
                                                config.id,
                                                message_id,
                                                account_id,
                                                due_epoch_ms,
                                                dest_json,
                                                now_epoch_ms
                                            ],
                                        )
                                        .map_err(error)?;
                                    inserted += count;
                                }
                            }
                        }
                    }
                    ReminderTrigger::UnreadInterval {
                        interval_hours,
                        min_amount_cents,
                    } => {
                        if is_read {
                            continue;
                        }
                        if let Some(min) = min_amount_cents {
                            let meets_min = badges.iter().any(|b| match b {
                                TriageBadge::FebrabanBoleto {
                                    amount_cents: Some(amt),
                                    ..
                                } => *amt >= min,
                                _ => false,
                            });
                            if !meets_min {
                                continue;
                            }
                        }
                        if let Some(due_epoch_ms) = unread_interval_trigger_epoch_ms(
                            received_at_epoch_ms,
                            interval_hours,
                        ) {
                            let job_id = format!("rem-{}-{}-unread", message_id, config.id);
                            let dest_json = serde_json::to_string(&config.destination_ids)
                                .unwrap_or_default();
                            let count = tx
                                .execute(
                                    "INSERT OR IGNORE INTO reminder_jobs
                                     (id, config_id, message_id, account_id, trigger_type, due_epoch_ms, destination_ids, status, created_at_epoch_ms)
                                     VALUES (?1, ?2, ?3, ?4, 'UnreadInterval', ?5, ?6, 'Pending', ?7)",
                                    params![
                                        job_id,
                                        config.id,
                                        message_id,
                                        account_id,
                                        due_epoch_ms,
                                        dest_json,
                                        now_epoch_ms
                                    ],
                                )
                                .map_err(error)?;
                            inserted += count;
                        }
                    }
                }
            }
            Ok(inserted)
        })
    }

    /// Claims pending reminder jobs that are due, reconciling unread/dismissed cancellations.
    pub fn claim_due_reminders(
        &self,
        now_epoch_ms: i64,
        limit: usize,
    ) -> Result<Vec<ReminderJobRecord>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT j.id, j.config_id, j.message_id, j.account_id, j.trigger_type,
                            j.due_epoch_ms, j.destination_ids, j.status, j.dispatched_at_epoch_ms,
                            j.created_at_epoch_ms,
                            m.is_read,
                            (d.dismissed_at_epoch_ms IS NOT NULL) AS is_dismissed
                     FROM reminder_jobs j
                     JOIN messages m ON m.id = j.message_id
                     LEFT JOIN due_date_dismissals d ON d.message_id = j.message_id
                     WHERE j.status = 'Pending' AND j.due_epoch_ms <= ?1
                     ORDER BY j.due_epoch_ms ASC
                     LIMIT ?2",
                )
                .map_err(error)?;

            let rows = stmt
                .query_map(params![now_epoch_ms, limit], |r| {
                    Ok((
                        ReminderJobRecord {
                            id: r.get(0)?,
                            config_id: r.get(1)?,
                            message_id: r.get(2)?,
                            account_id: r.get(3)?,
                            trigger_type: r.get(4)?,
                            due_epoch_ms: r.get(5)?,
                            destination_ids: serde_json::from_str(&r.get::<_, String>(6)?)
                                .unwrap_or_default(),
                            status: r.get(7)?,
                            dispatched_at_epoch_ms: r.get(8)?,
                            created_at_epoch_ms: r.get(9)?,
                        },
                        r.get::<_, bool>(10)?,
                        r.get::<_, bool>(11)?,
                    ))
                })
                .map_err(error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(error)?;

            let mut valid_jobs = Vec::new();
            for (job, is_read, is_dismissed) in rows {
                if is_dismissed {
                    tx.execute(
                        "UPDATE reminder_jobs SET status = 'Dismissed' WHERE id = ?1",
                        [&job.id],
                    )
                    .map_err(error)?;
                    continue;
                }
                if job.trigger_type == "UnreadInterval" && is_read {
                    tx.execute(
                        "UPDATE reminder_jobs SET status = 'Canceled' WHERE id = ?1",
                        [&job.id],
                    )
                    .map_err(error)?;
                    continue;
                }
                valid_jobs.push(job);
            }
            Ok(valid_jobs)
        })
    }

    /// Marks a reminder job as dispatched.
    pub fn finish_reminder_job(
        &self,
        job_id: &str,
        dispatched_at_epoch_ms: i64,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute(
                "UPDATE reminder_jobs SET status = 'Dispatched', dispatched_at_epoch_ms = ?1 WHERE id = ?2",
                params![dispatched_at_epoch_ms, job_id],
            )
            .map_err(error)?;
            Ok(())
        })
    }

    /// Dispatches a claimed reminder job by enqueuing alert deliveries to its destinations and marking it dispatched.
    pub fn dispatch_reminder_job(
        &self,
        job: &ReminderJobRecord,
        now_epoch_ms: i64,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            let mut facts = crate::rules::facts(tx, &job.message_id)?;
            if now_epoch_ms.saturating_sub(job.due_epoch_ms) > 15 * 60 * 1000 {
                facts.subject = format!("[Delayed Reminder] {}", facts.subject);
            }
            let active_destinations = crate::alerts::destinations(tx)?;
            for dest_id in &job.destination_ids {
                if let Some(dest) = active_destinations.iter().find(|d| &d.id == dest_id && d.enabled) {
                    let event_id = format!("rem-{}-{}", job.id, dest.id);
                    let payload = triage_ai::alerts::payload(&event_id, now_epoch_ms, &dest.fields, &facts);
                    tx.execute(
                        "INSERT OR IGNORE INTO alert_jobs (id, destination_id, revision, account_id, message_id, payload, created, due)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                        params![event_id, dest.id, dest.revision, job.account_id, job.message_id, payload, now_epoch_ms],
                    ).map_err(error)?;
                }
            }
            tx.execute(
                "UPDATE reminder_jobs SET status = 'Dispatched', dispatched_at_epoch_ms = ?1 WHERE id = ?2",
                params![now_epoch_ms, job.id],
            ).map_err(error)?;
            Ok(())
        })
    }
}
