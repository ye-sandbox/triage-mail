//! Reminders and Due-Date Dashboard orchestration.
use crate::{DesktopService, DueDateItemDto, IpcError, ScheduledReminderConfigDto};
use std::sync::Arc;
use storage::StorageError;

fn error(e: StorageError) -> IpcError {
    match e {
        StorageError::InvalidSearchQuery(_) => IpcError::InvalidRequest,
        _ => IpcError::StorageUnavailable,
    }
}

pub(crate) fn local_tz_offset_seconds() -> i32 {
    chrono::Local::now().offset().local_minus_utc()
}

impl DesktopService {
    pub async fn list_due_date_items(
        self: &Arc<Self>,
        account_id: Option<String>,
    ) -> Result<Vec<DueDateItemDto>, IpcError> {
        self.blocking(move |s| {
            s.store
                .list_due_date_items(account_id.as_deref())
                .map_err(error)
        })
        .await
    }

    pub async fn dismiss_due_date_item(
        self: &Arc<Self>,
        message_id: String,
    ) -> Result<(), IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| {
            s.store
                .dismiss_due_date_item(&message_id, storage::now_epoch_ms())
                .map_err(error)
        })
        .await
    }

    pub async fn list_reminder_configs(
        self: &Arc<Self>,
    ) -> Result<Vec<ScheduledReminderConfigDto>, IpcError> {
        self.blocking(|s| s.store.list_reminder_configs().map_err(error))
            .await
    }

    pub async fn save_reminder_config(
        self: &Arc<Self>,
        config: ScheduledReminderConfigDto,
    ) -> Result<ScheduledReminderConfigDto, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.save_reminder_config(config).map_err(error))
            .await
    }

    pub async fn delete_reminder_config(
        self: &Arc<Self>,
        id: String,
        revision: u32,
    ) -> Result<(), IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.delete_reminder_config(&id, revision).map_err(error))
            .await
    }

    pub async fn process_reminders(self: &Arc<Self>) -> Result<(), IpcError> {
        let now = storage::now_epoch_ms();
        let jobs = self
            .blocking(move |s| s.store.claim_due_reminders(now, 10).map_err(error))
            .await?;
        for job in jobs {
            self.blocking(move |s| s.store.dispatch_reminder_job(&job, now).map_err(error))
                .await?;
        }
        Ok(())
    }

    pub async fn due_date_summary_counts(
        self: &Arc<Self>,
    ) -> Result<DueDateSummaryCounts, IpcError> {
        let today_iso = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.blocking(move |s| s.due_date_summary_counts_for_date(&today_iso))
            .await
    }

    pub(crate) fn due_date_summary_counts_for_date(
        &self,
        today_iso: &str,
    ) -> Result<DueDateSummaryCounts, IpcError> {
        let items = self.store.list_due_date_items(None).map_err(error)?;
        let mut counts = DueDateSummaryCounts::default();
        for item in items {
            match item.status(today_iso) {
                triage_ai::reminders::DueDateStatus::Overdue => counts.overdue += 1,
                triage_ai::reminders::DueDateStatus::DueToday => counts.due_today += 1,
                triage_ai::reminders::DueDateStatus::Upcoming
                | triage_ai::reminders::DueDateStatus::Dismissed => {}
            }
        }
        Ok(counts)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DueDateSummaryCounts {
    pub overdue: usize,
    pub due_today: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_overdue_and_due_today_non_dismissed_boletos() {
        let store = Arc::new(
            storage::SqliteMailStore::open_in_memory(
                &storage::DatabaseKey::from_bytes([71; 32]),
                Default::default(),
            )
            .unwrap(),
        );
        let vault: Arc<dyn vault::SecretStore> =
            Arc::new(vault::EncryptedFallbackStore::in_memory(Arc::new(
                vault::Argon2AesGcmCipher::from_key([72; 32]),
            )));
        let service = DesktopService::new(
            store.clone(),
            vault.clone(),
            Arc::new(crate::remote::NativeRemoteMail::new(vault)),
            None,
        );
        store
            .insert_account(&storage::AccountRecord {
                id: "acct".into(),
                email: "reader@example.com".into(),
                auth_type: "AppPassword".into(),
                created_at_epoch_ms: 0,
                updated_at_epoch_ms: 0,
            })
            .unwrap();
        for (id, uid, due) in [
            ("m-overdue", 1, "2026-11-10"),
            ("m-today", 2, "2026-11-18"),
            ("m-future", 3, "2026-11-25"),
            ("m-dismissed", 4, "2026-11-11"),
        ] {
            let mut record: storage::MessageRecord = serde_json::from_value(serde_json::json!({
                "id": id,
                "account_id": "acct",
                "folder": "INBOX",
                "uid": uid,
                "subject": "Boleto",
                "sender": "billing@example.com",
                "recipients": "reader@example.com",
                "received_at_epoch_ms": uid,
                "is_read": false,
                "has_attachments": false,
                "raw_headers": "",
                "text_body": "",
                "html_body": "",
                "one_click_unsubscribe_supported": false,
                "created_at_epoch_ms": 0,
                "attachments": [],
                "tags": []
            }))
            .unwrap();
            record.triage_badges = vec![triage_ai::TriageBadge::FebrabanBoleto {
                digitable_line: "123".into(),
                amount_cents: Some(5000),
                due_date_iso: Some(due.into()),
            }];
            store.insert_message(&record).unwrap();
        }
        store.dismiss_due_date_item("m-dismissed", 10).unwrap();

        let counts = service
            .due_date_summary_counts_for_date("2026-11-18")
            .unwrap();
        assert_eq!(
            counts,
            DueDateSummaryCounts {
                overdue: 1,
                due_today: 1,
            }
        );
    }
}
