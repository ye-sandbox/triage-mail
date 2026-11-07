//! Headless sync host and execution adapters for pure Rust and CLI environments.
use std::sync::Arc;

use async_trait::async_trait;
use mail_engine::{
    sync::{execute_incremental_sync, IncrementalSyncQuery, IncrementalSyncResult},
    DefaultImapConnector, DiscoveredFolder, EngineError, NativeSession, ServerCapabilities,
    SyncEventSink, SyncProgressEvent,
};
use serde::{Deserialize, Serialize};
use storage::{FolderRecord, FolderSyncState, MailRepository, SqliteMailStore, StorageError};
use vault::SecretStore;

use crate::account::AccountConnectionConfig;
use crate::ingestion::{ingest_batch, IngestError};
use crate::SyncHost;

/// Errors returned during headless sync operations.
#[derive(Debug)]
pub enum HeadlessSyncError {
    Engine(EngineError),
    Storage(StorageError),
    Ingest(IngestError),
    Json(serde_json::Error),
    AccountNotFound(String),
}

impl std::fmt::Display for HeadlessSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(e) => write!(f, "engine error: {e}"),
            Self::Storage(e) => write!(f, "storage error: {e}"),
            Self::Ingest(e) => write!(f, "ingest error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::AccountNotFound(id) => write!(f, "account `{id}` not found"),
        }
    }
}

impl std::error::Error for HeadlessSyncError {}

impl From<EngineError> for HeadlessSyncError {
    fn from(err: EngineError) -> Self {
        Self::Engine(err)
    }
}

impl From<StorageError> for HeadlessSyncError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<IngestError> for HeadlessSyncError {
    fn from(err: IngestError) -> Self {
        Self::Ingest(err)
    }
}

/// Abstract remote IMAP transport enabling hermetic offline testing.
#[async_trait]
pub trait SyncTransport: Send + Sync {
    async fn discover_folders(
        &self,
        config: &AccountConnectionConfig,
    ) -> Result<Vec<DiscoveredFolder>, EngineError>;

    async fn sync_incremental(
        &self,
        config: &AccountConnectionConfig,
        checkpoint: &FolderSyncState,
    ) -> Result<IncrementalSyncResult, EngineError>;
}

/// Default production IMAP transport using `DefaultImapConnector` and TLS 1.3.
pub struct DefaultSyncTransport {
    vault: Arc<dyn SecretStore>,
}

impl DefaultSyncTransport {
    pub fn new(vault: Arc<dyn SecretStore>) -> Self {
        Self { vault }
    }

    async fn session(
        &self,
        config: &AccountConnectionConfig,
    ) -> Result<(NativeSession, ServerCapabilities), EngineError> {
        let (endpoint, _) = config.clone().into_endpoint_and_vault_key()?;
        let auth =
            DefaultImapConnector::new(self.vault.clone()).load_auth_material(&config.account_id)?;
        DefaultImapConnector::new(self.vault.clone())
            .connect_and_authenticate_as(&endpoint, &config.email_display, &auth)
            .await
    }
}

#[async_trait]
impl SyncTransport for DefaultSyncTransport {
    async fn discover_folders(
        &self,
        config: &AccountConnectionConfig,
    ) -> Result<Vec<DiscoveredFolder>, EngineError> {
        let (mut session, _) = self.session(config).await?;
        session.discover_folders().await
    }

    async fn sync_incremental(
        &self,
        config: &AccountConnectionConfig,
        checkpoint: &FolderSyncState,
    ) -> Result<IncrementalSyncResult, EngineError> {
        let (mut session, caps) = self.session(config).await?;
        let query = IncrementalSyncQuery {
            account_id: &config.account_id,
            folder: &checkpoint.folder_name,
            local_uidvalidity: checkpoint.uidvalidity,
            local_last_uid: checkpoint.uidnext.saturating_sub(1),
            local_last_modseq: checkpoint.highest_modseq,
        };
        struct NoopSink;
        impl SyncEventSink for NoopSink {
            fn on_sync_progress(&self, _: &SyncProgressEvent) -> Result<(), EngineError> {
                Ok(())
            }
        }
        execute_incremental_sync(&mut session, query, &caps, &NoopSink).await
    }
}

/// Pure Rust sync host bound to local storage and an account configuration.
pub struct HeadlessSyncHost<'a, T: SyncTransport> {
    pub store: &'a SqliteMailStore,
    pub config: &'a AccountConnectionConfig,
    pub transport: &'a T,
}

impl<'a, T: SyncTransport> HeadlessSyncHost<'a, T> {
    pub fn new(
        store: &'a SqliteMailStore,
        config: &'a AccountConnectionConfig,
        transport: &'a T,
    ) -> Self {
        Self {
            store,
            config,
            transport,
        }
    }
}

#[async_trait]
impl<T: SyncTransport> SyncHost for HeadlessSyncHost<'_, T> {
    type Error = HeadlessSyncError;
    type Batch = ();

    fn account_id(&self) -> &str {
        &self.config.account_id
    }

    async fn discover_folders(&self) -> Result<Vec<DiscoveredFolder>, Self::Error> {
        self.transport
            .discover_folders(self.config)
            .await
            .map_err(HeadlessSyncError::Engine)
    }

    async fn save_catalog(&self, folders: &[DiscoveredFolder]) -> Result<(), Self::Error> {
        let records = folders
            .iter()
            .map(|folder| FolderRecord {
                account_id: self.config.account_id.clone(),
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
                updated_at_epoch_ms: storage::now_epoch_ms(),
            })
            .collect::<Vec<_>>();
        self.store
            .save_folders(&self.config.account_id, &records)
            .map_err(HeadlessSyncError::Storage)
    }

    async fn cached_folders(&self) -> Result<Vec<String>, Self::Error> {
        self.store
            .list_synced_folders(&self.config.account_id)
            .map_err(HeadlessSyncError::Storage)
    }

    async fn checkpoint(&self, folder: &str) -> Result<FolderSyncState, Self::Error> {
        let state = self
            .store
            .get_folder_sync_state(&self.config.account_id, folder)
            .map_err(HeadlessSyncError::Storage)?;
        Ok(state.unwrap_or_else(|| FolderSyncState {
            account_id: self.config.account_id.clone(),
            folder_name: folder.to_string(),
            uidvalidity: 0,
            uidnext: 1,
            highest_modseq: None,
        }))
    }

    fn begin_batch(&self) -> Self::Batch {}

    async fn fetch_batch(
        &self,
        previous: &FolderSyncState,
        _batch: &Self::Batch,
    ) -> Result<IncrementalSyncResult, Self::Error> {
        self.transport
            .sync_incremental(self.config, previous)
            .await
            .map_err(HeadlessSyncError::Engine)
    }

    async fn commit_batch(
        &self,
        state: FolderSyncState,
        result: IncrementalSyncResult,
        _batch: Self::Batch,
    ) -> Result<(), Self::Error> {
        let records = ingest_batch(self.store, &state, result, storage::now_epoch_ms)
            .map_err(HeadlessSyncError::Ingest)?;
        let tz_offset = 0;
        for record in &records {
            let _ = self.store.schedule_reminders_for_message(
                &record.id,
                storage::now_epoch_ms(),
                tz_offset,
            );
        }
        Ok(())
    }
}

/// Outcome of synchronizing a single account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountSyncOutcome {
    pub account_id: String,
    pub synced_messages: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Execution summary of a headless sync operation across one or more accounts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncRunSummary {
    pub accounts: Vec<AccountSyncOutcome>,
    pub total_synced_messages: usize,
    pub duration_ms: u64,
}

/// Synchronizes configured accounts matching the optional filter.
pub async fn sync_accounts<T: SyncTransport>(
    store: &SqliteMailStore,
    transport: &T,
    account_filter: Option<&str>,
) -> Result<SyncRunSummary, HeadlessSyncError> {
    let start = std::time::Instant::now();
    let configs: Vec<AccountConnectionConfig> = match account_filter {
        Some(id) => {
            let json = store
                .get_account_connection(id)
                .map_err(HeadlessSyncError::Storage)?
                .ok_or_else(|| HeadlessSyncError::AccountNotFound(id.to_string()))?;
            vec![serde_json::from_str(&json).map_err(HeadlessSyncError::Json)?]
        }
        None => {
            let jsons = store
                .list_account_connections()
                .map_err(HeadlessSyncError::Storage)?;
            let mut list = Vec::with_capacity(jsons.len());
            for json in jsons {
                list.push(serde_json::from_str(&json).map_err(HeadlessSyncError::Json)?);
            }
            list
        }
    };

    let mut outcomes = Vec::new();
    let mut total_messages = 0;

    for config in &configs {
        let host = HeadlessSyncHost::new(store, config, transport);
        match crate::sync_account_once(&host).await {
            Ok(count) => {
                total_messages += count;
                outcomes.push(AccountSyncOutcome {
                    account_id: config.account_id.clone(),
                    synced_messages: count,
                    error: None,
                });
            }
            Err(err) => {
                let err_msg = match err {
                    crate::SyncError::Host(h) => h.to_string(),
                    crate::SyncError::RemoteUnavailable => "remote unavailable".to_string(),
                };
                outcomes.push(AccountSyncOutcome {
                    account_id: config.account_id.clone(),
                    synced_messages: 0,
                    error: Some(err_msg),
                });
            }
        }
    }

    Ok(SyncRunSummary {
        accounts: outcomes,
        total_synced_messages: total_messages,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
