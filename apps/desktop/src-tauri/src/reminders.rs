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
}
