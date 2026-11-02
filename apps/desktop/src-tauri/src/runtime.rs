//! Tauri-specific command registration and native lifecycle. No core crate depends on this module.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::{ipc::Channel, Manager, State};
use tokio::sync::OnceCell;
use vault::{SecretStore, VaultKey};
use zeroize::Zeroizing;

use crate::dto::*;
use crate::onboarding::{self, AuthorizationSessions, IncomingAuthorization, NativeTokenExchange};
use crate::remote::NativeRemoteMail;
use crate::service::{DesktopService, IncomingCredentials, IpcError};
use crate::worker::WorkerSupervisor;

pub struct RuntimeState {
    data_dir: PathBuf,
    service: OnceCell<Arc<DesktopService>>,
    workers: OnceCell<Arc<WorkerSupervisor>>,
    subscriptions: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    authorizations: AuthorizationSessions,
}

impl RuntimeState {
    fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            service: OnceCell::new(),
            workers: OnceCell::new(),
            subscriptions: Mutex::new(HashMap::new()),
            authorizations: AuthorizationSessions::default(),
        }
    }

    async fn initialize(
        &self,
        passphrase: Option<Zeroizing<String>>,
    ) -> Result<Arc<DesktopService>, IpcError> {
        let service = self
            .service
            .get_or_try_init(|| async {
                let path = self.data_dir.clone();
                tokio::task::spawn_blocking(move || bootstrap(&path, passphrase))
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
}

fn bootstrap(
    path: &Path,
    passphrase: Option<Zeroizing<String>>,
) -> Result<Arc<DesktopService>, IpcError> {
    std::fs::create_dir_all(path).map_err(|_| IpcError::StorageUnavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| IpcError::StorageUnavailable)?;
    }
    let vault: Arc<dyn SecretStore> = match passphrase {
        Some(passphrase) => {
            if passphrase.is_empty() {
                return Err(IpcError::InvalidRequest);
            }
            let cipher = Arc::new(vault::Argon2AesGcmCipher::from_passphrase(
                passphrase.as_bytes(),
            ));
            let fallback =
                vault::EncryptedFallbackStore::file_backed(path.join("vault.enc"), cipher)
                    .map_err(|_| IpcError::VaultUnavailable)?;
            Arc::new(vault::AutoSecretStore::new(Arc::new(fallback)))
        }
        None => Arc::new(vault::KeyringStore::new()),
    };
    let key_id =
        VaultKey::for_account("application", "database-key").map_err(|_| IpcError::Internal)?;
    let db = path.join("mail.db");
    let raw = match vault.get_secret(&key_id) {
        Ok(bytes) => Zeroizing::new(bytes),
        Err(vault::VaultError::NotFound(_)) if !db.exists() => {
            let mut raw = Zeroizing::new(vec![0u8; 32]);
            getrandom::getrandom(&mut raw).map_err(|_| IpcError::Internal)?;
            vault
                .set_secret(&key_id, &raw)
                .map_err(|_| IpcError::VaultUnavailable)?;
            raw
        }
        _ => return Err(IpcError::VaultUnavailable),
    };
    let bytes: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| IpcError::VaultUnavailable)?;
    let store = storage::SqliteMailStore::open(
        db,
        &storage::DatabaseKey::from_bytes(bytes),
        Default::default(),
    )
    .map_err(|_| IpcError::StorageUnavailable)?;
    let remote = Arc::new(NativeRemoteMail::new(vault.clone()));
    let embedding = triage_ai::EmbeddingModelConfig::from_xdg_env(
        triage_ai::embeddings::MODEL_ALL_MINILM_L6_V2,
    )
    .ok()
    .map(triage_ai::embeddings::LocalCpuEmbeddingProvider::new)
    .map(|p| Arc::new(p) as Arc<dyn triage_ai::EmbeddingProvider>);
    Ok(DesktopService::new(
        Arc::new(store),
        vault,
        remote,
        embedding,
    ))
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

pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            app.manage(RuntimeState::new(data));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_list_alert_destinations,
            cmd_save_alert_destination,
            cmd_delete_alert_destination,
            cmd_alert_history,
            cmd_test_alert,
            cmd_replay_alert,
            cmd_local_alert_available,
            cmd_list_saved_views,
            cmd_save_saved_view,
            cmd_delete_saved_view,
            cmd_search_saved_view,
            cmd_list_rules,
            cmd_save_rule,
            cmd_delete_rule,
            cmd_reorder_rules,
            cmd_simulate_rules,
            cmd_rule_draft,
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
    use serde_json::{json, Value};
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

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
        let state = RuntimeState::new(PathBuf::new());
        assert!(state.service.set(service.clone()).is_ok());
        // No worker starts in this fixture; all network operations remain unreachable.
        assert!(state.workers.set(WorkerSupervisor::new(service)).is_ok());
        let app = mock_builder()
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                cmd_list_alert_destinations,
                cmd_save_alert_destination,
                cmd_delete_alert_destination,
                cmd_alert_history,
                cmd_test_alert,
                cmd_replay_alert,
                cmd_local_alert_available,
                cmd_list_saved_views,
                cmd_save_saved_view,
                cmd_delete_saved_view,
                cmd_search_saved_view,
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
