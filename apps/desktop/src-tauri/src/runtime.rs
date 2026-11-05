//! Tauri-specific command registration and native lifecycle. No core crate depends on this module.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{ipc::Channel, Manager, State};
use tokio::sync::OnceCell;
use zeroize::Zeroizing;

use crate::dto::*;
use crate::onboarding::{self, AuthorizationSessions, IncomingAuthorization, NativeTokenExchange};
use crate::ownership::bootstrap;
use crate::service::{DesktopService, IncomingCredentials, IpcError};
use crate::worker::WorkerSupervisor;

pub struct RuntimeState {
    data_dir: PathBuf,
    service: OnceCell<Arc<DesktopService>>,
    workers: OnceCell<Arc<WorkerSupervisor>>,
    subscriptions: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    authorizations: AuthorizationSessions,
    recovery_gate: Arc<tokio::sync::Mutex<()>>,
    ownership: OnceCell<Arc<sync_runtime::ownership::WriterOwnership>>,
}

impl RuntimeState {
    fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            service: OnceCell::new(),
            workers: OnceCell::new(),
            subscriptions: Mutex::new(HashMap::new()),
            authorizations: AuthorizationSessions::default(),
            recovery_gate: Arc::new(tokio::sync::Mutex::new(())),
            ownership: OnceCell::new(),
        }
    }

    async fn initialize(
        &self,
        passphrase: Option<Zeroizing<String>>,
    ) -> Result<Arc<DesktopService>, IpcError> {
        let recovery = Arc::new(self.recovery_gate.clone().lock_owned().await);
        // INVARIANT: Always acquire recovery_gate before ownership. Retain the
        // guard through failed unlock/retry; cancellation cannot unlock active work.
        let ownership = self.ownership().await?;
        let service = self
            .service
            .get_or_try_init(|| async {
                let path = self.data_dir.clone();
                let recovery = recovery.clone();
                tokio::task::spawn_blocking(move || {
                    // INVARIANT: A canceled caller cannot release the in-process
                    // gate while its uncancelable bootstrap is still writing.
                    let _recovery = recovery;
                    bootstrap(&path, passphrase, ownership)
                })
                .await
                .map_err(|_| IpcError::Internal)?
            })
            .await?
            .clone();
        self.workers
            .get_or_try_init(|| async {
                let supervisor = WorkerSupervisor::new(service.clone());
                supervisor.start().await?;
                Ok::<_, IpcError>(supervisor)
            })
            .await?;
        Ok(service)
    }

    async fn ownership(&self) -> Result<Arc<sync_runtime::ownership::WriterOwnership>, IpcError> {
        self.ownership
            .get_or_try_init(|| async {
                let path = self.data_dir.clone();
                tokio::task::spawn_blocking(move || crate::ownership::acquire(&path))
                    .await
                    .map_err(|_| IpcError::Internal)?
            })
            .await
            .cloned()
    }
}

#[tauri::command]
pub async fn cmd_alert_schedule(
    state: State<'_, RuntimeState>,
    id: String,
) -> Result<storage::alerts::schedule::ScheduleState, IpcError> {
    state.initialize(None).await?.alert_schedule(id).await
}
#[tauri::command]
pub async fn cmd_save_alert_schedule(
    state: State<'_, RuntimeState>,
    policy: triage_ai::alerts::schedule::SchedulePolicy,
    consent: bool,
) -> Result<triage_ai::alerts::schedule::SchedulePolicy, IpcError> {
    state
        .initialize(None)
        .await?
        .save_alert_schedule(policy, consent)
        .await
}
#[tauri::command]
pub async fn cmd_preview_alert_schedule(
    state: State<'_, RuntimeState>,
    policy: triage_ai::alerts::schedule::SchedulePolicy,
) -> Result<crate::alert_schedule::SchedulePreview, IpcError> {
    state.initialize(None).await?;
    DesktopService::preview_alert_schedule(policy)
}

#[tauri::command]
pub async fn cmd_list_alert_destinations(
    state: State<'_, RuntimeState>,
) -> Result<Vec<triage_ai::alerts::Destination>, IpcError> {
    state.initialize(None).await?.list_alerts().await
}
#[tauri::command]
pub async fn cmd_save_alert_destination(
    state: State<'_, RuntimeState>,
    destination: triage_ai::alerts::Destination,
    secret: Option<vault::alerts::IncomingWebhook>,
    consent: bool,
) -> Result<triage_ai::alerts::Destination, IpcError> {
    state
        .initialize(None)
        .await?
        .save_alert(destination, secret, consent)
        .await
}
#[tauri::command]
pub async fn cmd_delete_alert_destination(
    state: State<'_, RuntimeState>,
    id: String,
    revision: u32,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .delete_alert(id, revision)
        .await
}
#[tauri::command]
pub async fn cmd_alert_history(
    state: State<'_, RuntimeState>,
) -> Result<Vec<storage::alerts::DeliveryHistory>, IpcError> {
    state.initialize(None).await?.alert_history().await
}
#[tauri::command]
pub async fn cmd_test_alert(
    state: State<'_, RuntimeState>,
    id: String,
    revision: u32,
    consent: bool,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .test_alert(id, revision, consent)
        .await
}
#[tauri::command]
pub async fn cmd_replay_alert(
    state: State<'_, RuntimeState>,
    ids: Vec<String>,
    destination: String,
    revision: u32,
    consent: bool,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .replay_alert(ids, destination, revision, consent)
        .await
}
#[tauri::command]
pub async fn cmd_local_alert_available(state: State<'_, RuntimeState>) -> Result<bool, IpcError> {
    Ok(state.initialize(None).await?.local_alert_available().await)
}

#[tauri::command]
pub async fn cmd_review_preferences(
    state: State<'_, RuntimeState>,
    account_id: String,
) -> Result<storage::sender_review::ReviewPreferences, IpcError> {
    state
        .initialize(None)
        .await?
        .review_preferences(account_id)
        .await
}
#[tauri::command]
pub async fn cmd_save_review_preferences(
    state: State<'_, RuntimeState>,
    value: storage::sender_review::ReviewPreferences,
) -> Result<storage::sender_review::ReviewPreferences, IpcError> {
    state
        .initialize(None)
        .await?
        .save_review_preferences(value)
        .await
}
#[tauri::command]
pub async fn cmd_review_metadata(
    state: State<'_, RuntimeState>,
    message_id: String,
) -> Result<Option<storage::sender_review::SenderChoice>, IpcError> {
    state
        .initialize(None)
        .await?
        .review_metadata(message_id)
        .await
}
#[tauri::command]
pub async fn cmd_list_sender_choices(
    state: State<'_, RuntimeState>,
    account_id: String,
    offset: usize,
) -> Result<Vec<storage::sender_review::SenderChoice>, IpcError> {
    state
        .initialize(None)
        .await?
        .list_sender_choices(account_id, offset)
        .await
}
#[tauri::command]
pub async fn cmd_save_sender_choice(
    state: State<'_, RuntimeState>,
    value: storage::sender_review::SenderChoice,
) -> Result<storage::sender_review::SenderChoice, IpcError> {
    state
        .initialize(None)
        .await?
        .save_sender_choice(value)
        .await
}
#[tauri::command]
pub async fn cmd_conversation_progress(
    state: State<'_, RuntimeState>,
    account_id: Option<String>,
) -> Result<storage::conversations::ConversationProgress, IpcError> {
    state
        .initialize(None)
        .await?
        .conversation_progress(account_id)
        .await
}
#[tauri::command]
pub async fn cmd_search_conversations(
    state: State<'_, RuntimeState>,
    query: SearchQueryDto,
    context: storage::conversations::ConversationQuery,
) -> Result<storage::conversations::ConversationPage, IpcError> {
    state
        .initialize(None)
        .await?
        .search_conversations(query, context)
        .await
}
#[tauri::command]
pub async fn cmd_conversation_members(
    state: State<'_, RuntimeState>,
    query: SearchQueryDto,
    context: storage::conversations::ConversationQuery,
    id: String,
    offset: usize,
) -> Result<storage::conversations::ConversationMembers, IpcError> {
    state
        .initialize(None)
        .await?
        .conversation_members(query, context, id, offset)
        .await
}
#[tauri::command]
pub async fn cmd_preview_conversation(
    state: State<'_, RuntimeState>,
    query: SearchQueryDto,
    context: storage::conversations::ConversationQuery,
    id: String,
    all: bool,
    action: storage::conversations::ConversationAction,
) -> Result<storage::conversations::ConversationPreview, IpcError> {
    state
        .initialize(None)
        .await?
        .preview_conversation(query, context, id, all, action)
        .await
}
#[tauri::command]
pub async fn cmd_apply_conversation(
    state: State<'_, RuntimeState>,
    token: String,
) -> Result<Vec<String>, IpcError> {
    state
        .initialize(None)
        .await?
        .apply_conversation(token)
        .await
}
#[tauri::command]
pub async fn cmd_conversation_status(
    state: State<'_, RuntimeState>,
    ids: Vec<String>,
) -> Result<Vec<storage::conversations::MutationStatus>, IpcError> {
    state.initialize(None).await?.conversation_status(ids).await
}

#[tauri::command]
pub async fn cmd_search_sender_review(
    state: State<'_, RuntimeState>,
    query: SearchQueryDto,
    review_state: storage::sender_review::ReviewState,
) -> Result<Vec<MessageRowDto>, IpcError> {
    state
        .initialize(None)
        .await?
        .search_sender_review(query, review_state)
        .await
}

#[tauri::command]
pub async fn cmd_category_metadata(
    state: State<'_, RuntimeState>,
    message_id: String,
) -> Result<storage::categories::CategoryMetadata, IpcError> {
    state
        .initialize(None)
        .await?
        .category_metadata(message_id)
        .await
}
#[tauri::command]
pub async fn cmd_correct_category(
    state: State<'_, RuntimeState>,
    message_id: String,
    revision: u32,
    category: Option<triage_ai::TriageCategory>,
) -> Result<storage::categories::CategoryMetadata, IpcError> {
    state
        .initialize(None)
        .await?
        .correct_category(message_id, revision, category)
        .await
}
#[tauri::command]
pub async fn cmd_list_category_overrides(
    state: State<'_, RuntimeState>,
    offset: Option<usize>,
) -> Result<Vec<storage::categories::SenderOverride>, IpcError> {
    state
        .initialize(None)
        .await?
        .list_category_overrides(offset.unwrap_or(0))
        .await
}
#[tauri::command]
pub async fn cmd_save_category_override(
    state: State<'_, RuntimeState>,
    value: storage::categories::SenderOverride,
) -> Result<storage::categories::SenderOverride, IpcError> {
    state
        .initialize(None)
        .await?
        .save_category_override(value)
        .await
}
#[tauri::command]
pub async fn cmd_search_category(
    state: State<'_, RuntimeState>,
    query: crate::SearchQueryDto,
    category: triage_ai::TriageCategory,
) -> Result<Vec<MessageRowDto>, IpcError> {
    state
        .initialize(None)
        .await?
        .search_category(query, category)
        .await
}

#[tauri::command]
pub async fn cmd_list_saved_views(
    state: State<'_, RuntimeState>,
) -> Result<Vec<storage::saved_views::SavedView>, IpcError> {
    state.initialize(None).await?.list_saved_views().await
}
#[tauri::command]
pub async fn cmd_save_saved_view(
    state: State<'_, RuntimeState>,
    view: storage::saved_views::SavedView,
) -> Result<storage::saved_views::SavedView, IpcError> {
    state.initialize(None).await?.save_saved_view(view).await
}
#[tauri::command]
pub async fn cmd_delete_saved_view(
    state: State<'_, RuntimeState>,
    id: String,
    revision: u32,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .delete_saved_view(id, revision)
        .await
}
#[tauri::command]
pub async fn cmd_search_saved_view(
    state: State<'_, RuntimeState>,
    id: String,
    revision: u32,
    limit: usize,
    offset: usize,
) -> Result<Vec<MessageRowDto>, IpcError> {
    state
        .initialize(None)
        .await?
        .search_saved_view(id, revision, limit, offset)
        .await
}

#[tauri::command]
pub async fn cmd_list_due_date_items(
    state: State<'_, RuntimeState>,
    account_id: Option<String>,
) -> Result<Vec<crate::DueDateItemDto>, IpcError> {
    state
        .initialize(None)
        .await?
        .list_due_date_items(account_id)
        .await
}

#[tauri::command]
pub async fn cmd_dismiss_due_date_item(
    state: State<'_, RuntimeState>,
    message_id: String,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .dismiss_due_date_item(message_id)
        .await
}

#[tauri::command]
pub async fn cmd_list_reminder_configs(
    state: State<'_, RuntimeState>,
) -> Result<Vec<crate::ScheduledReminderConfigDto>, IpcError> {
    state.initialize(None).await?.list_reminder_configs().await
}

#[tauri::command]
pub async fn cmd_save_reminder_config(
    state: State<'_, RuntimeState>,
    config: crate::ScheduledReminderConfigDto,
) -> Result<crate::ScheduledReminderConfigDto, IpcError> {
    state
        .initialize(None)
        .await?
        .save_reminder_config(config)
        .await
}

#[tauri::command]
pub async fn cmd_delete_reminder_config(
    state: State<'_, RuntimeState>,
    id: String,
    revision: u32,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .delete_reminder_config(id, revision)
        .await
}

#[tauri::command]
pub async fn cmd_list_rules(
    state: State<'_, RuntimeState>,
) -> Result<Vec<triage_ai::rules::Rule>, IpcError> {
    state.initialize(None).await?.list_rules().await
}
#[tauri::command]
pub async fn cmd_save_rule(
    state: State<'_, RuntimeState>,
    rule: triage_ai::rules::Rule,
) -> Result<triage_ai::rules::Rule, IpcError> {
    state.initialize(None).await?.save_rule(rule).await
}
#[tauri::command]
pub async fn cmd_delete_rule(
    state: State<'_, RuntimeState>,
    id: String,
    revision: u32,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .delete_rule(id, revision)
        .await
}
#[tauri::command]
pub async fn cmd_reorder_rules(
    state: State<'_, RuntimeState>,
    revisions: Vec<(String, u32)>,
) -> Result<(), IpcError> {
    state.initialize(None).await?.reorder_rules(revisions).await
}
#[tauri::command]
pub async fn cmd_simulate_rules(
    state: State<'_, RuntimeState>,
    ids: Vec<String>,
    draft: Option<triage_ai::rules::Rule>,
) -> Result<Vec<(String, triage_ai::rules::RulePlan)>, IpcError> {
    state
        .initialize(None)
        .await?
        .simulate_rules(ids, draft)
        .await
}
#[tauri::command]
pub async fn cmd_rule_draft(
    state: State<'_, RuntimeState>,
    message_id: String,
    rule_id: String,
) -> Result<triage_ai::rules::Rule, IpcError> {
    state
        .initialize(None)
        .await?
        .rule_draft(message_id, rule_id)
        .await
}

/// Optional explicit unlock for machines without an OS keyring. Passphrases never leave the vault cipher.
#[tauri::command]
pub async fn cmd_unlock_vault(
    state: State<'_, RuntimeState>,
    passphrase: String,
) -> Result<(), IpcError> {
    state.initialize(Some(Zeroizing::new(passphrase))).await?;
    Ok(())
}

/// Omitted `online` keeps existing callers strictly offline.
#[tauri::command]
pub async fn cmd_discover_email_provider(
    email: String,
    online: Option<bool>,
) -> Result<DiscoveredProviderDto, IpcError> {
    onboarding::discover_provider(
        &email,
        online.unwrap_or(false),
        &mail_engine::discovery::NativeDiscovery,
    )
    .await
}

/// Invoked only by the explicit confirmation button for the displayed host (TLS port 993).
#[tauri::command]
pub async fn cmd_probe_email_provider(host: String) -> Result<bool, IpcError> {
    onboarding::probe_provider(&host, &mail_engine::discovery::NativeDiscovery).await
}

#[tauri::command]
pub async fn cmd_list_accounts(
    state: State<'_, RuntimeState>,
) -> Result<Vec<AccountConfigDto>, IpcError> {
    state.initialize(None).await?.list_accounts().await
}

#[tauri::command]
pub async fn cmd_get_account_avatar(
    state: State<'_, RuntimeState>,
    account_id: String,
) -> Result<Option<storage::AccountAvatar>, IpcError> {
    state
        .initialize(None)
        .await?
        .get_account_avatar(account_id)
        .await
}

#[tauri::command]
pub async fn cmd_set_account_avatar(
    state: State<'_, RuntimeState>,
    account_id: String,
    bytes: Vec<u8>,
) -> Result<storage::AccountAvatar, IpcError> {
    if bytes.len() > crate::avatars::MAX_AVATAR_BYTES {
        return Err(IpcError::InvalidRequest);
    }
    state
        .initialize(None)
        .await?
        .set_account_avatar(account_id, bytes)
        .await
}

#[tauri::command]
pub async fn cmd_remove_account_avatar(
    state: State<'_, RuntimeState>,
    account_id: String,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .remove_account_avatar(account_id)
        .await
}

#[tauri::command]
pub async fn cmd_list_folders(
    state: State<'_, RuntimeState>,
    account_id: String,
) -> Result<Vec<FolderDto>, IpcError> {
    state.initialize(None).await?.list_folders(account_id).await
}

#[tauri::command]
pub async fn cmd_add_account(
    state: State<'_, RuntimeState>,
    config: AccountConfigDto,
    credentials: IncomingCredentials,
) -> Result<String, IpcError> {
    let service = state.initialize(None).await?;
    let id = service.add_account(config, credentials).await?;
    if let Some(workers) = state.workers.get() {
        workers.start_account(service.account(id.clone()).await?)?;
    }
    Ok(id)
}

#[tauri::command]
pub async fn cmd_authorize_account(
    state: State<'_, RuntimeState>,
    window: tauri::WebviewWindow,
    config: AccountConfigDto,
    authorization: IncomingAuthorization,
    on_ready: Channel<()>,
) -> Result<String, IpcError> {
    let (_lease, cancel) = state.authorizations.begin(window.label())?;
    // INVARIANT: Enable UI cancellation only after the native session exists.
    on_ready.send(()).map_err(|_| IpcError::RemoteUnavailable)?;
    let service = state.initialize(None).await?;
    let id = onboarding::authorize(
        service.clone(),
        config,
        authorization,
        cancel,
        onboarding::open_browser,
        &NativeTokenExchange,
    )
    .await?;
    if let Some(workers) = state.workers.get() {
        workers.start_account(service.account(id.clone()).await?)?;
    }
    Ok(id)
}

#[tauri::command]
pub fn cmd_cancel_authorization(state: State<'_, RuntimeState>, window: tauri::WebviewWindow) {
    state.authorizations.cancel(window.label());
}

#[tauri::command]
pub async fn cmd_get_settings(
    state: State<'_, RuntimeState>,
) -> Result<crate::settings::SettingsDto, IpcError> {
    state.initialize(None).await?.get_settings().await
}

#[tauri::command]
pub async fn cmd_save_settings(
    state: State<'_, RuntimeState>,
    settings: crate::settings::AiSettings,
    api_key: crate::settings::IncomingApiKey,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .save_settings(settings, api_key)
        .await
}

#[tauri::command]
pub async fn cmd_set_tokenizer(
    state: State<'_, RuntimeState>,
    tokenizer: storage::FtsTokenizerMode,
) -> Result<(), IpcError> {
    state.initialize(None).await?.set_tokenizer(tokenizer).await
}

#[tauri::command]
pub async fn cmd_set_experimental_semantic_search(
    state: State<'_, RuntimeState>,
    enabled: bool,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .set_experimental_semantic_search(enabled)
        .await
}

#[tauri::command]
pub async fn cmd_download_model(
    state: State<'_, RuntimeState>,
    request: crate::model_download::ModelDownloadRequest,
) -> Result<(), IpcError> {
    state.initialize(None).await?.download_model(request).await
}

#[tauri::command]
pub async fn cmd_summarize_message(
    state: State<'_, RuntimeState>,
    message_id: String,
    consent: bool,
    expected: crate::settings::AiSettings,
) -> Result<String, IpcError> {
    state
        .initialize(None)
        .await?
        .summarize_message(message_id, consent, expected)
        .await
}

#[tauri::command]
pub async fn cmd_search_messages(
    state: State<'_, RuntimeState>,
    query: SearchQueryDto,
) -> Result<Vec<MessageRowDto>, IpcError> {
    state.initialize(None).await?.search_messages(query).await
}

#[tauri::command]
pub async fn cmd_triage_action(
    state: State<'_, RuntimeState>,
    mutation: TriageMutationDto,
) -> Result<(), IpcError> {
    state.initialize(None).await?.triage_action(mutation).await
}

#[tauri::command]
pub async fn cmd_get_message_body(
    state: State<'_, RuntimeState>,
    message_id: MessageId,
) -> Result<MessageDetailDto, IpcError> {
    state
        .initialize(None)
        .await?
        .get_message_body(message_id)
        .await
}

#[tauri::command]
pub async fn cmd_fetch_attachment(
    state: State<'_, RuntimeState>,
    request: AttachmentRequestDto,
) -> Result<AttachmentBytesDto, IpcError> {
    state
        .initialize(None)
        .await?
        .fetch_attachment(request)
        .await
}

#[tauri::command]
pub async fn cmd_subscribe_sync(
    state: State<'_, RuntimeState>,
    window: tauri::WebviewWindow,
    on_progress: Channel<SyncProgressEvent>,
) -> Result<(), IpcError> {
    let service = state.initialize(None).await?;
    let mut receiver = service.events.subscribe();
    let task = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if on_progress.send(event).is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    if let Ok(accounts) = service.list_accounts().await {
                        for account in accounts {
                            let event = SyncProgressEvent {
                                account_id: account.account_id,
                                state: SyncState::ResyncRequired,
                                new_messages_count: 0,
                                triage_alerts: Vec::new(),
                                embedding_errors_count: 0,
                            };
                            if on_progress.send(event).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        }
    });
    let mut subscriptions = state.subscriptions.lock().map_err(|_| IpcError::Internal)?;
    if let Some(previous) = subscriptions.insert(window.label().into(), task) {
        previous.abort();
    }
    Ok(())
}

#[tauri::command]
pub async fn cmd_choose_transfer_file(save: bool) -> Result<Option<PathBuf>, IpcError> {
    crate::file_picker::choose(save).await
}
#[tauri::command]
pub async fn cmd_write_backup(
    state: State<'_, RuntimeState>,
    path: PathBuf,
    passphrase: crate::settings::IncomingApiKey,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .backup_file(path, passphrase)
        .await
}
#[tauri::command]
pub async fn cmd_export_definitions(
    state: State<'_, RuntimeState>,
    path: PathBuf,
) -> Result<(), IpcError> {
    state.initialize(None).await?.export_file(path).await
}
#[tauri::command]
pub async fn cmd_preview_definitions(
    path: PathBuf,
) -> Result<storage::portable::PortableDefinitions, IpcError> {
    tokio::task::spawn_blocking(move || crate::backup::preview(&path))
        .await
        .map_err(|_| IpcError::Internal)?
}
#[tauri::command]
pub async fn cmd_import_definitions(
    state: State<'_, RuntimeState>,
    path: PathBuf,
    mapping: storage::portable::ReferenceMapping,
    expected: storage::portable::PortableDefinitions,
    consent: bool,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .import_file(path, mapping, expected, consent)
        .await
}
#[tauri::command]
pub async fn cmd_recovery_available(state: State<'_, RuntimeState>) -> Result<bool, IpcError> {
    let _guard = state.recovery_gate.lock().await;
    Ok(state.service.get().is_none() && std::fs::symlink_metadata(&state.data_dir).is_err())
}
#[tauri::command]
pub async fn cmd_restore_backup(
    state: State<'_, RuntimeState>,
    path: PathBuf,
    passphrase: crate::settings::IncomingApiKey,
    consent: bool,
) -> Result<(), IpcError> {
    let recovery = state.recovery_gate.clone().lock_owned().await;
    if !consent || state.service.get().is_some() {
        return Err(IpcError::InvalidRequest);
    }
    let target = state.data_dir.clone();
    let ownership = state.ownership().await?;
    tokio::task::spawn_blocking(move || {
        let _recovery = recovery;
        crate::backup::restore_owned(&path, &target, passphrase.0, ownership)
    })
    .await
    .map_err(|_| IpcError::Internal)?
}

pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            app.manage(RuntimeState::new(data));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_choose_transfer_file,
            cmd_write_backup,
            cmd_export_definitions,
            cmd_preview_definitions,
            cmd_import_definitions,
            cmd_recovery_available,
            cmd_restore_backup,
            cmd_alert_schedule,
            cmd_save_alert_schedule,
            cmd_preview_alert_schedule,
            cmd_list_alert_destinations,
            cmd_save_alert_destination,
            cmd_delete_alert_destination,
            cmd_alert_history,
            cmd_test_alert,
            cmd_replay_alert,
            cmd_local_alert_available,
            cmd_review_preferences,
            cmd_save_review_preferences,
            cmd_review_metadata,
            cmd_list_sender_choices,
            cmd_save_sender_choice,
            cmd_conversation_progress,
            cmd_search_conversations,
            cmd_conversation_members,
            cmd_preview_conversation,
            cmd_apply_conversation,
            cmd_conversation_status,
            cmd_search_sender_review,
            cmd_category_metadata,
            cmd_correct_category,
            cmd_list_category_overrides,
            cmd_save_category_override,
            cmd_search_category,
            cmd_list_saved_views,
            cmd_save_saved_view,
            cmd_delete_saved_view,
            cmd_search_saved_view,
            cmd_list_due_date_items,
            cmd_dismiss_due_date_item,
            cmd_list_reminder_configs,
            cmd_save_reminder_config,
            cmd_delete_reminder_config,
            cmd_list_rules,
            cmd_save_rule,
            cmd_delete_rule,
            cmd_reorder_rules,
            cmd_simulate_rules,
            cmd_rule_draft,
            cmd_embedding_state,
            cmd_rebuild_embeddings,
            cmd_get_settings,
            cmd_save_settings,
            cmd_set_tokenizer,
            cmd_set_experimental_semantic_search,
            cmd_download_model,
            cmd_summarize_message,
            cmd_unlock_vault,
            cmd_list_accounts,
            cmd_discover_email_provider,
            cmd_probe_email_provider,
            cmd_list_folders,
            cmd_get_account_avatar,
            cmd_set_account_avatar,
            cmd_remove_account_avatar,
            cmd_add_account,
            cmd_authorize_account,
            cmd_cancel_authorization,
            cmd_search_messages,
            cmd_triage_action,
            cmd_get_message_body,
            cmd_fetch_attachment,
            cmd_subscribe_sync
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let state = window.state::<RuntimeState>();
                state.authorizations.cancel(window.label());
                if let Ok(mut subscriptions) = state.subscriptions.lock() {
                    if let Some(task) = subscriptions.remove(window.label()) {
                        task.abort();
                    }
                };
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to construct desktop runtime");
    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state = handle.state::<RuntimeState>();
            if let Some(workers) = state.workers.get() {
                workers.shutdown();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::NativeRemoteMail;
    use serde_json::{json, Value};
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    #[tokio::test]
    async fn runtime_retains_ownership_after_failed_unlock_and_reuses_it_on_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("installation");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("recovered-local-vault"), b"recovered").unwrap();
        let state = RuntimeState::new(path.clone());
        assert!(matches!(
            state.initialize(None).await,
            Err(IpcError::VaultUnavailable)
        ));
        assert!(matches!(
            sync_runtime::ownership::WriterOwnership::acquire(&path),
            Err(sync_runtime::ownership::OwnershipError::Busy)
        ));
        let first = state.ownership().await.unwrap();
        let service = state
            .initialize(Some(Zeroizing::new(
                "synthetic runtime ownership fixture phrase".into(),
            )))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&first, service._ownership.as_ref().unwrap()));
        state.workers.get().unwrap().shutdown();
    }
    use vault::SecretStore;

    fn invoke(
        window: &tauri::WebviewWindow<MockRuntime>,
        command: &str,
        body: Value,
    ) -> Result<Value, Value> {
        tauri::test::get_ipc_response(
            window,
            tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(any(windows, target_os = "android")) {
                    "https://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .map(|body| body.deserialize().unwrap())
    }

    #[test]
    fn registered_handlers_enforce_wire_contracts_and_redact_credential_parse_errors() {
        let store = Arc::new(
            storage::SqliteMailStore::open_in_memory(
                &storage::DatabaseKey::from_bytes([31; 32]),
                Default::default(),
            )
            .unwrap(),
        );
        let vault: Arc<dyn SecretStore> = Arc::new(vault::EncryptedFallbackStore::in_memory(
            Arc::new(vault::Argon2AesGcmCipher::from_key([37; 32])),
        ));
        let service = DesktopService::new(
            store,
            vault.clone(),
            Arc::new(NativeRemoteMail::new(vault)),
            None,
        );
        let directory = tempfile::tempdir().unwrap();
        let state = RuntimeState::new(directory.path().join("installation"));
        assert!(state.service.set(service.clone()).is_ok());
        // No worker starts in this fixture; all network operations remain unreachable.
        assert!(state.workers.set(WorkerSupervisor::new(service)).is_ok());
        let app = mock_builder()
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                cmd_write_backup,
                cmd_export_definitions,
                cmd_preview_definitions,
                cmd_import_definitions,
                cmd_recovery_available,
                cmd_restore_backup,
                cmd_alert_schedule,
                cmd_save_alert_schedule,
                cmd_preview_alert_schedule,
                cmd_list_alert_destinations,
                cmd_save_alert_destination,
                cmd_delete_alert_destination,
                cmd_alert_history,
                cmd_test_alert,
                cmd_replay_alert,
                cmd_local_alert_available,
                cmd_review_preferences,
                cmd_save_review_preferences,
                cmd_review_metadata,
                cmd_list_sender_choices,
                cmd_save_sender_choice,
                cmd_conversation_progress,
                cmd_search_conversations,
                cmd_conversation_members,
                cmd_preview_conversation,
                cmd_apply_conversation,
                cmd_conversation_status,
                cmd_search_sender_review,
                cmd_category_metadata,
                cmd_correct_category,
                cmd_list_category_overrides,
                cmd_save_category_override,
                cmd_search_category,
                cmd_list_saved_views,
                cmd_save_saved_view,
                cmd_delete_saved_view,
                cmd_search_saved_view,
                cmd_list_due_date_items,
                cmd_dismiss_due_date_item,
                cmd_list_reminder_configs,
                cmd_save_reminder_config,
                cmd_delete_reminder_config,
                cmd_list_rules,
                cmd_save_rule,
                cmd_delete_rule,
                cmd_reorder_rules,
                cmd_simulate_rules,
                cmd_rule_draft,
                cmd_list_accounts,
                cmd_discover_email_provider,
                cmd_probe_email_provider,
                cmd_get_account_avatar,
                cmd_set_account_avatar,
                cmd_remove_account_avatar,
                cmd_add_account,
                cmd_search_messages,
                cmd_get_message_body,
                cmd_triage_action,
                cmd_fetch_attachment,
                cmd_embedding_state,
                cmd_rebuild_embeddings,
                cmd_get_settings,
                cmd_save_settings,
                cmd_set_tokenizer,
                cmd_set_experimental_semantic_search,
                cmd_download_model,
                cmd_summarize_message
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        assert_eq!(
            invoke(&window, "cmd_recovery_available", json!({})).unwrap(),
            json!(false)
        );
        let malformed = invoke(
            &window,
            "cmd_write_backup",
            json!({"path":"/tmp/synthetic.tmb","passphrase":{"synthetic_sensitive":"never-echo"}}),
        )
        .unwrap_err();
        assert!(!malformed.to_string().contains("never-echo"));
        assert!(invoke(&window, "cmd_restore_backup", json!({"path":"/tmp/synthetic.tmb","passphrase":"synthetic backup passphrase","consent":true})).is_err());

        assert_eq!(invoke(&window,"cmd_search_sender_review",json!({"query":{"query":"","mode":"Lexical","limit":200,"offset":0},"reviewState":"Pending"})).unwrap(),json!([]));
        assert_eq!(
            invoke(
                &window,
                "cmd_review_preferences",
                json!({"accountId":"missing"})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_list_sender_choices",
                json!({"accountId":"missing","offset":0})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_review_metadata",
                json!({"messageId":"missing"})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(invoke(&window,"cmd_search_sender_review",json!({"query":{"query":"","mode":"Semantic","limit":200,"offset":0},"reviewState":"Pending"})).unwrap_err(),json!("InvalidRequest"));
        assert_eq!(invoke(&window,"cmd_save_review_preferences",json!({"value":{"account_id":"missing","revision":0,"enabled":true,"hide_in_inbox":false,"initializing":false}})).unwrap_err(),json!("InvalidRequest"));
        assert_eq!(invoke(&window,"cmd_save_sender_choice",json!({"value":{"account_id":"missing","sender":"sender@example.org","revision":1,"state":"Declined"}})).unwrap_err(),json!("InvalidRequest"));
        assert_eq!(
            invoke(&window, "cmd_list_category_overrides", json!({"offset":0})).unwrap(),
            json!([])
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_category_metadata",
                json!({"messageId":"missing"})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_correct_category",
                json!({"messageId":"missing","revision":0,"category":"Personal"})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(invoke(&window,"cmd_search_category",json!({"query":{"query":"","mode":"Lexical","limit":200,"offset":0},"category":"Notifications"})).unwrap(),json!([]));
        assert_eq!(
            invoke(&window, "cmd_conversation_progress", json!({})).unwrap(),
            json!({"indexed":0,"total":0,"ready":true})
        );
        let conversations=invoke(&window,"cmd_search_conversations",json!({"query":{"query":"","mode":"Lexical","limit":200,"offset":0},"context":{"source":"Messages","scope":null,"generation":null}})).unwrap();
        assert_eq!(conversations["rows"], json!([]));
        assert_eq!(conversations["generation"].as_str().unwrap().len(), 32);
        assert_eq!(invoke(&window,"cmd_search_conversations",json!({"query":{"query":"","mode":"Semantic","limit":200,"offset":0},"context":{"source":"Messages","scope":null,"generation":null}})).unwrap_err(),json!("InvalidRequest"));

        assert_eq!(invoke(&window,"cmd_search_category",json!({"query":{"query":"","mode":"Semantic","limit":200,"offset":0},"category":"Notifications"})).unwrap_err(),json!("InvalidRequest"));
        assert_eq!(invoke(&window,"cmd_save_category_override",json!({"value":{"account_id":"missing","sender":"sender@example.org","revision":0,"category":"Financial"}})).unwrap_err(),json!("InvalidRequest"));
        let view = storage::saved_views::SavedView {
            version: 1,
            id: "saved".into(),
            revision: 0,
            name: "Synthetic".into(),
            query: String::new(),
            mode: storage::SearchMode::Lexical,
            accounts: vec![],
            scope: None,
            filters: Default::default(),
        };
        assert_eq!(
            invoke(&window, "cmd_list_saved_views", json!({})).unwrap(),
            json!([])
        );
        let saved = invoke(&window, "cmd_save_saved_view", json!({"view":view})).unwrap();
        assert_eq!(saved["revision"], 1);
        assert_eq!(
            invoke(&window, "cmd_save_saved_view", json!({"view":view})).unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_search_saved_view",
                json!({"id":"saved","revision":1,"limit":200,"offset":0})
            )
            .unwrap(),
            json!([])
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_search_saved_view",
                json!({"id":"saved","revision":1,"limit":0,"offset":0})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_delete_saved_view",
                json!({"id":"saved","revision":2})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        invoke(
            &window,
            "cmd_delete_saved_view",
            json!({"id":"saved","revision":1}),
        )
        .unwrap();
        assert_eq!(
            invoke(&window, "cmd_save_saved_view", json!({"view":view})).unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(&window, "cmd_list_rules", json!({})).unwrap(),
            json!([])
        );
        let rule = json!({"version":1,"id":"rule","revision":0,"name":"Synthetic","enabled":true,"position":0,"all":[{"Subject":"invoice"}],"any":[],"except":[],"actions":["Star"],"stop_processing":false});
        let saved = invoke(&window, "cmd_save_rule", json!({"rule":rule})).unwrap();
        assert_eq!(saved["revision"], 1);
        assert_eq!(
            invoke(&window, "cmd_save_rule", json!({"rule":rule})).unwrap_err(),
            json!("InvalidRequest")
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_simulate_rules",
                json!({"ids":[],"draft":null})
            )
            .unwrap_err(),
            json!("InvalidRequest")
        );
        invoke(
            &window,
            "cmd_reorder_rules",
            json!({"revisions":[["rule",1]]}),
        )
        .unwrap();
        invoke(
            &window,
            "cmd_delete_rule",
            json!({"id":"rule","revision":2}),
        )
        .unwrap();
        assert_eq!(
            invoke(&window, "cmd_list_rules", json!({})).unwrap(),
            json!([])
        );
        // INVARIANT: Omitting online must not perform even a DNS lookup for an unknown domain.
        let discovery = invoke(
            &window,
            "cmd_discover_email_provider",
            json!({"email":"reader@example.com"}),
        )
        .unwrap();
        assert_eq!(discovery["source"], "Unresolved");
        assert!(discovery["imap_host"].is_null());
        assert!(!discovery.to_string().contains("reader@"));
        assert_eq!(
            invoke(
                &window,
                "cmd_probe_email_provider",
                json!({"host":"127.0.0.1"})
            ),
            Err(json!("InvalidRequest"))
        );
        let settings = invoke(&window, "cmd_get_settings", json!({})).unwrap();
        assert_eq!(settings["ai"]["enabled"], false);
        assert_eq!(settings["has_api_key"], false);
        assert_eq!(
            invoke(&window, "cmd_set_tokenizer", json!({"tokenizer":"Trigram"})),
            Ok(json!(null))
        );
        assert_eq!(
            invoke(&window, "cmd_get_settings", json!({})).unwrap()["tokenizer"],
            "Trigram"
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_summarize_message",
                json!({"messageId":"missing", "consent":false, "expected":settings["ai"]})
            ),
            Err(json!("InvalidRequest"))
        );
        let error = invoke(
            &window,
            "cmd_save_settings",
            json!({"settings":settings["ai"], "apiKey":{"synthetic-private-value":1}}),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("invalid API key material"));
        assert!(!error.contains("synthetic-private-value"));
        assert_eq!(
            invoke(&window, "cmd_list_accounts", json!({})),
            Ok(json!([]))
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_search_messages",
                json!({"query":{"query":"invoice","mode":"Lexical","limit":20,"offset":0}})
            ),
            Ok(json!([]))
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_get_message_body",
                json!({"messageId":"missing"})
            ),
            Err(json!("NotFound"))
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_triage_action",
                json!({"mutation":{"message_ids":[],"action":"MarkRead"}})
            ),
            Err(json!("InvalidRequest"))
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_fetch_attachment",
                json!({"request":{"account_id":"acct-1","folder":"INBOX","uid":0,"part_id":"2"}})
            ),
            Err(json!("InvalidRequest"))
        );
        let response=invoke(&window,"cmd_add_account",json!({
            "config":{"account_id":"acct-1","email_display":"reader@example.com","imap_host":"imap.example.com","imap_port":993,"auth_method":"AppPassword"},
            "credentials":"synthetic-private-value"
        })).unwrap_err().to_string();
        assert!(response.contains("invalid credential material"));
        assert!(!response.contains("synthetic-private-value"));
        for command in ["cmd_get_account_avatar", "cmd_remove_account_avatar"] {
            assert_eq!(
                invoke(&window, command, json!({"accountId":"missing"})),
                Err(json!("NotFound"))
            );
        }
        let state = window.state::<RuntimeState>();
        let service = state.service.get().unwrap();
        tauri::async_runtime::block_on(service.add_account(
            serde_json::from_value(json!({"account_id":"photo-account","email_display":"reader@example.com","imap_host":"imap.example.com","imap_port":993,"auth_method":"AppPassword"})).unwrap(),
            IncomingCredentials(vault::AccountAuthMaterial::AppPassword { secret: "synthetic-password".into() }),
        )).unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(16, 16)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let photo = invoke(
            &window,
            "cmd_set_account_avatar",
            json!({"accountId":"photo-account","bytes":bytes.into_inner()}),
        )
        .unwrap();
        assert_eq!(photo["width"], 128);
        assert_eq!(photo["mime_type"], "image/png");
        assert_eq!(
            invoke(
                &window,
                "cmd_get_account_avatar",
                json!({"accountId":"photo-account"})
            ),
            Ok(photo)
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_remove_account_avatar",
                json!({"accountId":"photo-account"})
            ),
            Ok(json!(null))
        );
        assert_eq!(
            invoke(
                &window,
                "cmd_get_account_avatar",
                json!({"accountId":"photo-account"})
            ),
            Ok(json!(null))
        );
    }
}

#[tauri::command]
pub async fn cmd_embedding_state(
    state: State<'_, RuntimeState>,
) -> Result<crate::embeddings::EmbeddingState, IpcError> {
    state.initialize(None).await?.embedding_state().await
}
#[tauri::command]
pub async fn cmd_rebuild_embeddings(
    state: State<'_, RuntimeState>,
    consent: bool,
) -> Result<(), IpcError> {
    state
        .initialize(None)
        .await?
        .rebuild_embeddings(consent)
        .await
}
