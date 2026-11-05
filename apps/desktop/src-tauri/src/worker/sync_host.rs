//! Desktop execution policy and post-commit hooks for the shared coordinator.
use async_trait::async_trait;
use mail_engine::{sync::IncrementalSyncResult, DiscoveredFolder};
use sync_runtime::SyncHost;

use std::sync::Arc;

use storage::{FolderRecord, FolderSyncState, MessageRecord};
use triage_ai::{DeterministicTriageResult, TriageCategory, TriageEventHook};

use super::{checkpoint, index_injected_provider, message_record, status};
use crate::events::BatchEvents;
use crate::service::{now_ms, storage_error};
use crate::{AccountConfigDto, DesktopService, IpcError, SyncState};

pub(super) struct DesktopSyncHost<'a> {
    pub service: &'a Arc<DesktopService>,
    pub config: &'a AccountConfigDto,
}

#[async_trait]
impl SyncHost for DesktopSyncHost<'_> {
    type Error = IpcError;
    type Batch = Arc<BatchEvents>;

    fn account_id(&self) -> &str {
        &self.config.account_id
    }

    async fn discover_folders(&self) -> Result<Vec<DiscoveredFolder>, IpcError> {
        self.service.remote.discover_folders(self.config).await
    }

    async fn save_catalog(&self, folders: &[DiscoveredFolder]) -> Result<(), IpcError> {
        let account = self.config.account_id.clone();
        let records = folders
            .iter()
            .map(|folder| FolderRecord {
                account_id: account.clone(),
                folder_name: folder.name.clone(),
                display_name: folder.display_name.clone(),
                delimiter: folder.delimiter.clone(),
                role: folder
                    .role
                    .map(|role| role.as_role_str())
                    .unwrap_or("other")
                    .into(),
                is_selectable: folder.is_selectable,
                total_messages: None,
                unread_messages: None,
                updated_at_epoch_ms: now_ms(),
            })
            .collect::<Vec<_>>();
        self.service
            .blocking(move |s| {
                s.store
                    .save_folders(&account, &records)
                    .map_err(storage_error)
            })
            .await
    }

    async fn cached_folders(&self) -> Result<Vec<String>, IpcError> {
        let account = self.config.account_id.clone();
        self.service
            .blocking(move |s| s.store.list_synced_folders(&account).map_err(storage_error))
            .await
    }

    async fn checkpoint(&self, folder: &str) -> Result<FolderSyncState, IpcError> {
        checkpoint(self.service, &self.config.account_id, folder).await
    }

    fn begin_batch(&self) -> Self::Batch {
        Arc::new(BatchEvents::new(
            self.config.account_id.clone(),
            self.service.events.clone(),
        ))
    }

    async fn fetch_batch(
        &self,
        previous: &FolderSyncState,
        batch: &Self::Batch,
    ) -> Result<IncrementalSyncResult, IpcError> {
        self.service
            .remote
            .sync(self.config, previous, batch.as_ref())
            .await
    }

    async fn commit_batch(
        &self,
        state: FolderSyncState,
        result: IncrementalSyncResult,
        batch: Self::Batch,
    ) -> Result<(), IpcError> {
        // INVARIANT: Hold the existing mutation gate across blocking ingestion and
        // every post-commit hook, before the coordinator advances to another batch.
        let _guard = self.service.mutation_gate.lock().await;
        self.service
            .blocking(move |s| persist_batch(&s, state, result, batch))
            .await
    }
}

fn persist_batch(
    s: &DesktopService,
    state: FolderSyncState,
    result: IncrementalSyncResult,
    events: Arc<BatchEvents>,
) -> Result<(), IpcError> {
    s.store
        .establish_rule_baseline(
            &state.account_id,
            &state.folder_name,
            state.uidvalidity,
            result.server_uidnext,
        )
        .map_err(storage_error)?;
    s.store
        .establish_review_frontier(
            &state.account_id,
            &state.folder_name,
            state.uidvalidity,
            result.server_uidnext,
        )
        .map_err(storage_error)?;
    let records = result
        .new_messages
        .into_iter()
        .map(|m| message_record(m, &state))
        .collect::<Result<Vec<_>, _>>()?;
    let flags = message_flags(result.flag_deltas);
    let records = s
        .store
        .commit_sync_batch(&records, &flags, &state, result.uidvalidity_reset)
        .map_err(storage_error)?;
    finish_batch(s, &state, &records, &events, result.uidvalidity_reset)
}

fn finish_batch(
    s: &DesktopService,
    state: &FolderSyncState,
    records: &[MessageRecord],
    events: &BatchEvents,
    reset: bool,
) -> Result<(), IpcError> {
    triage_hooks(s, state, records, events)?;
    let tz_offset = crate::reminders::local_tz_offset_seconds();
    for record in records {
        let _ =
            s.store
                .schedule_reminders_for_message(&record.id, storage::now_epoch_ms(), tz_offset);
    }
    index_injected_provider(s, records, events)?;
    // INVARIANT: Progress is published only after messages and checkpoints commit together.
    events.committed_with_state(if reset {
        SyncState::ResyncRequired
    } else {
        SyncState::Idle
    });
    Ok(())
}

fn triage_hooks(
    s: &DesktopService,
    state: &FolderSyncState,
    records: &[MessageRecord],
    events: &BatchEvents,
) -> Result<(), IpcError> {
    for (index, record) in records.iter().enumerate() {
        let category = record
            .category_badge
            .as_deref()
            .and_then(|c| match c {
                "Security" => Some(TriageCategory::Security),
                "Financial" => Some(TriageCategory::Financial),
                "Shopping" => Some(TriageCategory::Shopping),
                "Newsletter" => Some(TriageCategory::Newsletter),
                "Personal" => Some(TriageCategory::Personal),
                "Notifications" => Some(TriageCategory::Notifications),
                _ => None,
            })
            .unwrap_or(TriageCategory::Personal);
        let triage = DeterministicTriageResult {
            category,
            badges: record.triage_badges.clone(),
        };
        events
            .on_message_triaged(&record.id, &triage)
            .map_err(|_| IpcError::Internal)?;
        // INVARIANT: One post-triage queue drain per committed batch;
        // failures remain durable and retry independently of ingestion.
        if index + 1 == records.len()
            && crate::rules::RuleObserver(s)
                .on_message_triaged(&record.id, &triage)
                .is_err()
        {
            status(s, &state.account_id, SyncState::MutationFailed);
        }
    }
    Ok(())
}

fn message_flags(deltas: Vec<mail_engine::protocol::UidFlags>) -> Vec<storage::MessageFlagsDelta> {
    deltas
        .into_iter()
        .map(|delta| storage::MessageFlagsDelta {
            uid: delta.uid,
            is_read: delta
                .flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("\\Seen")),
            is_starred: delta
                .flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("\\Flagged")),
        })
        .collect()
}
