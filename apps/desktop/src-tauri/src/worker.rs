mod sync_host;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mail_engine::{AdaptivePollConfig, AdaptivePollSchedule};
use storage::{FolderSyncState, MailRepository, MessageRecord};
use tokio::sync::watch;

use crate::events::BatchEvents;
use crate::remote::{MutationOutcome, RemoteMutation};
use crate::service::storage_error;
use crate::{AccountConfigDto, DesktopService, IpcError, SyncProgressEvent, SyncState};

/// Owns cancellation for desktop workers, independently of frontend channel subscriptions.
pub struct WorkerSupervisor {
    service: Arc<DesktopService>,
    shutdown: watch::Sender<bool>,
    accounts: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    outbox: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl WorkerSupervisor {
    pub fn new(service: Arc<DesktopService>) -> Arc<Self> {
        Arc::new(Self {
            service,
            shutdown: watch::channel(false).0,
            accounts: Mutex::new(HashMap::new()),
            outbox: Mutex::new(None),
        })
    }

    pub async fn start(self: &Arc<Self>) -> Result<(), IpcError> {
        for config in self.service.list_accounts().await? {
            self.start_account(config)?;
        }
        let mut outbox = self.outbox.lock().map_err(|_| IpcError::Internal)?;
        if outbox.is_none() {
            let service = self.service.clone();
            let mut stop = self.shutdown.subscribe();
            *outbox = Some(tokio::spawn(async move {
                let work = async {
                    tokio::join!(
                        outbox_loop(service.clone()),
                        category_loop(service.clone()),
                        embedding_loop(service)
                    );
                };
                tokio::select! { _ = work => {}, _ = stop.changed() => {} }
            }));
        }
        Ok(())
    }

    pub fn start_account(&self, config: AccountConfigDto) -> Result<(), IpcError> {
        // INVARIANT: Recovered cached-only and local-archive accounts have no remote endpoint.
        if config.imap_host.is_empty()
            || config.auth_method == mail_engine::AuthMethod::LocalArchive
        {
            return Ok(());
        }
        let mut accounts = self.accounts.lock().map_err(|_| IpcError::Internal)?;
        if accounts
            .get(&config.account_id)
            .is_some_and(|h| !h.is_finished())
        {
            return Ok(());
        }
        let service = self.service.clone();
        let mut stop = self.shutdown.subscribe();
        accounts.insert(
            config.account_id.clone(),
            tokio::spawn(async move {
                tokio::select! { _ = account_loop(service,config) => {}, _ = stop.changed() => {} }
            }),
        );
        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown.send_replace(true);
    }
}

impl Drop for WorkerSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn status(service: &DesktopService, account: &str, state: SyncState) {
    service.events.publish(SyncProgressEvent {
        account_id: account.into(),
        state,
        new_messages_count: 0,
        triage_alerts: Vec::new(),
        embedding_errors_count: 0,
    });
}

async fn account_loop(service: Arc<DesktopService>, config: AccountConfigDto) {
    let mut schedule = AdaptivePollSchedule::new(AdaptivePollConfig::default());
    loop {
        status(&service, &config.account_id, SyncState::Syncing);
        match sync_account_once(&service, &config).await {
            Ok(count) => {
                if count > 0 {
                    schedule.record_activity();
                } else {
                    schedule.record_empty_poll();
                }
                let checkpoint = checkpoint(&service, &config.account_id, "INBOX").await;
                let wait = async { service.remote.wait_for_change(&config, &checkpoint?).await };
                let deadline = Duration::from_secs(mail_engine::DEFAULT_IDLE_HEARTBEAT_SECS + 30);
                match tokio::time::timeout(deadline, wait)
                    .await
                    .unwrap_or(Err(IpcError::RemoteUnavailable))
                {
                    Ok(true) => continue,
                    Ok(false) => tokio::time::sleep(schedule.current_interval()).await,
                    Err(_) => {
                        status(&service, &config.account_id, SyncState::Retrying);
                        tokio::time::sleep(schedule.record_error()).await;
                    }
                }
            }
            Err(_) => {
                status(&service, &config.account_id, SyncState::Retrying);
                tokio::time::sleep(schedule.record_error()).await;
            }
        }
    }
}

async fn checkpoint(
    service: &Arc<DesktopService>,
    account: &str,
    folder: &str,
) -> Result<FolderSyncState, IpcError> {
    let account = account.to_string();
    let folder = folder.to_string();
    service
        .blocking(move |s| {
            Ok(s.store
                .get_folder_sync_state(&account, &folder)
                .map_err(storage_error)?
                .unwrap_or(FolderSyncState {
                    account_id: account,
                    folder_name: folder,
                    uidvalidity: 0,
                    uidnext: 1,
                    highest_modseq: None,
                }))
        })
        .await
}

pub async fn sync_account_once(
    service: &Arc<DesktopService>,
    config: &AccountConfigDto,
) -> Result<usize, IpcError> {
    let host = sync_host::DesktopSyncHost { service, config };
    sync_runtime::sync_account_once(&host)
        .await
        .map_err(|error| match error {
            sync_runtime::SyncError::Host(error) => error,
            sync_runtime::SyncError::RemoteUnavailable => IpcError::RemoteUnavailable,
        })
}

async fn category_loop(service: Arc<DesktopService>) {
    loop {
        // INVARIANT: Yield between local batches; backfill cannot stall alert/outbox scheduling.
        let delay = match async {
            let categories = service.backfill_categories().await?;
            let review = service.backfill_sender_review().await?;
            let conversations = service.backfill_conversations().await?;
            Ok::<_, IpcError>(categories.max(review).max(conversations))
        }
        .await
        {
            Ok(300) => Duration::from_millis(25),
            Ok(_) => Duration::from_secs(5),
            Err(_) => {
                status(&service, "", SyncState::MutationFailed);
                Duration::from_secs(5)
            }
        };
        tokio::time::sleep(delay).await;
    }
}

async fn outbox_loop(service: Arc<DesktopService>) {
    loop {
        if service.process_rules().await.is_err() {
            status(&service, "", SyncState::MutationFailed);
        }
        let _ = service.process_reminders().await;
        let _ = service.process_alerts().await;
        let _ = process_outbox_once(&service).await;
        let _ = service.reconcile_snoozes().await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

pub async fn process_outbox_once(service: &Arc<DesktopService>) -> Result<(), IpcError> {
    let _worker = service.outbox_gate.lock().await;
    let mut failed = std::collections::HashSet::new();
    let mut cursor = 0;
    loop {
        let entries = service
            .blocking(move |s| {
                s.store
                    .pending_mutations_after(cursor)
                    .map_err(storage_error)
            })
            .await?;
        if entries.is_empty() {
            break;
        }
        for (id, json) in entries {
            cursor = id;
            let mutation: RemoteMutation =
                serde_json::from_str(&json).map_err(|_| IpcError::StorageUnavailable)?;
            if failed.contains(&mutation.account_id) {
                continue;
            }
            let failed_attempt = process_remote_mutation(service, id, mutation.clone())
                .await
                .is_err();
            let message_id = mutation.message_id.clone();
            service
                .blocking(move |s| {
                    s.store
                        .record_conversation_attempt(&message_id, failed_attempt)
                        .map_err(storage_error)
                })
                .await?;
            if failed_attempt {
                status(service, &mutation.account_id, SyncState::MutationFailed);
                // INVARIANT: Preserve per-account ordering without starving other configured accounts.
                failed.insert(mutation.account_id);
            } else {
                status(service, &mutation.account_id, SyncState::ResyncRequired);
            }
        }
    }
    Ok(())
}

async fn process_remote_mutation(
    service: &Arc<DesktopService>,
    id: i64,
    mut mutation: RemoteMutation,
) -> Result<(), IpcError> {
    let message_id = mutation.message_id.clone();
    let current = service
        .blocking(move |s| {
            s.store
                .get_message_by_id(&message_id)
                .map_err(storage_error)
        })
        .await?;
    let Some(current) = current else {
        return Ok(());
    };
    if current.folder != mutation.folder {
        mutation.folder = current.folder;
        mutation.uid = current.uid;
        mutation.uidvalidity = checkpoint(service, &mutation.account_id, &mutation.folder)
            .await?
            .uidvalidity;
    }
    let config = service.account(mutation.account_id.clone()).await?;
    let outcome = tokio::time::timeout(
        Duration::from_secs(30),
        service.remote.mutate(&config, &mutation),
    )
    .await
    .map_err(|_| IpcError::RemoteUnavailable)??;
    // Hold the local mutation lock only for persistence, never across network I/O.
    let _guard = service.mutation_gate.lock().await;
    service
        .blocking(move |s| match outcome {
            MutationOutcome::FlagsUpdated => s.store.complete_mutation(id).map_err(storage_error),
            MutationOutcome::Moved {
                folder,
                uid,
                uidvalidity,
            } => {
                let state = FolderSyncState {
                    account_id: mutation.account_id,
                    folder_name: folder,
                    uidvalidity: uidvalidity.unwrap_or(0),
                    uidnext: 1,
                    highest_modseq: None,
                };
                s.store
                    .complete_remote_move(id, &mutation.message_id, &state, uid)
                    .map_err(storage_error)
            }
        })
        .await
}

async fn embedding_loop(service: Arc<DesktopService>) {
    loop {
        let delay = match service.index_embeddings().await {
            Ok(n) if n > 0 => {
                status(&service, "", SyncState::ResyncRequired);
                Duration::from_millis(100)
            }
            Ok(_) => Duration::from_secs(2),
            Err(_) => {
                status(&service, "", SyncState::MutationFailed);
                Duration::from_secs(30)
            }
        };
        tokio::time::sleep(delay).await;
    }
}

pub(crate) fn index_injected_provider(
    service: &DesktopService,
    records: &[MessageRecord],
    events: &BatchEvents,
) -> Result<(), IpcError> {
    // INVARIANT: Preserve the existing injectable service contract. Production always
    // uses new_learned and its separate consent/generation-managed worker, never this path.
    if service.learned.is_some() {
        return Ok(());
    }
    if let Some(provider) = &service.embedding {
        for record in records {
            let text = format!(
                "{}\n{}",
                record.subject,
                record.text_body.as_deref().unwrap_or_default()
            );
            match provider.embed_int8(&text) {
                Ok(vector) => service
                    .store
                    .insert_message_embedding(&record.id, &vector)
                    .map_err(storage_error)?,
                Err(err) => events.on_embedding_failed(&record.id, &err),
            }
        }
    }
    Ok(())
}
