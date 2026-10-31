use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use mail_engine::{AttachmentMeta, AuthMethod, UnsubscribeDescriptor};
use serde::{Deserialize, Serialize};
use storage::{
    desktop::LocalMutation, AccountRecord, MailRepository, SearchEngine, SearchMode,
    SqliteMailStore,
};
use triage_ai::{DefaultDeterministicTriage, DeterministicTriage, EmbeddingProvider};
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
    #[error("stale message identity")]
    StaleMessage,
    #[error("operation unsupported by server")]
    Unsupported,
    #[error("internal operation failed")]
    Internal,
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
            } => {
                client_id.zeroize();
                client_secret.zeroize();
                access_token.zeroize();
                refresh_token.zeroize();
            }
        }
    }
}

pub struct DesktopService {
    pub(crate) store: Arc<SqliteMailStore>,
    pub(crate) vault: Arc<dyn SecretStore>,
    pub(crate) remote: Arc<dyn RemoteMail>,
    pub(crate) embedding: Option<Arc<dyn EmbeddingProvider>>,
    pub events: Arc<EventBus>,
    account_gate: Mutex<()>,
    pub(crate) mutation_gate: tokio::sync::Mutex<()>,
    pub(crate) outbox_gate: tokio::sync::Mutex<()>,
}

impl DesktopService {
    pub fn new(
        store: Arc<SqliteMailStore>,
        vault: Arc<dyn SecretStore>,
        remote: Arc<dyn RemoteMail>,
        embedding: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store,
            vault,
            remote,
            embedding,
            events: Arc::new(EventBus::default()),
            account_gate: Mutex::new(()),
            mutation_gate: tokio::sync::Mutex::new(()),
            outbox_gate: tokio::sync::Mutex::new(()),
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
            s.store
                .list_account_connections()
                .map_err(storage_error)?
                .iter()
                .map(|json| serde_json::from_str(json).map_err(|_| IpcError::StorageUnavailable))
                .collect()
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
        credentials: IncomingCredentials,
    ) -> Result<String, IpcError> {
        if config.account_id.is_empty() {
            config.account_id = opaque_id()?;
        }
        validate_config(&config, &credentials.0)?;
        self.blocking(move |s| {
            let _guard = s.account_gate.lock().map_err(|_| IpcError::Internal)?;
            if s.store
                .get_account_connection(&config.account_id)
                .map_err(storage_error)?
                .is_some()
            {
                return Err(IpcError::AccountExists);
            }
            let (_, key) = config
                .clone()
                .into_endpoint_and_vault_key()
                .map_err(|_| IpcError::InvalidRequest)?;
            let previous = match s.vault.get_secret(&key) {
                Ok(bytes) => Some(Zeroizing::new(bytes)),
                Err(vault::VaultError::NotFound(_)) => None,
                Err(_) => return Err(IpcError::VaultUnavailable),
            };
            let bytes = Zeroizing::new(
                credentials
                    .0
                    .to_bytes()
                    .map_err(|_| IpcError::InvalidRequest)?,
            );
            s.vault
                .set_secret(&key, &bytes)
                .map_err(|_| IpcError::VaultUnavailable)?;
            let record = AccountRecord {
                id: config.account_id.clone(),
                email: config.email_display.clone(),
                auth_type: format!("{:?}", config.auth_method),
                created_at_epoch_ms: now_ms(),
                updated_at_epoch_ms: now_ms(),
            };
            let json = serde_json::to_string(&config).map_err(|_| IpcError::Internal)?;
            if s.store.insert_account_connection(&record, &json).is_err() {
                // INVARIANT: Compensate a failed metadata commit without losing an older vault entry.
                let restored = match previous {
                    Some(bytes) => s.vault.set_secret(&key, &bytes),
                    None => s.vault.delete_secret(&key),
                };
                restored.map_err(|_| IpcError::VaultUnavailable)?;
                return Err(IpcError::StorageUnavailable);
            }
            Ok(config.account_id)
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
                _ => Some(
                    s.embedding
                        .as_ref()
                        .ok_or(IpcError::EmbeddingUnavailable)?
                        .embed_int8(&query.query)
                        .map_err(|_| IpcError::EmbeddingUnavailable)?,
                ),
            };
            let query = query
                .into_hybrid_query(embedding)
                .map_err(|_| IpcError::InvalidRequest)?;
            let mut rows = s.store.search(&query).map_err(storage_error)?;
            for row in &mut rows {
                if let Some(folder) = s.store.folder_override(&row.id).map_err(storage_error)? {
                    row.folder = folder;
                }
            }
            Ok(rows)
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
            let triage = DefaultDeterministicTriage.analyze(
                &record.subject,
                &record.sender,
                record.text_body.as_deref().unwrap_or_default(),
                record.one_click_unsubscribe_supported,
            );
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
                headers: record.raw_headers,
                text_plain: record.text_body,
                html_raw: record.html_body,
                triage_badges: triage.badges.clone(),
                triage_result: triage,
                attachments_meta: attachments,
            })
        })
        .await
    }

    pub async fn fetch_attachment(
        self: &Arc<Self>,
        request: AttachmentRequestDto,
    ) -> Result<AttachmentBytesDto, IpcError> {
        if request.uid == 0 || !valid_folder(&request.folder) || !valid_part(&request.part_id) {
            return Err(IpcError::InvalidRequest);
        }
        let check = request.clone();
        let expected_uidvalidity = self
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
                if !record
                    .attachments
                    .iter()
                    .any(|a| a.part_id == check.part_id)
                {
                    return Err(IpcError::NotFound);
                }
                let state = s
                    .store
                    .get_folder_sync_state(&check.account_id, &check.folder)
                    .map_err(storage_error)?
                    .ok_or(IpcError::StaleMessage)?;
                Ok(state.uidvalidity)
            })
            .await?;
        let config = self.account(request.account_id).await?;
        let bytes = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            self.remote.fetch_attachment(
                &config,
                &request.folder,
                request.uid,
                &request.part_id,
                expected_uidvalidity,
            ),
        )
        .await
        .map_err(|_| IpcError::RemoteUnavailable)??;
        Ok(AttachmentBytesDto { bytes })
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
        if mutation.action == TriageActionDto::OneClickUnsubscribe {
            return self.unsubscribe_messages(mutation.message_ids).await;
        }
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.enqueue_mutation(mutation)).await
    }

    fn enqueue_mutation(&self, mutation: TriageMutationDto) -> Result<(), IpcError> {
        let mut accounts = HashSet::new();
        let mut changes = Vec::new();
        for id in &mutation.message_ids {
            let message = self
                .store
                .get_message_by_id(id)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            accounts.insert(message.account_id.clone());
            let local = match &mutation.action {
                TriageActionDto::MarkRead => LocalMutation::Read(true),
                TriageActionDto::MarkUnread => LocalMutation::Read(false),
                TriageActionDto::Archive => LocalMutation::Folder("Archive"),
                TriageActionDto::Trash => LocalMutation::Folder("Trash"),
                TriageActionDto::Tag(tag) => LocalMutation::Tag(tag),
                _ => return Err(IpcError::InvalidRequest),
            };
            let remote = if matches!(mutation.action, TriageActionDto::Tag(_)) {
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
        self.store
            .apply_local_mutations(&changes)
            .map_err(storage_error)?;
        for account in accounts {
            self.events.publish(SyncProgressEvent {
                account_id: account,
                state: if matches!(mutation.action, TriageActionDto::Tag(_)) {
                    SyncState::ResyncRequired
                } else {
                    SyncState::MutationPending
                },
                new_messages_count: 0,
                triage_alerts: Vec::new(),
            });
        }
        Ok(())
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
        || config.imap_host.is_empty()
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
