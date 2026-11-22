use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use mail_engine::{AttachmentMeta, AuthMethod, UnsubscribeDescriptor};
use serde::{Deserialize, Serialize};
use storage::{
    desktop::LocalMutation, AccountRecord, MailRepository, SearchEngine, SearchMode,
    SqliteMailStore,
};
use triage_ai::{
    DefaultDeterministicTriage, DeterministicTriage, DeterministicTriageResult, EmbeddingProvider,
    TriageCategory,
};
use vault::{AccountAuthMaterial, SecretStore};
use zeroize::{Zeroize, Zeroizing};

use crate::dto::*;
use crate::events::EventBus;
use crate::remote::{RemoteMail, RemoteMutation};

/// Only fixed error codes cross IPC; backend/provider errors may contain credentials or PII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, thiserror::Error)]
pub enum IpcError {
    #[error("invalid request")]
    InvalidRequest,
    #[error("account already exists")]
    AccountExists,
    #[error("record not found")]
    NotFound,
    #[error("vault unavailable")]
    VaultUnavailable,
    #[error("storage unavailable")]
    StorageUnavailable,
    #[error("embedding provider unavailable")]
    EmbeddingUnavailable,
    #[error("remote service unavailable")]
    RemoteUnavailable,
    #[error("tls handshake failed")]
    TlsHandshakeFailed,
    #[error("authentication rejected")]
    AuthenticationRejected,
    #[error("imap disabled")]
    ImapDisabledHint,
    #[error("oauth access denied")]
    OAuthAccessDenied,
    #[error("oauth invalid client")]
    OAuthInvalidClient,
    #[error("oauth cancelled")]
    OAuthCancelled,
    #[error("network timeout")]
    NetworkTimeout,
    #[error("stale message identity")]
    StaleMessage,
    #[error("operation unsupported by server")]
    Unsupported,
    #[error("internal operation failed")]
    Internal,
}

pub(crate) fn diagnostic_to_ipc(code: ConnectionDiagnosticCode) -> IpcError {
    match code {
        ConnectionDiagnosticCode::TlsHandshakeFailed => IpcError::TlsHandshakeFailed,
        ConnectionDiagnosticCode::AuthenticationRejected => IpcError::AuthenticationRejected,
        ConnectionDiagnosticCode::ImapDisabledHint => IpcError::ImapDisabledHint,
        ConnectionDiagnosticCode::OAuthAccessDenied => IpcError::OAuthAccessDenied,
        ConnectionDiagnosticCode::OAuthInvalidClient => IpcError::OAuthInvalidClient,
        ConnectionDiagnosticCode::OAuthCancelled => IpcError::OAuthCancelled,
        ConnectionDiagnosticCode::NetworkTimeout => IpcError::NetworkTimeout,
    }
}

pub(crate) fn ipc_error_to_diagnostic(error: IpcError) -> Option<ConnectionDiagnosticCode> {
    match error {
        IpcError::TlsHandshakeFailed => Some(ConnectionDiagnosticCode::TlsHandshakeFailed),
        IpcError::AuthenticationRejected => Some(ConnectionDiagnosticCode::AuthenticationRejected),
        IpcError::ImapDisabledHint => Some(ConnectionDiagnosticCode::ImapDisabledHint),
        IpcError::OAuthAccessDenied => Some(ConnectionDiagnosticCode::OAuthAccessDenied),
        IpcError::OAuthInvalidClient => Some(ConnectionDiagnosticCode::OAuthInvalidClient),
        IpcError::OAuthCancelled => Some(ConnectionDiagnosticCode::OAuthCancelled),
        IpcError::NetworkTimeout => Some(ConnectionDiagnosticCode::NetworkTimeout),
        _ => None,
    }
}

/// Input-only envelope: never part of a command response, event, or persistent metadata.
pub struct IncomingCredentials(pub AccountAuthMaterial);

impl<'de> Deserialize<'de> for IncomingCredentials {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // INVARIANT: Serde's detailed errors can echo the offending secret string.
        AccountAuthMaterial::deserialize(deserializer)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("invalid credential material"))
    }
}

impl IncomingCredentials {
    /// INVARIANT: only `AppPassword` secrets are normalized; Google displays app passwords in
    /// space-separated groups, but Proton Bridge passwords and OAuth client secrets are opaque.
    pub fn normalize(&mut self) {
        if let AccountAuthMaterial::AppPassword { secret } = &mut self.0 {
            let cleaned: String = secret.chars().filter(|c| !c.is_whitespace()).collect();
            secret.zeroize();
            *secret = cleaned;
        }
    }
}

impl std::fmt::Debug for IncomingCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IncomingCredentials([REDACTED])")
    }
}

impl Drop for IncomingCredentials {
    fn drop(&mut self) {
        match &mut self.0 {
            AccountAuthMaterial::AppPassword { secret } => secret.zeroize(),
            AccountAuthMaterial::ProtonBridgeLocal { bridge_password } => bridge_password.zeroize(),
            AccountAuthMaterial::OAuth2PublicPkce {
                client_id,
                access_token,
                refresh_token,
                ..
            } => {
                client_id.zeroize();
                access_token.zeroize();
                refresh_token.zeroize();
            }
            AccountAuthMaterial::OAuth2ByoCredentials {
                client_id,
                client_secret,
                access_token,
                refresh_token,
                ..
            } => {
                client_id.zeroize();
                client_secret.zeroize();
                access_token.zeroize();
                refresh_token.zeroize();
            }
            AccountAuthMaterial::LocalArchive => {}
        }
    }
}

/// Repository handle preserving production ownership and explicit injected composition.
#[derive(Clone)]
pub(crate) struct ServiceStore(StoreHandle);

#[derive(Clone)]
enum StoreHandle {
    Injected(Arc<SqliteMailStore>),
    Owned(Arc<sync_runtime::bootstrap::OwnedStorage>),
}
impl From<Arc<SqliteMailStore>> for ServiceStore {
    fn from(store: Arc<SqliteMailStore>) -> Self {
        Self(StoreHandle::Injected(store))
    }
}
impl From<Arc<sync_runtime::bootstrap::OwnedStorage>> for ServiceStore {
    fn from(store: Arc<sync_runtime::bootstrap::OwnedStorage>) -> Self {
        Self(StoreHandle::Owned(store))
    }
}
impl std::ops::Deref for ServiceStore {
    type Target = SqliteMailStore;
    fn deref(&self) -> &Self::Target {
        match &self.0 {
            StoreHandle::Injected(store) => store,
            StoreHandle::Owned(store) => store,
        }
    }
}

pub struct DesktopService {
    pub(crate) store: ServiceStore,
    pub(crate) vault: Arc<dyn SecretStore>,
    pub(crate) remote: Arc<dyn RemoteMail>,
    pub(crate) embedding: Option<Arc<dyn EmbeddingProvider>>,
    pub events: Arc<EventBus>,
    pub(crate) learned: Option<Arc<triage_ai::onnx::OnnxEmbeddingProvider>>,
    account_gate: Mutex<()>,
    account_diagnostics: Mutex<HashMap<String, ConnectionDiagnosticCode>>,
    pub(crate) quota_refreshed: Mutex<HashMap<String, std::time::Instant>>,
    pub(crate) settings_gate: Mutex<()>,
    pub(crate) download_gate: tokio::sync::Mutex<()>,
    pub(crate) mutation_gate: tokio::sync::Mutex<()>,
    pub(crate) alert_gate: tokio::sync::Mutex<()>,
    pub(crate) alert_adapter: Arc<dyn crate::alerts::AlertTransport>,
    pub(crate) outbox_gate: tokio::sync::Mutex<()>,
    // INVARIANT: Workers and blocking storage closures retain Arc<DesktopService>.
    // Drop ownership after the store/vault/remote fields and their destructors.
    pub(crate) _ownership: Option<Arc<sync_runtime::ownership::WriterOwnership>>,
}

impl DesktopService {
    pub fn new(
        store: Arc<SqliteMailStore>,
        vault: Arc<dyn SecretStore>,
        remote: Arc<dyn RemoteMail>,
        embedding: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Arc<Self> {
        Self::new_with_store(store, vault, remote, embedding)
    }

    pub(crate) fn new_with_store(
        store: impl Into<ServiceStore>,
        vault: Arc<dyn SecretStore>,
        remote: Arc<dyn RemoteMail>,
        embedding: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Arc<Self> {
        let store = store.into();
        crate::onboarding::clean_orphaned_staging(vault.as_ref());
        let _ = store.reconcile_expired_snoozes(storage::now_epoch_ms());
        Arc::new(Self {
            store,
            vault,
            remote,
            embedding,
            learned: None,
            events: Arc::new(EventBus::default()),
            account_gate: Mutex::new(()),
            account_diagnostics: Mutex::new(HashMap::new()),
            quota_refreshed: Mutex::new(HashMap::new()),
            settings_gate: Mutex::new(()),
            download_gate: tokio::sync::Mutex::new(()),
            mutation_gate: tokio::sync::Mutex::new(()),
            alert_gate: tokio::sync::Mutex::new(()),
            alert_adapter: Arc::new(crate::alerts::NativeAlerts),
            outbox_gate: tokio::sync::Mutex::new(()),
            _ownership: None,
        })
    }

    /// All synchronous SQLCipher/keyring/embedding work runs outside executor threads.
    pub(crate) async fn blocking<T: Send + 'static>(
        self: &Arc<Self>,
        work: impl FnOnce(Arc<Self>) -> Result<T, IpcError> + Send + 'static,
    ) -> Result<T, IpcError> {
        let service = self.clone();
        tokio::task::spawn_blocking(move || work(service))
            .await
            .map_err(|_| IpcError::Internal)?
    }

    pub async fn list_accounts(self: &Arc<Self>) -> Result<Vec<AccountConfigDto>, IpcError> {
        self.blocking(|s| {
            let mut accounts: Vec<AccountConfigDto> = s
                .store
                .list_account_connections()
                .map_err(storage_error)?
                .iter()
                .map(|json| serde_json::from_str(json).map_err(|_| IpcError::StorageUnavailable))
                .collect::<Result<_, _>>()?;
            accounts.extend(
                s.store
                    .disconnected_accounts()
                    .map_err(storage_error)?
                    .into_iter()
                    .map(|(account_id, email_display)| AccountConfigDto {
                        account_id,
                        email_display,
                        imap_host: String::new(),
                        imap_port: 993,
                        auth_method: AuthMethod::AppPassword,
                    }),
            );
            Ok(accounts)
        })
        .await
    }

    pub(crate) async fn account(
        self: &Arc<Self>,
        id: String,
    ) -> Result<AccountConfigDto, IpcError> {
        self.blocking(move |s| {
            let json = s
                .store
                .get_account_connection(&id)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            serde_json::from_str(&json).map_err(|_| IpcError::StorageUnavailable)
        })
        .await
    }

    pub async fn add_account(
        self: &Arc<Self>,
        mut config: AccountConfigDto,
        mut credentials: IncomingCredentials,
    ) -> Result<String, IpcError> {
        if config.account_id.is_empty() {
            config.account_id = opaque_id()?;
        }
        normalize_and_validate_credentials(&config, &mut credentials)?;
        self.blocking(move |s| {
            let _guard = s.account_gate.lock().map_err(|_| IpcError::Internal)?;
            if let Some((id, _)) = s
                .store
                .disconnected_accounts()
                .map_err(storage_error)?
                .into_iter()
                .find(|(_, email)| email == &config.email_display)
            {
                config.account_id = id;
            }
            if s.store
                .get_account_connection(&config.account_id)
                .map_err(storage_error)?
                .is_some()
            {
                return Err(IpcError::AccountExists);
            }
            s.insert_account_with_vault(&config, &credentials.0)?;
            Ok(config.account_id)
        })
        .await
    }

    pub(crate) fn insert_account_with_vault(
        &self,
        config: &AccountConfigDto,
        material: &AccountAuthMaterial,
    ) -> Result<(), IpcError> {
        let (_, key) = config
            .clone()
            .into_endpoint_and_vault_key()
            .map_err(|_| IpcError::InvalidRequest)?;
        let previous = match self.vault.get_secret(&key) {
            Ok(bytes) => Some(Zeroizing::new(bytes)),
            Err(vault::VaultError::NotFound(_)) => None,
            Err(_) => return Err(IpcError::VaultUnavailable),
        };
        let bytes = Zeroizing::new(material.to_bytes().map_err(|_| IpcError::InvalidRequest)?);
        self.vault
            .set_secret(&key, &bytes)
            .map_err(|_| IpcError::VaultUnavailable)?;
        let record = AccountRecord {
            id: config.account_id.clone(),
            email: config.email_display.clone(),
            auth_type: format!("{:?}", config.auth_method),
            created_at_epoch_ms: now_ms(),
            updated_at_epoch_ms: now_ms(),
        };
        let json = serde_json::to_string(config).map_err(|_| IpcError::Internal)?;
        if self
            .store
            .insert_account_connection(&record, &json)
            .is_err()
        {
            // INVARIANT: Compensate a failed metadata commit without losing an older vault entry.
            let restored = match previous {
                Some(bytes) => self.vault.set_secret(&key, &bytes),
                None => self.vault.delete_secret(&key),
            };
            restored.map_err(|_| IpcError::VaultUnavailable)?;
            return Err(IpcError::StorageUnavailable);
        }
        Ok(())
    }

    pub(crate) fn ensure_local_archive_account(
        &self,
        email_display: &str,
    ) -> Result<String, IpcError> {
        let _guard = self.account_gate.lock().map_err(|_| IpcError::Internal)?;
        for json in self
            .store
            .list_account_connections()
            .map_err(storage_error)?
        {
            let existing: AccountConfigDto =
                serde_json::from_str(&json).map_err(|_| IpcError::StorageUnavailable)?;
            if existing.email_display == email_display {
                return if existing.auth_method == AuthMethod::LocalArchive {
                    Ok(existing.account_id)
                } else {
                    Err(IpcError::AccountExists)
                };
            }
        }
        if self
            .store
            .disconnected_accounts()
            .map_err(storage_error)?
            .into_iter()
            .any(|(_, email)| email == email_display)
        {
            return Err(IpcError::AccountExists);
        }
        let config = AccountConfigDto {
            account_id: opaque_id()?,
            email_display: email_display.to_string(),
            imap_host: String::new(),
            imap_port: 0,
            auth_method: AuthMethod::LocalArchive,
        };
        validate_config(&config, &AccountAuthMaterial::LocalArchive)?;
        self.insert_account_with_vault(&config, &AccountAuthMaterial::LocalArchive)?;
        Ok(config.account_id)
    }

    pub(crate) fn record_account_diagnostic(
        &self,
        account_id: &str,
        diagnostic: Option<ConnectionDiagnosticCode>,
    ) {
        if account_id.is_empty() {
            return;
        }
        if let Ok(mut map) = self.account_diagnostics.lock() {
            match diagnostic {
                Some(code) => {
                    map.insert(account_id.to_string(), code);
                }
                None => {
                    map.remove(account_id);
                }
            }
        }
    }

    pub async fn account_diagnostics(
        self: &Arc<Self>,
    ) -> Result<HashMap<String, ConnectionDiagnosticCode>, IpcError> {
        self.account_diagnostics
            .lock()
            .map(|map| map.clone())
            .map_err(|_| IpcError::Internal)
    }

    pub async fn test_account_connection(
        self: &Arc<Self>,
        mut config: AccountConfigDto,
        mut credentials: IncomingCredentials,
    ) -> Result<ConnectionTestResultDto, IpcError> {
        if config.auth_method == AuthMethod::LocalArchive {
            return Err(IpcError::InvalidRequest);
        }
        // INVARIANT: VaultKey rejects empty account IDs, so use a transient placeholder
        // strictly for pre-save endpoint validation without touching vault or SQLite.
        if config.account_id.is_empty() {
            config.account_id = String::from("transient-test");
        }
        normalize_and_validate_credentials(&config, &mut credentials)?;
        match self.remote.test_connection(&config, &credentials).await {
            Ok(()) => Ok(ConnectionTestResultDto {
                ok: true,
                diagnostic: None,
            }),
            Err(err) => match ipc_error_to_diagnostic(err) {
                Some(code) => Ok(ConnectionTestResultDto {
                    ok: false,
                    diagnostic: Some(code),
                }),
                None => Err(err),
            },
        }
    }

    pub(crate) async fn load_oauth_client_material(
        self: &Arc<Self>,
        account_id: String,
    ) -> Result<StoredOAuthClient, IpcError> {
        if account_id.is_empty() {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(move |s| {
            let json = s
                .store
                .get_account_connection(&account_id)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            let config: AccountConfigDto =
                serde_json::from_str(&json).map_err(|_| IpcError::StorageUnavailable)?;
            if !matches!(
                config.auth_method,
                AuthMethod::OAuth2PublicPkce | AuthMethod::OAuth2ByoCredentials
            ) {
                return Err(IpcError::InvalidRequest);
            }
            let key = vault::VaultKey::for_account(&account_id, "imap-auth")
                .map_err(|_| IpcError::InvalidRequest)?;
            let bytes = Zeroizing::new(
                s.vault
                    .get_secret(&key)
                    .map_err(|_| IpcError::VaultUnavailable)?,
            );
            let auth = IncomingCredentials(
                AccountAuthMaterial::from_bytes(&bytes).map_err(|_| IpcError::VaultUnavailable)?,
            );
            let (client_id, client_secret, token_endpoint) = match &auth.0 {
                AccountAuthMaterial::OAuth2PublicPkce {
                    client_id,
                    token_endpoint,
                    ..
                } => (client_id.clone(), None, token_endpoint.clone()),
                AccountAuthMaterial::OAuth2ByoCredentials {
                    client_id,
                    client_secret,
                    token_endpoint,
                    ..
                } => (
                    client_id.clone(),
                    Some(client_secret.clone()),
                    token_endpoint.clone(),
                ),
                _ => return Err(IpcError::InvalidRequest),
            };
            Ok(StoredOAuthClient {
                config,
                client_id,
                client_secret,
                token_endpoint,
            })
        })
        .await
    }

    /// Updates existing vault auth material in place without deleting or altering indexed mail.
    pub async fn update_account_auth(
        self: &Arc<Self>,
        config: AccountConfigDto,
        mut credentials: IncomingCredentials,
    ) -> Result<(), IpcError> {
        if config.account_id.is_empty() {
            return Err(IpcError::InvalidRequest);
        }
        normalize_and_validate_credentials(&config, &mut credentials)?;
        self.blocking(move |s| {
            let _guard = s.account_gate.lock().map_err(|_| IpcError::Internal)?;
            let json = s
                .store
                .get_account_connection(&config.account_id)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            let existing: AccountConfigDto =
                serde_json::from_str(&json).map_err(|_| IpcError::StorageUnavailable)?;
            if existing.auth_method != config.auth_method {
                return Err(IpcError::InvalidRequest);
            }
            let (_, key) = config
                .clone()
                .into_endpoint_and_vault_key()
                .map_err(|_| IpcError::InvalidRequest)?;
            let bytes = Zeroizing::new(
                credentials
                    .0
                    .to_bytes()
                    .map_err(|_| IpcError::InvalidRequest)?,
            );
            s.vault
                .set_secret(&key, &bytes)
                .map_err(|_| IpcError::VaultUnavailable)?;
            s.record_account_diagnostic(&config.account_id, None);
            Ok(())
        })
        .await
    }

    pub async fn list_folders(
        self: &Arc<Self>,
        account_id: String,
    ) -> Result<Vec<FolderDto>, IpcError> {
        self.blocking(move |s| {
            let folders = s.store.list_folders(&account_id).map_err(storage_error)?;
            Ok(folders.into_iter().map(FolderDto::from).collect())
        })
        .await
    }

    pub async fn search_messages(
        self: &Arc<Self>,
        query: SearchQueryDto,
    ) -> Result<Vec<MessageRowDto>, IpcError> {
        if query.limit == 0
            || query.limit > 500
            || query
                .offset
                .checked_add(query.limit)
                .is_none_or(|end| i64::try_from(end).is_err())
        {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(move |s| {
            let embedding = match query.mode {
                SearchMode::Lexical => None,
                _ => Some(s.query_embedding(&query.query)?),
            };
            let scope = query.scope.clone();
            let recipient = query.recipient.clone();
            let query = query
                .into_hybrid_query(embedding)
                .map_err(|_| IpcError::InvalidRequest)?;
            // INVARIANT: Display overrides never replace the canonical IMAP folder/UID.
            match (scope, recipient) {
                (Some(scope), Some(alias)) => {
                    s.store.search_scoped_recipient(&query, &scope, &alias)
                }
                (None, Some(_)) => Err(storage::StorageError::InvalidSearchQuery(
                    "recipient requires an account scope".into(),
                )),
                (Some(scope), None) => s.store.search_scoped(&query, &scope),
                (None, None) => s.store.search(&query),
            }
            .map_err(|error| match error {
                storage::StorageError::InvalidSearchQuery(_) => IpcError::InvalidRequest,
                _ => storage_error(error),
            })
        })
        .await
    }

    pub async fn get_message_body(
        self: &Arc<Self>,
        id: MessageId,
    ) -> Result<MessageDetailDto, IpcError> {
        self.blocking(move |s| {
            let record = s
                .store
                .get_message_by_id(&id)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            let mut triage = if !record.triage_badges.is_empty() || record.category_badge.is_some()
            {
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
                DeterministicTriageResult {
                    category,
                    badges: record.triage_badges.clone(),
                }
            } else {
                DefaultDeterministicTriage.analyze(
                    &record.subject,
                    &record.sender,
                    record.text_body.as_deref().unwrap_or_default(),
                    record.one_click_unsubscribe_supported,
                )
            };
            triage.category = s
                .store
                .category_metadata(&id)
                .map_err(storage_error)?
                .effective;
            let attachments = record
                .attachments
                .into_iter()
                .map(|a| {
                    Ok(AttachmentMeta {
                        part_id: a.part_id,
                        filename: a.filename,
                        mime_type: a.content_type,
                        size_bytes: a
                            .size_bytes
                            .try_into()
                            .map_err(|_| IpcError::StorageUnavailable)?,
                        content_id: a.content_id,
                        is_inline: a.is_inline,
                    })
                })
                .collect::<Result<_, IpcError>>()?;
            Ok(MessageDetailDto {
                id: record.id,
                subject: record.subject,
                sender: record.sender,
                recipients: record.recipients,
                headers: record.raw_headers,
                text_plain: record.text_body,
                html_raw: record.html_body,
                triage_badges: triage.badges.clone(),
                triage_result: triage,
                attachments_meta: attachments,
                is_starred: record.is_starred,
                snooze_until_epoch_ms: record.snooze_until_epoch_ms,
            })
        })
        .await
    }

    pub async fn fetch_attachment(
        self: &Arc<Self>,
        request: AttachmentRequestDto,
    ) -> Result<AttachmentBytesDto, IpcError> {
        let (config, expected_uidvalidity, _) = self
            .validate_attachment_request(&request, MAX_PREVIEW_ATTACHMENT_BYTES)
            .await?;
        let target = crate::remote::RemoteAttachmentTarget {
            folder: &request.folder,
            uid: request.uid,
            part: &request.part_id,
            expected_uidvalidity,
        };
        let bytes = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            self.remote
                .fetch_attachment_capped(&config, target, MAX_PREVIEW_ATTACHMENT_BYTES),
        )
        .await
        .map_err(|_| IpcError::RemoteUnavailable)??;
        Ok(AttachmentBytesDto { bytes })
    }

    /// Validates attachment metadata, UIDVALIDITY, account mode, and 50 MiB cap before
    /// prompting the user with a native save dialog, returning the raw suggested filename.
    pub async fn prepare_attachment_save(
        self: &Arc<Self>,
        request: &AttachmentRequestDto,
    ) -> Result<String, IpcError> {
        let (_, _, filename) = self
            .validate_attachment_request(request, MAX_SAVE_ATTACHMENT_BYTES)
            .await?;
        Ok(filename)
    }

    /// Streams an attachment directly via `BODY.PEEK[part_id]` into a validated non-symlink
    /// local file path without buffering megabyte payloads across IPC or writing to SQLCipher.
    pub async fn save_attachment_to_path(
        self: &Arc<Self>,
        request: AttachmentRequestDto,
        path: std::path::PathBuf,
    ) -> Result<AttachmentSavedDto, IpcError> {
        let (config, expected_uidvalidity, _) = self
            .validate_attachment_request(&request, MAX_SAVE_ATTACHMENT_BYTES)
            .await?;

        let (stage_path, stage_file) = tokio::task::spawn_blocking({
            let path = path.clone();
            move || prepare_attachment_stage_file(&path)
        })
        .await
        .map_err(|_| IpcError::Internal)??;

        let mut writer = std::io::BufWriter::with_capacity(64 * 1024, stage_file);
        let target = crate::remote::RemoteAttachmentTarget {
            folder: &request.folder,
            uid: request.uid,
            part: &request.part_id,
            expected_uidvalidity,
        };
        let stream_result = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            self.remote.save_attachment_to_writer(
                &config,
                target,
                MAX_SAVE_ATTACHMENT_BYTES,
                &mut writer,
            ),
        )
        .await;

        let bytes_written = match stream_result {
            Ok(Ok(written)) => written,
            Ok(Err(err)) => {
                drop(writer);
                let _ = std::fs::remove_file(&stage_path);
                return Err(err);
            }
            Err(_) => {
                drop(writer);
                let _ = std::fs::remove_file(&stage_path);
                return Err(IpcError::RemoteUnavailable);
            }
        };

        tokio::task::spawn_blocking(move || -> Result<(), IpcError> {
            use std::io::Write;
            let result = (|| -> Result<(), IpcError> {
                writer.flush().map_err(|_| IpcError::StorageUnavailable)?;
                let file = writer
                    .into_inner()
                    .map_err(|_| IpcError::StorageUnavailable)?;
                file.sync_all().map_err(|_| IpcError::StorageUnavailable)?;
                drop(file);
                commit_attachment_stage_file(&stage_path, &path)
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(&stage_path);
            }
            result
        })
        .await
        .map_err(|_| IpcError::Internal)??;

        Ok(AttachmentSavedDto { bytes_written })
    }

    async fn validate_attachment_request(
        self: &Arc<Self>,
        request: &AttachmentRequestDto,
        max_bytes: usize,
    ) -> Result<(AccountConfigDto, u32, String), IpcError> {
        if request.uid == 0 || !valid_folder(&request.folder) || !valid_part(&request.part_id) {
            return Err(IpcError::InvalidRequest);
        }
        let check = request.clone();
        let (expected_uidvalidity, filename) = self
            .blocking(move |s| {
                let id = s
                    .store
                    .find_message_id(&check.account_id, &check.folder, check.uid)
                    .map_err(storage_error)?
                    .ok_or(IpcError::NotFound)?;
                let record = s
                    .store
                    .get_message_by_id(&id)
                    .map_err(storage_error)?
                    .ok_or(IpcError::NotFound)?;
                let attachment = record
                    .attachments
                    .iter()
                    .find(|a| a.part_id == check.part_id)
                    .ok_or(IpcError::NotFound)?;
                if attachment.size_bytes < 0 || attachment.size_bytes > max_bytes as i64 {
                    return Err(IpcError::InvalidRequest);
                }
                let state = s
                    .store
                    .get_folder_sync_state(&check.account_id, &check.folder)
                    .map_err(storage_error)?
                    .ok_or(IpcError::StaleMessage)?;
                Ok((state.uidvalidity, attachment.filename.clone()))
            })
            .await?;
        let config = self.account(request.account_id.clone()).await?;
        if config.auth_method == AuthMethod::LocalArchive {
            return Err(IpcError::Unsupported);
        }
        Ok((config, expected_uidvalidity, filename))
    }

    pub async fn triage_action(
        self: &Arc<Self>,
        mutation: TriageMutationDto,
    ) -> Result<(), IpcError> {
        if mutation.message_ids.is_empty()
            || mutation.message_ids.len() > 500
            || mutation.message_ids.iter().collect::<HashSet<_>>().len()
                != mutation.message_ids.len()
        {
            return Err(IpcError::InvalidRequest);
        }
        if let TriageActionDto::Tag(tag) = &mutation.action {
            if tag.trim().is_empty() || tag.len() > 128 || tag.chars().any(char::is_control) {
                return Err(IpcError::InvalidRequest);
            }
        }
        if let TriageActionDto::Snooze { until_epoch_ms } = &mutation.action {
            let now = storage::now_epoch_ms();
            if *until_epoch_ms <= now || *until_epoch_ms > now + 365 * 24 * 3600 * 1000 {
                return Err(IpcError::InvalidRequest);
            }
        }
        if mutation.action == TriageActionDto::OneClickUnsubscribe {
            return self.unsubscribe_messages(mutation.message_ids).await;
        }
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.enqueue_mutation(mutation)).await
    }

    fn enqueue_mutation(&self, mutation: TriageMutationDto) -> Result<(), IpcError> {
        self.enqueue_mutation_guarded(mutation, None)
    }

    pub(crate) fn enqueue_mutation_guarded(
        &self,
        mutation: TriageMutationDto,
        snapshot: Option<(&str, &str)>,
    ) -> Result<(), IpcError> {
        let mut accounts = HashSet::new();
        let mut has_remote_outbox = false;
        let mut changes = Vec::new();
        for id in &mutation.message_ids {
            let message = self
                .store
                .get_message_by_id(id)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            let is_local_only = self
                .store
                .is_local_archive_account(&message.account_id)
                .map_err(storage_error)?;
            accounts.insert(message.account_id.clone());
            let local = match &mutation.action {
                TriageActionDto::MarkRead => LocalMutation::Read(true),
                TriageActionDto::MarkUnread => LocalMutation::Read(false),
                TriageActionDto::Archive => LocalMutation::Folder("Archive"),
                TriageActionDto::Trash => LocalMutation::Folder("Trash"),
                TriageActionDto::Tag(tag) => LocalMutation::Tag(tag),
                TriageActionDto::Star => LocalMutation::Star(true),
                TriageActionDto::Unstar => LocalMutation::Star(false),
                TriageActionDto::Snooze { until_epoch_ms } => {
                    LocalMutation::Snooze(Some(*until_epoch_ms))
                }
                TriageActionDto::Unsnooze => LocalMutation::Snooze(None),
                _ => return Err(IpcError::InvalidRequest),
            };
            let remote = if is_local_only
                || matches!(
                    mutation.action,
                    TriageActionDto::Tag(_)
                        | TriageActionDto::Snooze { .. }
                        | TriageActionDto::Unsnooze
                ) {
                None
            } else {
                let checkpoint = self
                    .store
                    .get_folder_sync_state(&message.account_id, &message.folder)
                    .map_err(storage_error)?
                    .ok_or(IpcError::StaleMessage)?;
                if checkpoint.uidvalidity == 0 {
                    return Err(IpcError::StaleMessage);
                }
                has_remote_outbox = true;
                Some(
                    serde_json::to_string(&RemoteMutation {
                        message_id: id.clone(),
                        account_id: message.account_id,
                        folder: message.folder,
                        uid: message.uid,
                        uidvalidity: checkpoint.uidvalidity,
                        action: mutation.action.clone(),
                    })
                    .map_err(|_| IpcError::Internal)?,
                )
            };
            changes.push((id.clone(), local, remote));
        }
        match snapshot {
            Some((generation, token)) => self
                .store
                .apply_preview_mutations(&changes, generation, token),
            None => self.store.apply_local_mutations(&changes),
        }
        .map_err(storage_error)?;
        for account in accounts {
            self.events.publish(SyncProgressEvent {
                account_id: account,
                state: if !has_remote_outbox
                    || matches!(
                        mutation.action,
                        TriageActionDto::Tag(_)
                            | TriageActionDto::Snooze { .. }
                            | TriageActionDto::Unsnooze
                    ) {
                    SyncState::ResyncRequired
                } else {
                    SyncState::MutationPending
                },
                new_messages_count: 0,
                triage_alerts: Vec::new(),
                embedding_errors_count: 0,
                diagnostic: None,
            });
        }
        Ok(())
    }

    pub async fn reconcile_snoozes(self: &Arc<Self>) -> Result<usize, IpcError> {
        let now = storage::now_epoch_ms();
        let expired_ids = self
            .blocking(move |s| {
                s.store
                    .reconcile_expired_snoozes(now)
                    .map_err(storage_error)
            })
            .await?;
        if !expired_ids.is_empty() {
            let accounts = self.list_accounts().await?;
            for acct in accounts {
                self.events.publish(SyncProgressEvent {
                    account_id: acct.account_id,
                    state: SyncState::ResyncRequired,
                    new_messages_count: 0,
                    triage_alerts: Vec::new(),
                    embedding_errors_count: 0,
                    diagnostic: None,
                });
            }
        }
        Ok(expired_ids.len())
    }

    async fn unsubscribe_messages(self: &Arc<Self>, ids: Vec<String>) -> Result<(), IpcError> {
        let descriptors = self
            .blocking(move |s| {
                ids.iter()
                    .map(|id| {
                        let record = s
                            .store
                            .get_message_by_id(id)
                            .map_err(storage_error)?
                            .ok_or(IpcError::NotFound)?;
                        let parsed = mail_engine::StalwartMimeParser;
                        use mail_engine::MimeExtractor;
                        // Parse stored headers rather than trust a frontend-supplied URL.
                        let message = parsed
                            .parse_lightweight(
                                format!("{}\r\n\r\n", record.raw_headers.trim_end()).as_bytes(),
                            )
                            .map_err(|_| IpcError::InvalidRequest)?;
                        let descriptor: UnsubscribeDescriptor =
                            message.unsubscribe.ok_or(IpcError::InvalidRequest)?;
                        mail_engine::validate_rfc8058_descriptor(&descriptor, false)
                            .map_err(|_| IpcError::InvalidRequest)?;
                        Ok(descriptor)
                    })
                    .collect::<Result<Vec<_>, IpcError>>()
            })
            .await?;
        for descriptor in descriptors {
            self.remote.unsubscribe(&descriptor).await?;
        }
        Ok(())
    }
}

pub(crate) fn storage_error(_: storage::StorageError) -> IpcError {
    IpcError::StorageUnavailable
}
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
pub(crate) fn valid_folder(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
}
fn valid_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .split('.')
            .all(|n| n.parse::<u32>().is_ok_and(|n| n > 0))
}
pub(crate) fn opaque_id() -> Result<String, IpcError> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| IpcError::Internal)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn normalize_and_validate_credentials(
    config: &AccountConfigDto,
    credentials: &mut IncomingCredentials,
) -> Result<(), IpcError> {
    // INVARIANT: validate control characters before whitespace normalization so CRLF/control
    // injection attempts are rejected instead of silently stripped, then re-validate after
    // normalization so whitespace-only AppPasswords are rejected.
    validate_config(config, &credentials.0)?;
    credentials.normalize();
    validate_config(config, &credentials.0)
}

pub(crate) fn validate_config(
    config: &AccountConfigDto,
    material: &AccountAuthMaterial,
) -> Result<(), IpcError> {
    if config.account_id.len() > 128
        || !config
            .account_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || config.email_display.is_empty()
        || config.email_display.chars().any(char::is_control)
    {
        return Err(IpcError::InvalidRequest);
    }
    if config.auth_method == AuthMethod::LocalArchive {
        if !config.imap_host.is_empty() || config.imap_port != 0 {
            return Err(IpcError::InvalidRequest);
        }
    } else if config.imap_host.is_empty()
        || config
            .imap_host
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
        || config.imap_port == 0
    {
        return Err(IpcError::InvalidRequest);
    }
    config
        .clone()
        .into_endpoint_and_vault_key()
        .map_err(|_| IpcError::InvalidRequest)?;
    let valid = match (config.auth_method, material) {
        (AuthMethod::AppPassword, AccountAuthMaterial::AppPassword { secret }) => {
            valid_credential(secret)
        }
        (
            AuthMethod::ProtonBridgeLocal,
            AccountAuthMaterial::ProtonBridgeLocal { bridge_password },
        ) => valid_credential(bridge_password),
        (
            AuthMethod::OAuth2PublicPkce,
            AccountAuthMaterial::OAuth2PublicPkce {
                client_id,
                access_token,
                ..
            },
        ) => valid_credential(client_id) && valid_credential(access_token),
        (
            AuthMethod::OAuth2ByoCredentials,
            AccountAuthMaterial::OAuth2ByoCredentials {
                client_id,
                client_secret,
                access_token,
                ..
            },
        ) => {
            valid_credential(client_id)
                && valid_credential(client_secret)
                && valid_credential(access_token)
        }
        (AuthMethod::LocalArchive, AccountAuthMaterial::LocalArchive) => true,
        _ => false,
    };
    if !valid {
        return Err(IpcError::InvalidRequest);
    }
    Ok(())
}

fn valid_credential(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

pub(crate) struct StoredOAuthClient {
    pub config: AccountConfigDto,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub token_endpoint: Option<String>,
}

impl Drop for StoredOAuthClient {
    fn drop(&mut self) {
        self.client_id.zeroize();
        self.client_secret.zeroize();
    }
}

pub const MAX_PREVIEW_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_SAVE_ATTACHMENT_BYTES: usize = 50 * 1024 * 1024;

fn validate_save_target_path(path: &std::path::Path) -> Result<&std::path::Path, IpcError> {
    crate::backup::local_path(path)?;
    if path.file_name().is_none() {
        return Err(IpcError::InvalidRequest);
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(IpcError::InvalidRequest);
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(IpcError::InvalidRequest),
    }
    path.parent().ok_or(IpcError::InvalidRequest)
}

fn prepare_attachment_stage_file(
    target: &std::path::Path,
) -> Result<(std::path::PathBuf, std::fs::File), IpcError> {
    let parent = validate_save_target_path(target)?;
    let stage_path = parent.join(format!(
        ".triage-att-{}.part",
        vault::backup::random_id().map_err(|_| IpcError::Internal)?
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&stage_path)
        .map_err(|_| IpcError::StorageUnavailable)?;
    Ok((stage_path, file))
}

fn commit_attachment_stage_file(
    stage_path: &std::path::Path,
    target: &std::path::Path,
) -> Result<(), IpcError> {
    let parent = validate_save_target_path(target)?;
    std::fs::rename(stage_path, target).map_err(|_| IpcError::StorageUnavailable)?;
    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod normalize_tests {
    use super::*;

    fn app(secret: &str) -> IncomingCredentials {
        IncomingCredentials(AccountAuthMaterial::AppPassword {
            secret: secret.into(),
        })
    }

    #[test]
    fn app_password_whitespace_is_stripped() {
        let mut c = app(" abcd efgh\u{a0}ijkl\tmnop\r\n");
        c.normalize();
        assert!(
            matches!(&c.0, AccountAuthMaterial::AppPassword { secret } if secret == "abcdefghijklmnop")
        );
    }

    #[test]
    fn all_whitespace_secret_fails_validation() {
        let mut c = app(" \t\u{a0}");
        c.normalize();
        assert!(
            !matches!(&c.0, AccountAuthMaterial::AppPassword { secret } if valid_credential(secret))
        );
    }

    #[test]
    fn bridge_and_oauth_secrets_are_untouched() {
        let mut c = IncomingCredentials(AccountAuthMaterial::ProtonBridgeLocal {
            bridge_password: "a b".into(),
        });
        c.normalize();
        assert!(
            matches!(&c.0, AccountAuthMaterial::ProtonBridgeLocal { bridge_password } if bridge_password == "a b")
        );
        let mut c = IncomingCredentials(AccountAuthMaterial::OAuth2ByoCredentials {
            client_id: "id".into(),
            client_secret: "x y".into(),
            access_token: "t".into(),
            refresh_token: None,
            token_endpoint: Some("https://example.com/token".into()),
        });
        c.normalize();
        assert!(
            matches!(&c.0, AccountAuthMaterial::OAuth2ByoCredentials { client_secret, .. } if client_secret == "x y")
        );
    }
}
