//! Network operations are injectable so IPC tests never contact external services.
use async_trait::async_trait;
use mail_engine::mutations::{
    execute_imap_mutation, fetch_attachment_checked, ImapMutation, ImapMutationRequest,
    MutationError,
};
use mail_engine::protocol::IdleWaitOutcome;
use mail_engine::provider::{ProviderKind, SpecialFolderRole};
use mail_engine::sync::{execute_incremental_sync, IncrementalSyncQuery, IncrementalSyncResult};
use mail_engine::{
    DefaultImapConnector, EngineError, ImapEndpointConfig, SyncEventSink, UnsubscribeDescriptor,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage::FolderSyncState;
use vault::{AccountAuthMaterial, SecretStore, VaultKey};
use zeroize::Zeroizing;

use crate::{AccountConfigDto, IpcError, TriageActionDto};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteMutation {
    pub message_id: String,
    pub account_id: String,
    pub folder: String,
    pub uid: u32,
    pub uidvalidity: u32,
    pub action: TriageActionDto,
}

pub use mail_engine::mutations::MutationOutcome;

#[async_trait]
pub trait RemoteMail: Send + Sync {
    async fn fetch_attachment(
        &self,
        config: &AccountConfigDto,
        folder: &str,
        uid: u32,
        part: &str,
        expected_uidvalidity: u32,
    ) -> Result<Vec<u8>, IpcError>;
    async fn mutate(
        &self,
        config: &AccountConfigDto,
        mutation: &RemoteMutation,
    ) -> Result<MutationOutcome, IpcError>;
    async fn unsubscribe(&self, descriptor: &UnsubscribeDescriptor) -> Result<(), IpcError>;
    async fn sync(
        &self,
        config: &AccountConfigDto,
        checkpoint: &FolderSyncState,
        sink: &dyn SyncEventSink,
    ) -> Result<IncrementalSyncResult, IpcError>;
    /// Returns true on activity; false when IDLE is unavailable (caller uses adaptive polling).
    async fn wait_for_change(
        &self,
        config: &AccountConfigDto,
        checkpoint: &FolderSyncState,
    ) -> Result<bool, IpcError>;
    async fn discover_folders(
        &self,
        config: &AccountConfigDto,
    ) -> Result<Vec<mail_engine::DiscoveredFolder>, IpcError>;
    async fn test_connection(
        &self,
        _config: &AccountConfigDto,
        _credentials: &crate::service::IncomingCredentials,
    ) -> Result<(), IpcError> {
        Err(IpcError::RemoteUnavailable)
    }
}

pub use mail_engine::NativeSession;

pub struct NativeRemoteMail {
    vault: Arc<dyn SecretStore>,
}

impl NativeRemoteMail {
    pub fn new(vault: Arc<dyn SecretStore>) -> Self {
        Self { vault }
    }

    async fn session(
        &self,
        config: &AccountConfigDto,
    ) -> Result<(NativeSession, mail_engine::provider::ServerCapabilities), IpcError> {
        let endpoint: ImapEndpointConfig = config
            .clone()
            .into_endpoint_and_vault_key()
            .map_err(|_| IpcError::InvalidRequest)?
            .0;
        let vault = self.vault.clone();
        let id = config.account_id.clone();
        let auth = tokio::task::spawn_blocking(move || {
            let key =
                VaultKey::for_account(&id, "imap-auth").map_err(|_| IpcError::VaultUnavailable)?;
            let bytes = Zeroizing::new(
                vault
                    .get_secret(&key)
                    .map_err(|_| IpcError::VaultUnavailable)?,
            );
            AccountAuthMaterial::from_bytes(&bytes)
                .map(crate::service::IncomingCredentials)
                .map_err(|_| IpcError::VaultUnavailable)
        })
        .await
        .map_err(|_| IpcError::Internal)??;
        DefaultImapConnector::new(self.vault.clone())
            .connect_and_authenticate_as(&endpoint, &config.email_display, &auth.0)
            .await
            .map_err(remote_error)
    }
}

#[async_trait]
impl RemoteMail for NativeRemoteMail {
    async fn fetch_attachment(
        &self,
        config: &AccountConfigDto,
        folder: &str,
        uid: u32,
        part: &str,
        expected_uidvalidity: u32,
    ) -> Result<Vec<u8>, IpcError> {
        let (mut session, _) = self.session(config).await?;
        fetch_attachment_checked(&mut session, folder, uid, part, expected_uidvalidity)
            .await
            .map_err(mutation_error)
    }

    async fn mutate(
        &self,
        config: &AccountConfigDto,
        mutation: &RemoteMutation,
    ) -> Result<MutationOutcome, IpcError> {
        let action = match mutation.action {
            TriageActionDto::MarkRead => ImapMutation::Read(true),
            TriageActionDto::MarkUnread => ImapMutation::Read(false),
            TriageActionDto::Archive => ImapMutation::MoveTo(SpecialFolderRole::Archive),
            TriageActionDto::Trash => ImapMutation::MoveTo(SpecialFolderRole::Trash),
            TriageActionDto::Star => ImapMutation::Star(true),
            TriageActionDto::Unstar => ImapMutation::Star(false),
            _ => return Err(IpcError::InvalidRequest),
        };
        let (mut session, caps) = self.session(config).await?;
        let provider = ProviderKind::detect(&config.imap_host, Some(&caps));
        let request = ImapMutationRequest {
            folder: &mutation.folder,
            uid: mutation.uid,
            uidvalidity: mutation.uidvalidity,
            action,
        };
        execute_imap_mutation(&mut session, &caps, provider, &request)
            .await
            .map_err(mutation_error)
    }

    async fn unsubscribe(&self, descriptor: &UnsubscribeDescriptor) -> Result<(), IpcError> {
        mail_engine::Rfc8058UnsubscribeClient::new(Default::default())
            .map_err(remote_error)?
            .execute_one_click(descriptor)
            .await
            .map_err(remote_error)?;
        Ok(())
    }

    async fn sync(
        &self,
        config: &AccountConfigDto,
        checkpoint: &FolderSyncState,
        sink: &dyn SyncEventSink,
    ) -> Result<IncrementalSyncResult, IpcError> {
        let (mut session, caps) = self.session(config).await?;
        let query = IncrementalSyncQuery {
            account_id: &config.account_id,
            folder: &checkpoint.folder_name,
            local_uidvalidity: checkpoint.uidvalidity,
            local_last_uid: checkpoint.uidnext.saturating_sub(1),
            local_last_modseq: checkpoint.highest_modseq,
        };
        execute_incremental_sync(&mut session, query, &caps, sink)
            .await
            .map_err(remote_error)
    }

    async fn wait_for_change(
        &self,
        config: &AccountConfigDto,
        checkpoint: &FolderSyncState,
    ) -> Result<bool, IpcError> {
        let (mut session, caps) = self.session(config).await?;
        if !caps.idle {
            return Ok(false);
        }
        let status = session
            .select_folder(&checkpoint.folder_name)
            .await
            .map_err(remote_error)?;
        // INVARIANT: Recheck watermarks after reconnect so changes between sync and IDLE are not lost.
        if status.uidvalidity != checkpoint.uidvalidity
            || status.uidnext > checkpoint.uidnext
            || status.highest_modseq != checkpoint.highest_modseq
        {
            return Ok(true);
        }
        let tag = session.enter_idle().await.map_err(remote_error)?;
        match session
            .wait_idle_event_or_timeout(std::time::Duration::from_secs(
                mail_engine::DEFAULT_IDLE_HEARTBEAT_SECS,
            ))
            .await
            .map_err(remote_error)?
        {
            IdleWaitOutcome::ServerDisconnect(_) => Err(IpcError::RemoteUnavailable),
            _ => {
                session.exit_idle(&tag).await.map_err(remote_error)?;
                Ok(true)
            }
        }
    }

    async fn discover_folders(
        &self,
        config: &AccountConfigDto,
    ) -> Result<Vec<mail_engine::DiscoveredFolder>, IpcError> {
        let (mut session, _) = self.session(config).await?;
        session.discover_folders().await.map_err(remote_error)
    }

    async fn test_connection(
        &self,
        config: &AccountConfigDto,
        credentials: &crate::service::IncomingCredentials,
    ) -> Result<(), IpcError> {
        let endpoint: ImapEndpointConfig = config
            .clone()
            .into_endpoint_and_vault_key()
            .map_err(|_| IpcError::InvalidRequest)?
            .0;
        DefaultImapConnector::new(self.vault.clone())
            .test_connection_as(&endpoint, &config.email_display, &credentials.0)
            .await
            .map(|_| ())
            .map_err(remote_error)
    }
}

fn remote_error(error: EngineError) -> IpcError {
    error
        .diagnostic_code()
        .map(crate::service::diagnostic_to_ipc)
        .unwrap_or(IpcError::RemoteUnavailable)
}

fn mutation_error(error: MutationError) -> IpcError {
    match error {
        MutationError::StaleIdentity => IpcError::StaleMessage,
        MutationError::Unsupported => IpcError::Unsupported,
        MutationError::InvalidRequest => IpcError::InvalidRequest,
        MutationError::Engine(err) => remote_error(err),
    }
}
