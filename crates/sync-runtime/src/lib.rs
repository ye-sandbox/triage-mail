//! Shared local bootstrap and incremental sync without workers or GUI dependencies.
#![forbid(unsafe_code)]

pub mod account;
pub mod bootstrap;
pub mod daemon;
pub mod host;
pub mod ingestion;
pub mod ownership;

pub use account::AccountConnectionConfig;
pub use daemon::{
    run_daemon, DaemonConfig, DaemonEvent, DEFAULT_INTERVAL_SECS, MAX_INTERVAL_SECS,
    MIN_INTERVAL_SECS,
};
pub use host::{
    sync_accounts, AccountSyncOutcome, DefaultSyncTransport, HeadlessSyncError, HeadlessSyncHost,
    SyncRunSummary, SyncTransport,
};

use std::time::Duration;

use async_trait::async_trait;
use mail_engine::{sync::IncrementalSyncResult, DiscoveredFolder};
use storage::FolderSyncState;

/// Matches the desktop's existing per-remote-batch deadline.
pub const REMOTE_BATCH_TIMEOUT: Duration = Duration::from_secs(120);

/// Host failures remain opaque; deadline and invalid remote state share one safe error.
#[derive(Debug, PartialEq, Eq)]
pub enum SyncError<E> {
    Host(E),
    RemoteUnavailable,
}

/// Host-owned transport, storage execution and post-commit effects for one account.
///
/// Implementations bind the account configuration and credentials independently of
/// the coordinator. These methods must not provision installations or start workers.
#[async_trait]
pub trait SyncHost: Sync {
    type Error: Send;
    type Batch: Send + Sync;

    /// The same account must be used by every host operation in this pass.
    fn account_id(&self) -> &str;

    async fn discover_folders(&self) -> Result<Vec<DiscoveredFolder>, Self::Error>;
    async fn save_catalog(&self, folders: &[DiscoveredFolder]) -> Result<(), Self::Error>;
    async fn cached_folders(&self) -> Result<Vec<String>, Self::Error>;
    async fn checkpoint(&self, folder: &str) -> Result<FolderSyncState, Self::Error>;
    fn begin_batch(&self) -> Self::Batch;
    async fn fetch_batch(
        &self,
        previous: &FolderSyncState,
        batch: &Self::Batch,
    ) -> Result<IncrementalSyncResult, Self::Error>;

    /// Establish baselines/frontiers before ingestion, atomically commit records,
    /// flags and checkpoint, then run host hooks and publish buffered progress.
    ///
    /// INVARIANT: The host retains serialization and blocking-storage policy. No
    /// success event may escape before persistence; hook failures retain the host's
    /// existing durability/error semantics. This operation is awaited before the
    /// next checkpoint is read. Dropping the pass does not roll back committed work
    /// or stop an already running host blocking-storage operation.
    async fn commit_batch(
        &self,
        state: FolderSyncState,
        result: IncrementalSyncResult,
        batch: Self::Batch,
    ) -> Result<(), Self::Error>;
}

/// Synchronize selectable discovered folders, or cached folders plus INBOX.
/// No retries, IDLE, automatic provisioning or independent tasks are started here.
pub async fn sync_account_once<H: SyncHost>(host: &H) -> Result<usize, SyncError<H::Error>> {
    let folders = select_folders(host).await?;
    let mut count = 0;
    for folder in folders {
        loop {
            let (batch_count, reset) = sync_batch(host, &folder).await?;
            count += batch_count;
            if batch_count < mail_engine::DEFAULT_SYNC_UID_BATCH_SIZE as usize || reset {
                break;
            }
        }
    }
    Ok(count)
}

async fn select_folders<H: SyncHost>(host: &H) -> Result<Vec<String>, SyncError<H::Error>> {
    let discovered = host.discover_folders().await;
    let mut folders = Vec::new();
    if let Ok(discovered) = discovered {
        // INVARIANT: Catalog persistence is best effort, as in the desktop worker;
        // discovery still determines this pass even when saving its catalog fails.
        let _ = host.save_catalog(&discovered).await;
        folders.extend(
            discovered
                .into_iter()
                .filter(|folder| folder.is_selectable)
                .map(|folder| folder.name),
        );
    }
    if folders.is_empty() {
        folders = host.cached_folders().await.map_err(SyncError::Host)?;
        // INVARIANT: Preserve the existing exact fallback comparison and order.
        if !folders.iter().any(|folder| folder == "INBOX") {
            folders.push("INBOX".into());
        }
    } else if let Some(pos) = folders.iter().position(|f| f.eq_ignore_ascii_case("INBOX")) {
        let inbox = folders.remove(pos);
        folders.insert(0, inbox);
    }
    Ok(folders)
}

async fn sync_batch<H: SyncHost>(
    host: &H,
    folder: &str,
) -> Result<(usize, bool), SyncError<H::Error>> {
    let previous = host.checkpoint(folder).await.map_err(SyncError::Host)?;
    let batch = host.begin_batch();
    let result = tokio::time::timeout(REMOTE_BATCH_TIMEOUT, host.fetch_batch(&previous, &batch))
        .await
        .map_err(|_| SyncError::RemoteUnavailable)?
        .map_err(SyncError::Host)?;
    if result.uidvalidity == 0 {
        return Err(SyncError::RemoteUnavailable);
    }
    let count = result.new_messages.len();
    let reset = result.uidvalidity_reset;
    let state = FolderSyncState {
        account_id: host.account_id().into(),
        folder_name: folder.into(),
        uidvalidity: result.uidvalidity,
        uidnext: if reset {
            1
        } else {
            result.new_last_uid.saturating_add(1)
        },
        highest_modseq: result.highest_modseq,
    };
    host.commit_batch(state, result, batch)
        .await
        .map_err(SyncError::Host)?;
    Ok((count, reset))
}
