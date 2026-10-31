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

/// Optional explicit unlock for machines without an OS keyring. Passphrases never leave the vault cipher.
#[tauri::command]
pub async fn cmd_unlock_vault(
    state: State<'_, RuntimeState>,
    passphrase: String,
) -> Result<(), IpcError> {
    state.initialize(Some(Zeroizing::new(passphrase))).await?;
    Ok(())
}

#[tauri::command]
pub async fn cmd_list_accounts(
    state: State<'_, RuntimeState>,
) -> Result<Vec<AccountConfigDto>, IpcError> {
    state.initialize(None).await?.list_accounts().await
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
            cmd_unlock_vault,
            cmd_list_accounts,
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
                cmd_list_accounts,
                cmd_add_account,
                cmd_search_messages,
                cmd_get_message_body,
                cmd_triage_action,
                cmd_fetch_attachment
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
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
    }
}
