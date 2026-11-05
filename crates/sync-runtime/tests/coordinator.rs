use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use async_trait::async_trait;
use mail_engine::{sync::IncrementalSyncResult, DiscoveredFolder, MimeExtractor};
use storage::FolderSyncState;
use sync_runtime::{sync_account_once, SyncError, SyncHost, REMOTE_BATCH_TIMEOUT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    Discovery,
    Catalog,
    Cached,
    Checkpoint,
    Fetch,
    Commit,
}

#[derive(Default)]
struct State {
    trace: Vec<String>,
    checkpoints: BTreeMap<String, FolderSyncState>,
    fetched: Vec<FolderSyncState>,
    results: VecDeque<IncrementalSyncResult>,
}

struct Host {
    discovered: Vec<DiscoveredFolder>,
    cached: Vec<String>,
    failures: Vec<Failure>,
    pending_fetch_at: Option<usize>,
    dropped: Arc<AtomicUsize>,
    state: Mutex<State>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            discovered: vec![folder("INBOX", true)],
            cached: vec![],
            failures: vec![],
            pending_fetch_at: None,
            dropped: Arc::default(),
            state: Mutex::default(),
        }
    }
}

impl Host {
    fn operation(&self, stage: Failure, label: &str) -> Result<(), Failure> {
        self.state.lock().unwrap().trace.push(label.into());
        if self.failures.contains(&stage) {
            Err(stage)
        } else {
            Ok(())
        }
    }

    fn commits(&self) -> usize {
        self.state.lock().unwrap().checkpoints.len()
    }
}

struct Batch(Arc<AtomicUsize>);
impl Drop for Batch {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[async_trait]
impl SyncHost for Host {
    type Error = Failure;
    type Batch = Batch;

    fn account_id(&self) -> &str {
        "account"
    }

    async fn discover_folders(&self) -> Result<Vec<DiscoveredFolder>, Failure> {
        self.operation(Failure::Discovery, "discover")?;
        Ok(self.discovered.clone())
    }

    async fn save_catalog(&self, _: &[DiscoveredFolder]) -> Result<(), Failure> {
        self.operation(Failure::Catalog, "catalog")
    }

    async fn cached_folders(&self) -> Result<Vec<String>, Failure> {
        self.operation(Failure::Cached, "cached")?;
        Ok(self.cached.clone())
    }

    async fn checkpoint(&self, folder: &str) -> Result<FolderSyncState, Failure> {
        self.operation(Failure::Checkpoint, "checkpoint")?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .checkpoints
            .get(folder)
            .cloned()
            .unwrap_or(FolderSyncState {
                account_id: "account".into(),
                folder_name: folder.into(),
                uidvalidity: 0,
                uidnext: 1,
                highest_modseq: None,
            }))
    }

    fn begin_batch(&self) -> Batch {
        self.state.lock().unwrap().trace.push("begin".into());
        Batch(self.dropped.clone())
    }

    async fn fetch_batch(
        &self,
        previous: &FolderSyncState,
        _: &Batch,
    ) -> Result<IncrementalSyncResult, Failure> {
        self.operation(Failure::Fetch, "fetch")?;
        self.state.lock().unwrap().fetched.push(previous.clone());
        if self.pending_fetch_at == Some(self.state.lock().unwrap().fetched.len()) {
            std::future::pending::<()>().await;
        }
        Ok(self
            .state
            .lock()
            .unwrap()
            .results
            .pop_front()
            .unwrap_or_else(|| {
                let mut result = result(0);
                result.folder.clone_from(&previous.folder_name);
                result.new_last_uid = previous.uidnext.saturating_sub(1);
                result
            }))
    }

    async fn commit_batch(
        &self,
        state: FolderSyncState,
        _: IncrementalSyncResult,
        _: Batch,
    ) -> Result<(), Failure> {
        self.operation(Failure::Commit, "commit")?;
        let mut stored = self.state.lock().unwrap();
        stored.checkpoints.insert(state.folder_name.clone(), state);
        // This adapter models buffered progress after durable checkpoint publication.
        stored.trace.push("progress".into());
        Ok(())
    }
}

fn folder(name: &str, is_selectable: bool) -> DiscoveredFolder {
    DiscoveredFolder {
        name: name.into(),
        display_name: name.into(),
        delimiter: Some("/".into()),
        attributes: vec![],
        role: None,
        is_selectable,
    }
}

fn result(count: usize) -> IncrementalSyncResult {
    let message = mail_engine::StalwartMimeParser
        .parse_lightweight(b"From: sender@example.org\r\nSubject: Synthetic\r\n\r\nBody")
        .unwrap();
    IncrementalSyncResult {
        folder: "INBOX".into(),
        uidvalidity: 7,
        uidvalidity_reset: false,
        server_uidnext: 401,
        new_last_uid: count as u32,
        highest_modseq: Some(9),
        new_messages: (1..=count)
            .map(|uid| {
                let mut message = message.clone();
                message.uid = uid as u32;
                message
            })
            .collect(),
        flag_deltas: vec![],
    }
}

#[tokio::test]
async fn discovery_skips_unselectable_folders_and_prioritizes_inbox_even_if_catalog_fails() {
    let host = Host {
        discovered: vec![
            folder("Archive", true),
            folder("Parent", false),
            folder("inbox", true),
        ],
        failures: vec![Failure::Catalog],
        ..Host::default()
    };
    assert_eq!(sync_account_once(&host).await, Ok(0));
    let state = host.state.lock().unwrap();
    assert_eq!(
        state
            .fetched
            .iter()
            .map(|s| s.folder_name.as_str())
            .collect::<Vec<_>>(),
        ["inbox", "Archive"]
    );
    assert!(!state.trace.contains(&"cached".into()));
}

#[tokio::test]
async fn fallback_preserves_cached_order_and_exact_inbox_comparison() {
    for discovered in [vec![], vec![folder("Parent", false)]] {
        let host = Host {
            discovered,
            cached: vec!["Archive".into(), "inbox".into()],
            ..Host::default()
        };
        assert_eq!(sync_account_once(&host).await, Ok(0));
        let state = host.state.lock().unwrap();
        assert_eq!(
            state
                .fetched
                .iter()
                .map(|s| s.folder_name.as_str())
                .collect::<Vec<_>>(),
            ["Archive", "inbox", "INBOX"]
        );
    }
    let host = Host {
        failures: vec![Failure::Discovery],
        cached: vec!["Archive".into(), "INBOX".into()],
        ..Host::default()
    };
    assert_eq!(sync_account_once(&host).await, Ok(0));
    let state = host.state.lock().unwrap();
    assert_eq!(state.fetched.len(), 2);
    assert!(!state.trace.contains(&"catalog".into()));
}

#[tokio::test]
async fn full_batches_commit_before_the_next_checkpoint_and_short_batches_stop() {
    let host = Host::default();
    let mut tail = result(2);
    tail.new_last_uid = 302;
    host.state.lock().unwrap().results = [result(300), tail].into();
    assert_eq!(sync_account_once(&host).await, Ok(302));
    let state = host.state.lock().unwrap();
    assert_eq!(state.fetched[1].uidnext, 301);
    assert_eq!(state.fetched[1].highest_modseq, Some(9));
    assert_eq!(state.checkpoints["INBOX"].uidnext, 303);
    assert_eq!(
        state.trace,
        [
            "discover",
            "catalog",
            "checkpoint",
            "begin",
            "fetch",
            "commit",
            "progress",
            "checkpoint",
            "begin",
            "fetch",
            "commit",
            "progress"
        ]
    );
    assert_eq!(host.dropped.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn reset_commits_frontier_one_and_ends_the_folder_even_with_a_full_batch() {
    let host = Host::default();
    let mut reset = result(300);
    reset.uidvalidity_reset = true;
    reset.uidvalidity = 10;
    host.state.lock().unwrap().results.push_back(reset);
    assert_eq!(sync_account_once(&host).await, Ok(300));
    let state = host.state.lock().unwrap();
    assert_eq!(state.fetched.len(), 1);
    assert_eq!(state.checkpoints["INBOX"].uidvalidity, 10);
    assert_eq!(state.checkpoints["INBOX"].uidnext, 1);
}

#[tokio::test]
async fn invalid_remote_uidvalidity_never_commits_or_publishes() {
    let host = Host::default();
    let mut invalid = result(1);
    invalid.uidvalidity = 0;
    host.state.lock().unwrap().results.push_back(invalid);
    assert_eq!(
        sync_account_once(&host).await,
        Err(SyncError::RemoteUnavailable)
    );
    assert_eq!(host.commits(), 0);
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn exhausted_uid_frontier_saturates_without_wrapping() {
    let host = Host::default();
    let mut last = result(1);
    last.new_last_uid = u32::MAX;
    host.state.lock().unwrap().results.push_back(last);
    assert_eq!(sync_account_once(&host).await, Ok(1));
    assert_eq!(
        host.state.lock().unwrap().checkpoints["INBOX"].uidnext,
        u32::MAX
    );
}

#[tokio::test]
async fn fatal_host_failures_do_not_publish_or_advance() {
    for failure in [
        Failure::Cached,
        Failure::Checkpoint,
        Failure::Fetch,
        Failure::Commit,
    ] {
        let host = Host {
            discovered: vec![],
            failures: vec![failure],
            ..Host::default()
        };
        assert_eq!(
            sync_account_once(&host).await,
            Err(SyncError::Host(failure))
        );
        assert_eq!(host.commits(), 0);
        assert!(!host
            .state
            .lock()
            .unwrap()
            .trace
            .contains(&"progress".into()));
    }
}

#[tokio::test(start_paused = true)]
async fn remote_deadline_drops_buffered_batch_without_commit_or_retry() {
    let host = Host {
        pending_fetch_at: Some(1),
        ..Host::default()
    };
    let started = tokio::time::Instant::now();
    assert_eq!(
        sync_account_once(&host).await,
        Err(SyncError::RemoteUnavailable)
    );
    assert_eq!(started.elapsed(), REMOTE_BATCH_TIMEOUT);
    assert_eq!(host.commits(), 0);
    assert_eq!(host.state.lock().unwrap().fetched.len(), 1);
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn dropping_a_pass_cancels_pending_remote_work_without_detached_tasks() {
    let host = Host {
        pending_fetch_at: Some(1),
        ..Host::default()
    };
    {
        tokio::select! {
            _ = sync_account_once(&host) => panic!("pending remote completed"),
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
    tokio::time::advance(REMOTE_BATCH_TIMEOUT * 2).await;
    assert_eq!(host.commits(), 0);
    assert_eq!(host.state.lock().unwrap().fetched.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn cancellation_preserves_a_committed_batch_and_resumes_from_its_checkpoint() {
    let mut host = Host {
        pending_fetch_at: Some(2),
        ..Host::default()
    };
    host.state.lock().unwrap().results.push_back(result(300));
    tokio::select! {
        _ = sync_account_once(&host) => panic!("second fetch should remain pending"),
        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
    }
    assert_eq!(host.state.lock().unwrap().checkpoints["INBOX"].uidnext, 301);
    assert_eq!(host.dropped.load(Ordering::SeqCst), 2);
    host.pending_fetch_at = None;
    assert_eq!(sync_account_once(&host).await, Ok(0));
    let state = host.state.lock().unwrap();
    assert_eq!(state.fetched[2].uidnext, 301);
    assert_eq!(state.checkpoints["INBOX"].uidnext, 301);
    assert_eq!(
        state
            .trace
            .iter()
            .filter(|entry| entry.as_str() == "progress")
            .count(),
        2
    );
}
