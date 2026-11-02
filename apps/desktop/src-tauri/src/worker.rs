use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mail_engine::{AdaptivePollConfig, AdaptivePollSchedule, ParsedInboundMessage};
use storage::{AttachmentRecord, FolderRecord, FolderSyncState, MailRepository, MessageRecord};
use tokio::sync::watch;
use triage_ai::{
    DefaultDeterministicTriage, DeterministicTriage, DeterministicTriageResult, TriageCategory,
    TriageEventHook,
};

use crate::events::BatchEvents;
use crate::remote::{MutationOutcome, RemoteMutation};
use crate::service::{now_ms, storage_error};
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
                tokio::select! { _ = outbox_loop(service) => {}, _ = stop.changed() => {} }
            }));
        }
        Ok(())
    }

    pub fn start_account(&self, config: AccountConfigDto) -> Result<(), IpcError> {
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

fn status(service: &DesktopService, account: &str, state: SyncState) {
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
    // 1. Discover folders from remote and save catalog
    let discovered = service.remote.discover_folders(config).await;
    if let Ok(ref discovered_list) = discovered {
        let account_id = config.account_id.clone();
        let folder_records: Vec<FolderRecord> = discovered_list
            .iter()
            .map(|f| FolderRecord {
                account_id: account_id.clone(),
                folder_name: f.name.clone(),
                display_name: f.display_name.clone(),
                delimiter: f.delimiter.clone(),
                role: f
                    .role
                    .map(|r| r.as_role_str())
                    .unwrap_or("other")
                    .to_string(),
                is_selectable: f.is_selectable,
                total_messages: None,
                unread_messages: None,
                updated_at_epoch_ms: now_ms(),
            })
            .collect();

        let acct_copy = account_id.clone();
        let _ = service
            .blocking(move |s| {
                s.store
                    .save_folders(&acct_copy, &folder_records)
                    .map_err(storage_error)
            })
            .await;
    }

    // 2. Determine folders to synchronize (selectable folders)
    let mut folders: Vec<String> = Vec::new();
    if let Ok(ref discovered_list) = discovered {
        for f in discovered_list {
            if f.is_selectable {
                folders.push(f.name.clone());
            }
        }
    }
    if folders.is_empty() {
        let account = config.account_id.clone();
        folders = service
            .blocking(move |s| s.store.list_synced_folders(&account).map_err(storage_error))
            .await?;
        if !folders.iter().any(|f| f == "INBOX") {
            folders.push("INBOX".into());
        }
    } else if let Some(pos) = folders.iter().position(|f| f.eq_ignore_ascii_case("INBOX")) {
        let inbox = folders.remove(pos);
        folders.insert(0, inbox);
    }

    let mut count = 0;
    for folder in folders {
        loop {
            let previous = checkpoint(service, &config.account_id, &folder).await?;
            let events = Arc::new(BatchEvents::new(
                config.account_id.clone(),
                service.events.clone(),
            ));
            let result = tokio::time::timeout(
                Duration::from_secs(120),
                service.remote.sync(config, &previous, events.as_ref()),
            )
            .await
            .map_err(|_| IpcError::RemoteUnavailable)??;
            if result.uidvalidity == 0 {
                return Err(IpcError::RemoteUnavailable);
            }
            let batch_len = result.new_messages.len();
            count += batch_len;
            let state = FolderSyncState {
                account_id: config.account_id.clone(),
                folder_name: folder.clone(),
                uidvalidity: result.uidvalidity,
                uidnext: if result.uidvalidity_reset {
                    1
                } else {
                    result.new_last_uid.saturating_add(1)
                },
                highest_modseq: result.highest_modseq,
            };
            let _guard = service.mutation_gate.lock().await;
            service
                .blocking(move |s| {
                    let records = result
                        .new_messages
                        .into_iter()
                        .map(|m| message_record(m, &state))
                        .collect::<Result<Vec<_>, _>>()?;
                    let flags: Vec<_> = result
                        .flag_deltas
                        .into_iter()
                        .map(|f| {
                            (
                                f.uid,
                                f.flags
                                    .iter()
                                    .any(|flag| flag.eq_ignore_ascii_case("\\Seen")),
                            )
                        })
                        .collect();
                    let records = s
                        .store
                        .commit_sync_batch(&records, &flags, &state, result.uidvalidity_reset)
                        .map_err(storage_error)?;
                    for record in &records {
                        let category = record
                            .category_badge
                            .as_deref()
                            .and_then(|c| match c {
                                "Security" => Some(TriageCategory::Security),
                                "Financial" => Some(TriageCategory::Financial),
                                "Shopping" => Some(TriageCategory::Shopping),
                                "Newsletter" => Some(TriageCategory::Newsletter),
                                "Personal" => Some(TriageCategory::Personal),
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
                    }
                    // Index only with an explicitly configured provider; missing models never trigger downloads.
                    if let Some(provider) = &s.embedding {
                        for record in &records {
                            let text = format!(
                                "{}\n{}",
                                record.subject,
                                record.text_body.as_deref().unwrap_or_default()
                            );
                            match provider.embed_int8(&text) {
                                Ok(vector) => {
                                    s.store
                                        .insert_message_embedding(&record.id, &vector)
                                        .map_err(storage_error)?;
                                }
                                Err(err) => {
                                    events.on_embedding_failed(&record.id, &err);
                                    eprintln!(
                                        "[worker] failed to generate embedding for message {}: {}",
                                        record.id, err
                                    );
                                }
                            }
                        }
                    }
                    // INVARIANT: Progress is published only after messages and checkpoints commit together.
                    events.committed_with_state(if result.uidvalidity_reset {
                        SyncState::ResyncRequired
                    } else {
                        SyncState::Idle
                    });
                    Ok(())
                })
                .await?;

            if batch_len < mail_engine::DEFAULT_SYNC_UID_BATCH_SIZE as usize
                || result.uidvalidity_reset
            {
                break;
            }
        }
    }
    Ok(count)
}

fn message_record(
    message: ParsedInboundMessage,
    state: &FolderSyncState,
) -> Result<MessageRecord, IpcError> {
    if message.uid == 0 {
        return Err(IpcError::InvalidRequest);
    }
    let id = serde_json::to_string(&(
        &state.account_id,
        &state.folder_name,
        state.uidvalidity,
        message.uid,
    ))
    .map_err(|_| IpcError::Internal)?;
    let unsubscribe = message.unsubscribe;
    let supported = unsubscribe
        .as_ref()
        .is_some_and(|u| u.supports_rfc8058_one_click);
    let triage = DefaultDeterministicTriage.analyze(
        &message.subject,
        &message.sender,
        message.text_plain.as_deref().unwrap_or_default(),
        supported,
    );
    let attachments = message
        .attachments
        .into_iter()
        .map(|a| {
            Ok(AttachmentRecord {
                id: format!("{id}:{}", a.part_id),
                message_id: id.clone(),
                part_id: a.part_id,
                filename: a.filename,
                content_type: a.mime_type,
                size_bytes: a
                    .size_bytes
                    .try_into()
                    .map_err(|_| IpcError::InvalidRequest)?,
                content_id: a.content_id,
                is_inline: a.is_inline,
            })
        })
        .collect::<Result<Vec<_>, IpcError>>()?;
    Ok(MessageRecord {
        id,
        account_id: state.account_id.clone(),
        folder: state.folder_name.clone(),
        uid: message.uid,
        message_id_header: message.message_id_header,
        in_reply_to: message.in_reply_to,
        references_header: message.references_header,
        subject: message.subject,
        sender: message.sender,
        recipients: message.recipients.join(", "),
        received_at_epoch_ms: message.date_epoch_ms.unwrap_or_else(now_ms),
        is_read: false,
        has_attachments: !attachments.is_empty(),
        category_badge: Some(format!("{:?}", triage.category)),
        triage_badges: triage.badges,
        raw_headers: message.raw_headers.unwrap_or_default(),
        text_body: message.text_plain,
        html_body: message.html_raw,
        list_unsubscribe: unsubscribe
            .as_ref()
            .and_then(|u| u.raw_list_unsubscribe.clone()),
        list_unsubscribe_post: unsubscribe.and_then(|u| u.raw_list_unsubscribe_post),
        one_click_unsubscribe_supported: supported,
        created_at_epoch_ms: now_ms(),
        attachments,
        tags: Vec::new(),
        is_starred: false,
        snooze_until_epoch_ms: None,
        provider_message_id: message.provider_message_id,
    })
}

async fn outbox_loop(service: Arc<DesktopService>) {
    loop {
        let _ = process_outbox_once(&service).await;
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
            if process_remote_mutation(service, id, mutation.clone())
                .await
                .is_err()
            {
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
