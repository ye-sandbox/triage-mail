// INVARIANT: Refresh IMAP IDLE (`DONE` -> `IDLE`) every 14 minutes (840s) instead of the
// theoretical 29-minute RFC 2177 limit to prevent silent NAT and Outlook firewall drops (k-002).
// INVARIANT: Gracefully fall back to adaptive exponential backoff polling when a server omits
// the `IDLE` capability or when transient network disconnects occur.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::watch;

use crate::protocol::{IdleNotification, IdleWaitOutcome, ImapSession};
use crate::provider::ServerCapabilities;
use crate::sync::{execute_incremental_sync, IncrementalSyncQuery};
use crate::{EngineError, ImapConnector, ImapEndpointConfig, SyncEventSink};

/// Default RFC 2177 `IDLE` heartbeat renewal interval (14 minutes = 840 seconds).
pub const DEFAULT_IDLE_HEARTBEAT_SECS: u64 = 14 * 60;

/// Default base polling interval when `IDLE` is unsupported (60 seconds).
pub const DEFAULT_BASE_POLL_SECS: u64 = 60;

/// Default maximum backoff polling interval (15 minutes = 900 seconds).
pub const DEFAULT_MAX_POLL_SECS: u64 = 15 * 60;

/// Configuration for the IMAP `IDLE` keep-alive manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleConfig {
    pub account_id: String,
    pub folder: String,
    /// Renewal window before sending `DONE` -> `IDLE` (defaults to 14 minutes).
    pub heartbeat_interval: Duration,
    pub poll_config: AdaptivePollConfig,
}

impl IdleConfig {
    /// Creates a standard `IdleConfig` with a 14-minute heartbeat for the given account and folder.
    pub fn new(account_id: impl Into<String>, folder: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            folder: folder.into(),
            heartbeat_interval: Duration::from_secs(DEFAULT_IDLE_HEARTBEAT_SECS),
            poll_config: AdaptivePollConfig::default(),
        }
    }
}

/// Configuration for adaptive polling fallback when `IDLE` is unavailable or dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptivePollConfig {
    pub base_interval: Duration,
    pub max_interval: Duration,
    pub empty_poll_step_factor: u32,
}

impl Default for AdaptivePollConfig {
    fn default() -> Self {
        Self {
            base_interval: Duration::from_secs(DEFAULT_BASE_POLL_SECS),
            max_interval: Duration::from_secs(DEFAULT_MAX_POLL_SECS),
            empty_poll_step_factor: 2,
        }
    }
}

/// Stateful adaptive backoff calculator for folder polling and error recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptivePollSchedule {
    config: AdaptivePollConfig,
    consecutive_empty_polls: u32,
    consecutive_errors: u32,
    current_interval: Duration,
}

impl AdaptivePollSchedule {
    pub fn new(config: AdaptivePollConfig) -> Self {
        Self {
            current_interval: config.base_interval,
            config,
            consecutive_empty_polls: 0,
            consecutive_errors: 0,
        }
    }

    /// Returns the currently scheduled wait duration before the next poll.
    pub fn current_interval(&self) -> Duration {
        self.current_interval
    }

    /// Returns consecutive error count.
    pub fn consecutive_errors(&self) -> u32 {
        self.consecutive_errors
    }

    /// Resets the polling schedule to `base_interval` when new messages or flag changes arrive.
    pub fn record_activity(&mut self) -> Duration {
        self.consecutive_empty_polls = 0;
        self.consecutive_errors = 0;
        self.current_interval = self.config.base_interval;
        self.current_interval
    }

    /// Advances the polling interval after an idle poll with zero changes, capped at `max_interval`.
    pub fn record_empty_poll(&mut self) -> Duration {
        self.consecutive_errors = 0;
        self.consecutive_empty_polls = self.consecutive_empty_polls.saturating_add(1);

        // Step up every 2 consecutive empty polls using exponential scaling up to max_interval
        let exponent = (self.consecutive_empty_polls / 2).min(6);
        let multiplier = self
            .config
            .empty_poll_step_factor
            .saturating_pow(exponent)
            .max(1);
        let scaled = self.config.base_interval.saturating_mul(multiplier);
        self.current_interval = scaled.min(self.config.max_interval);
        self.current_interval
    }

    /// Applies exponential backoff after a transport or protocol error, capped at `max_interval`.
    pub fn record_error(&mut self) -> Duration {
        self.consecutive_errors = self.consecutive_errors.saturating_add(1);
        let shift = self.consecutive_errors.min(8);
        let multiplier = 1u32 << shift;
        let scaled = self.config.base_interval.saturating_mul(multiplier);
        self.current_interval = scaled.min(self.config.max_interval);
        self.current_interval
    }
}

/// Result of a single `IdleManager` supervision cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleCycleOutcome {
    /// Server pushed a real-time `IDLE` notification and incremental sync completed.
    SyncedOnEvent {
        notification: IdleNotification,
        synced_count: usize,
        new_last_uid: u32,
        highest_modseq: Option<u64>,
    },
    /// The 14-minute heartbeat window elapsed; `DONE` was sent cleanly to keep NAT alive.
    HeartbeatRenewed,
    /// Server does not advertise `IDLE`; executed a poll pass and computed next adaptive delay.
    PolledFallback {
        synced_count: usize,
        next_delay: Duration,
        new_last_uid: u32,
        highest_modseq: Option<u64>,
    },
    /// Server closed the connection (`* BYE`).
    Disconnected(String),
}

/// Manages real-time IMAP `IDLE` loops with 14-minute NAT heartbeats and adaptive polling fallback.
pub struct IdleManager {
    config: IdleConfig,
    schedule: AdaptivePollSchedule,
    local_uidvalidity: u32,
    local_last_uid: u32,
    local_last_modseq: Option<u64>,
}

impl IdleManager {
    /// Creates a new `IdleManager` initialized with local folder watermarks.
    pub fn new(
        config: IdleConfig,
        local_uidvalidity: u32,
        local_last_uid: u32,
        local_last_modseq: Option<u64>,
    ) -> Self {
        let schedule = AdaptivePollSchedule::new(config.poll_config);
        Self {
            config,
            schedule,
            local_uidvalidity,
            local_last_uid,
            local_last_modseq,
        }
    }

    /// Returns a reference to the current adaptive polling schedule.
    pub fn schedule(&self) -> &AdaptivePollSchedule {
        &self.schedule
    }

    /// Returns the latest synchronized UID watermark.
    pub fn local_last_uid(&self) -> u32 {
        self.local_last_uid
    }

    /// Returns the latest synchronized `HIGHESTMODSEQ` watermark.
    pub fn local_last_modseq(&self) -> Option<u64> {
        self.local_last_modseq
    }

    /// Executes a single supervision cycle over an active `ImapSession`:
    /// - If `capabilities.idle` is true: selects the folder, enters `IDLE`, waits for either an
    ///   untagged event or the 14-minute heartbeat timeout, sends `DONE`, and syncs incrementally on events.
    /// - If `capabilities.idle` is false: runs an incremental sync pass and updates the adaptive poll timer.
    pub async fn run_session_cycle<S: AsyncRead + AsyncWrite + Unpin + Send>(
        &mut self,
        session: &mut ImapSession<S>,
        capabilities: &ServerCapabilities,
        sink: &dyn SyncEventSink,
    ) -> Result<IdleCycleOutcome, EngineError> {
        if capabilities.idle {
            let _status = session.select_folder(&self.config.folder).await?;
            let idle_tag = session.enter_idle().await?;
            let wait_outcome = session
                .wait_idle_event_or_timeout(self.config.heartbeat_interval)
                .await?;

            match wait_outcome {
                IdleWaitOutcome::HeartbeatTimeout => {
                    // INVARIANT: Send DONE before NAT/firewall drops the idle TCP socket (14m window)
                    session.exit_idle(&idle_tag).await?;
                    Ok(IdleCycleOutcome::HeartbeatRenewed)
                }
                IdleWaitOutcome::ServerEvent(notification) => {
                    session.exit_idle(&idle_tag).await?;
                    let sync_res = self
                        .sync_now_on_session(session, capabilities, sink)
                        .await?;
                    self.schedule.record_activity();
                    Ok(IdleCycleOutcome::SyncedOnEvent {
                        notification,
                        synced_count: sync_res.new_messages.len(),
                        new_last_uid: self.local_last_uid,
                        highest_modseq: self.local_last_modseq,
                    })
                }
                IdleWaitOutcome::ServerDisconnect(reason) => {
                    self.schedule.record_error();
                    Ok(IdleCycleOutcome::Disconnected(reason))
                }
            }
        } else {
            let sync_res = self
                .sync_now_on_session(session, capabilities, sink)
                .await?;
            let synced_count = sync_res.new_messages.len();
            let next_delay = if synced_count > 0 || !sync_res.flag_deltas.is_empty() {
                self.schedule.record_activity()
            } else {
                self.schedule.record_empty_poll()
            };

            Ok(IdleCycleOutcome::PolledFallback {
                synced_count,
                next_delay,
                new_last_uid: self.local_last_uid,
                highest_modseq: self.local_last_modseq,
            })
        }
    }

    /// Spawns a background `tokio` worker task that polls/syncs via an `ImapConnector` with
    /// adaptive exponential backoff until `shutdown_rx` signals termination.
    pub fn spawn_polling_worker(
        mut self,
        connector: Arc<dyn ImapConnector>,
        endpoint: ImapEndpointConfig,
        sink: Arc<dyn SyncEventSink>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<Result<(), EngineError>> {
        tokio::spawn(async move {
            loop {
                if *shutdown_rx.borrow() {
                    return Ok(());
                }

                let sync_attempt = connector
                    .sync_incremental(
                        &endpoint,
                        &self.config.folder,
                        self.local_last_uid,
                        self.local_last_modseq,
                        sink.as_ref(),
                    )
                    .await;

                let wait_duration = match sync_attempt {
                    Ok(messages) => {
                        for msg in &messages {
                            if msg.uid > self.local_last_uid {
                                self.local_last_uid = msg.uid;
                            }
                            if let Some(m) = msg.modseq {
                                self.local_last_modseq =
                                    Some(self.local_last_modseq.unwrap_or(0).max(m));
                            }
                        }
                        if !messages.is_empty() {
                            self.schedule.record_activity()
                        } else {
                            self.schedule.record_empty_poll()
                        }
                    }
                    Err(_err) => self.schedule.record_error(),
                };

                tokio::select! {
                    _ = tokio::time::sleep(wait_duration) => {}
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            return Ok(());
                        }
                    }
                }
            }
        })
    }

    async fn sync_now_on_session<S: AsyncRead + AsyncWrite + Unpin + Send>(
        &mut self,
        session: &mut ImapSession<S>,
        capabilities: &ServerCapabilities,
        sink: &dyn SyncEventSink,
    ) -> Result<crate::sync::IncrementalSyncResult, EngineError> {
        let query = IncrementalSyncQuery {
            account_id: &self.config.account_id,
            folder: &self.config.folder,
            local_uidvalidity: self.local_uidvalidity,
            local_last_uid: self.local_last_uid,
            local_last_modseq: self.local_last_modseq,
        };

        let res = execute_incremental_sync(session, query, capabilities, sink).await?;
        if res.uidvalidity_reset {
            self.local_uidvalidity = res.uidvalidity;
            self.local_last_uid = 0;
            self.local_last_modseq = res.highest_modseq;
        } else {
            self.local_uidvalidity = res.uidvalidity;
            if res.new_last_uid > self.local_last_uid {
                self.local_last_uid = res.new_last_uid;
            }
            if res.highest_modseq.is_some() {
                self.local_last_modseq = res.highest_modseq;
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

    use crate::{AuthMethod, ParsedInboundMessage, SyncProgressEvent};

    struct MockSink {
        events: AtomicUsize,
        synced_messages: AtomicUsize,
    }

    impl MockSink {
        fn new() -> Self {
            Self {
                events: AtomicUsize::new(0),
                synced_messages: AtomicUsize::new(0),
            }
        }
    }

    impl SyncEventSink for MockSink {
        fn on_sync_progress(&self, event: &SyncProgressEvent) -> Result<(), EngineError> {
            self.events.fetch_add(1, Ordering::SeqCst);
            self.synced_messages
                .fetch_add(event.synced_count, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn default_idle_config_enforces_14_minute_heartbeat() {
        let cfg = IdleConfig::new("acct-1", "INBOX");
        assert_eq!(cfg.heartbeat_interval, Duration::from_secs(14 * 60));
    }

    #[test]
    fn adaptive_poll_schedule_backs_off_and_resets_on_activity() {
        let poll_cfg = AdaptivePollConfig {
            base_interval: Duration::from_secs(30),
            max_interval: Duration::from_secs(600),
            empty_poll_step_factor: 2,
        };
        let mut schedule = AdaptivePollSchedule::new(poll_cfg);
        assert_eq!(schedule.current_interval(), Duration::from_secs(30));

        // First empty poll stays at base (exponent = 1 / 2 = 0)
        assert_eq!(schedule.record_empty_poll(), Duration::from_secs(30));
        // Second empty poll doubles (exponent = 2 / 2 = 1 -> 60s)
        assert_eq!(schedule.record_empty_poll(), Duration::from_secs(60));
        // Fourth empty poll quadruples (120s)
        let _ = schedule.record_empty_poll();
        assert_eq!(schedule.record_empty_poll(), Duration::from_secs(120));

        // Activity immediately resets to 30s
        assert_eq!(schedule.record_activity(), Duration::from_secs(30));

        // Errors apply exponential backoff: 60s, 120s, 240s, 480s, capped at 600s
        assert_eq!(schedule.record_error(), Duration::from_secs(60));
        assert_eq!(schedule.record_error(), Duration::from_secs(120));
        assert_eq!(schedule.record_error(), Duration::from_secs(240));
        assert_eq!(schedule.record_error(), Duration::from_secs(480));
        assert_eq!(schedule.record_error(), Duration::from_secs(600));
        assert_eq!(schedule.consecutive_errors(), 5);
    }

    #[tokio::test]
    async fn idle_manager_renews_heartbeat_on_timeout_via_done() {
        let (client_stream, mut server_stream) = duplex(4096);
        let mut session = ImapSession::new(client_stream);
        let sink = MockSink::new();

        let mut cfg = IdleConfig::new("acct-heartbeat", "INBOX");
        cfg.heartbeat_interval = Duration::from_millis(35);

        let mut manager = IdleManager::new(cfg, 100, 10, Some(50));
        let caps = ServerCapabilities::from_tokens(["IMAP4rev1", "IDLE", "CONDSTORE"]);

        let server_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            // 1. Expect SELECT "INBOX"
            let n = server_stream.read(&mut buf).await.unwrap();
            let select_cmd = String::from_utf8_lossy(&buf[..n]);
            assert!(select_cmd.contains("SELECT \"INBOX\""));
            server_stream
                .write_all(
                    b"* 10 EXISTS\r\n* OK [UIDVALIDITY 100]\r\nA0001 OK SELECT completed\r\n",
                )
                .await
                .unwrap();

            // 2. Expect A0002 IDLE
            let n = server_stream.read(&mut buf).await.unwrap();
            let idle_cmd = String::from_utf8_lossy(&buf[..n]);
            assert!(idle_cmd.contains("A0002 IDLE"));
            server_stream.write_all(b"+ idling\r\n").await.unwrap();

            // 3. Do not send anything; client heartbeat timer (35ms) should expire and send DONE
            let n = server_stream.read(&mut buf).await.unwrap();
            let done_cmd = String::from_utf8_lossy(&buf[..n]);
            assert_eq!(done_cmd, "DONE\r\n");

            server_stream
                .write_all(b"A0002 OK IDLE terminated\r\n")
                .await
                .unwrap();
        });

        let outcome = manager
            .run_session_cycle(&mut session, &caps, &sink)
            .await
            .unwrap();
        assert_eq!(outcome, IdleCycleOutcome::HeartbeatRenewed);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn idle_manager_wakes_on_exists_and_triggers_incremental_sync() {
        let (client_stream, mut server_stream) = duplex(8192);
        let mut session = ImapSession::new(client_stream);
        let sink = MockSink::new();

        let cfg = IdleConfig::new("acct-live", "INBOX");
        let mut manager = IdleManager::new(cfg, 100, 10, Some(50));
        let caps = ServerCapabilities::from_tokens(["IMAP4rev1", "IDLE", "CONDSTORE"]);

        let server_task = tokio::spawn(async move {
            let mut buf = [0u8; 1024];

            // 1. Initial SELECT before IDLE
            let _ = server_stream.read(&mut buf).await.unwrap();
            server_stream
                .write_all(
                    b"* 10 EXISTS\r\n* OK [UIDVALIDITY 100]\r\nA0001 OK SELECT completed\r\n",
                )
                .await
                .unwrap();

            // 2. Enter IDLE + push "* 11 EXISTS"
            let _ = server_stream.read(&mut buf).await.unwrap();
            server_stream
                .write_all(b"+ idling\r\n* 11 EXISTS\r\n")
                .await
                .unwrap();

            // 3. Expect DONE from client
            let n = server_stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"DONE\r\n");
            server_stream
                .write_all(b"A0002 OK IDLE completed\r\n")
                .await
                .unwrap();

            // 4. Incremental sync SELECT "INBOX"
            let _ = server_stream.read(&mut buf).await.unwrap();
            server_stream
                .write_all(
                    b"* 11 EXISTS\r\n* OK [UIDVALIDITY 100]\r\n* OK [UIDNEXT 12]\r\n* OK [HIGHESTMODSEQ 55]\r\nA0003 OK SELECT completed\r\n",
                )
                .await
                .unwrap();

            // 5. Incremental sync UID FETCH 11:*
            let _ = server_stream.read(&mut buf).await.unwrap();
            let header = b"From: sender@example.com\r\nTo: recipient@example.org\r\nSubject: Real-time Alert\r\nMessage-ID: <msg-11@example.com>\r\n\r\n";
            let body = b"Hello from IDLE wakeup!\r\n";
            let fetch_resp = format!(
                "* 11 FETCH (UID 11 MODSEQ (55) FLAGS (\\Seen) BODY[HEADER] {{{}}}\r\n",
                header.len()
            );
            server_stream
                .write_all(fetch_resp.as_bytes())
                .await
                .unwrap();
            server_stream.write_all(header).await.unwrap();
            let text_part = format!(" BODY[TEXT] {{{}}}\r\n", body.len());
            server_stream.write_all(text_part.as_bytes()).await.unwrap();
            server_stream.write_all(body).await.unwrap();
            server_stream
                .write_all(b")\r\nA0004 OK UID FETCH completed\r\n")
                .await
                .unwrap();

            // 6. CONDSTORE CHANGEDSINCE 50
            let _ = server_stream.read(&mut buf).await.unwrap();
            server_stream
                .write_all(b"A0005 OK FETCH completed\r\n")
                .await
                .unwrap();
        });

        let outcome = manager
            .run_session_cycle(&mut session, &caps, &sink)
            .await
            .unwrap();
        assert_eq!(
            outcome,
            IdleCycleOutcome::SyncedOnEvent {
                notification: IdleNotification::Exists(11),
                synced_count: 1,
                new_last_uid: 11,
                highest_modseq: Some(55),
            }
        );
        assert_eq!(sink.events.load(Ordering::SeqCst), 1);
        assert_eq!(sink.synced_messages.load(Ordering::SeqCst), 1);
        server_task.await.unwrap();
    }

    struct MockPollingConnector {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl ImapConnector for MockPollingConnector {
        async fn sync_incremental(
            &self,
            endpoint: &ImapEndpointConfig,
            folder: &str,
            last_uid: u32,
            _last_modseq: Option<u64>,
            sink: &dyn SyncEventSink,
        ) -> Result<Vec<ParsedInboundMessage>, EngineError> {
            let call_idx = self.calls.fetch_add(1, Ordering::SeqCst);
            let _ = sink.on_sync_progress(&SyncProgressEvent {
                account_id: endpoint.account_id.clone(),
                folder: folder.to_string(),
                synced_count: if call_idx == 0 { 1 } else { 0 },
                uidvalidity: 100,
                highest_modseq: Some(10),
            });
            if call_idx == 0 {
                Ok(vec![ParsedInboundMessage {
                    uid: last_uid + 1,
                    modseq: Some(10),
                    message_id_header: Some("<poll-1@example.com>".to_string()),
                    in_reply_to: None,
                    references: Vec::new(),
                    references_header: None,
                    subject: "Polled message".to_string(),
                    sender: "alerts@example.com".to_string(),
                    recipients: vec!["user@example.org".to_string()],
                    date_epoch_ms: None,
                    text_plain: Some("Body".to_string()),
                    html_raw: None,
                    raw_headers: None,
                    unsubscribe: None,
                    attachments: Vec::new(),
                }])
            } else {
                Ok(Vec::new())
            }
        }

        async fn fetch_attachment_bytes(
            &self,
            _endpoint: &ImapEndpointConfig,
            _folder: &str,
            _uid: u32,
            _part_id: &str,
        ) -> Result<Vec<u8>, EngineError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn background_polling_worker_runs_and_stops_cleanly_on_shutdown() {
        let mut cfg = IdleConfig::new("acct-poll", "INBOX");
        cfg.poll_config = AdaptivePollConfig {
            base_interval: Duration::from_millis(15),
            max_interval: Duration::from_millis(50),
            empty_poll_step_factor: 2,
        };

        let manager = IdleManager::new(cfg, 100, 1, None);
        let connector = Arc::new(MockPollingConnector {
            calls: AtomicUsize::new(0),
        });
        let sink = Arc::new(MockSink::new());
        let endpoint = ImapEndpointConfig {
            account_id: "acct-poll".to_string(),
            host: "imap.example.com".to_string(),
            port: 993,
            auth_method: AuthMethod::AppPassword,
            allow_loopback_self_signed_tls: false,
        };

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let handle =
            manager.spawn_polling_worker(connector.clone(), endpoint, sink.clone(), shutdown_rx);

        tokio::time::sleep(Duration::from_millis(40)).await;
        shutdown_tx.send(true).unwrap();

        let worker_res = handle.await.unwrap();
        assert!(worker_res.is_ok());
        assert!(connector.calls.load(Ordering::SeqCst) >= 1);
        assert!(sink.synced_messages.load(Ordering::SeqCst) >= 1);
    }
}
