use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use mail_engine::protocol::{IdleNotification, ImapSession};
use mail_engine::provider::ServerCapabilities;
use mail_engine::{
    validate_rfc8058_uri, AdaptivePollConfig, EngineError, IdleConfig, IdleCycleOutcome,
    IdleManager, Rfc8058ClientConfig, Rfc8058UnsubscribeClient, SyncEventSink, SyncProgressEvent,
    UnsubscribeDescriptor,
};

struct IntegrationSink {
    batches: AtomicUsize,
    total_synced: AtomicUsize,
}

impl IntegrationSink {
    fn new() -> Self {
        Self {
            batches: AtomicUsize::new(0),
            total_synced: AtomicUsize::new(0),
        }
    }
}

impl SyncEventSink for IntegrationSink {
    fn on_sync_progress(&self, event: &SyncProgressEvent) -> Result<(), EngineError> {
        self.batches.fetch_add(1, Ordering::SeqCst);
        self.total_synced
            .fetch_add(event.synced_count, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn test_idle_manager_falls_back_to_adaptive_polling_when_idle_unsupported() {
    let (client_stream, mut server_stream) = duplex(8192);
    let mut session = ImapSession::new(client_stream);
    let sink = IntegrationSink::new();

    let mut cfg = IdleConfig::new("acct-fallback", "INBOX");
    cfg.poll_config = AdaptivePollConfig {
        base_interval: Duration::from_secs(45),
        max_interval: Duration::from_secs(600),
        empty_poll_step_factor: 2,
    };

    let mut manager = IdleManager::new(cfg, 500, 20, None);
    // Server without IDLE or CONDSTORE
    let caps = ServerCapabilities::from_tokens(["IMAP4rev1"]);

    let server_task = tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        // Pass 1: SELECT INBOX -> no new messages (UIDNEXT == 21)
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(
                b"* 20 EXISTS\r\n* OK [UIDVALIDITY 500]\r\n* OK [UIDNEXT 21]\r\nA0001 OK SELECT completed\r\n",
            )
            .await
            .unwrap();

        // Fallback UID FETCH 1:* (UID FLAGS)
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"A0002 OK UID FETCH completed\r\n")
            .await
            .unwrap();

        // Pass 2: SELECT INBOX -> still empty
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(
                b"* 20 EXISTS\r\n* OK [UIDVALIDITY 500]\r\n* OK [UIDNEXT 21]\r\nA0003 OK SELECT completed\r\n",
            )
            .await
            .unwrap();

        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"A0004 OK UID FETCH completed\r\n")
            .await
            .unwrap();
    });

    // First empty poll -> stays at base 45s
    let cycle1 = manager
        .run_session_cycle(&mut session, &caps, &sink)
        .await
        .unwrap();
    assert_eq!(
        cycle1,
        IdleCycleOutcome::PolledFallback {
            synced_count: 0,
            next_delay: Duration::from_secs(45),
            new_last_uid: 20,
            highest_modseq: None,
        }
    );

    // Second consecutive empty poll -> steps up to 90s
    let cycle2 = manager
        .run_session_cycle(&mut session, &caps, &sink)
        .await
        .unwrap();
    assert_eq!(
        cycle2,
        IdleCycleOutcome::PolledFallback {
            synced_count: 0,
            next_delay: Duration::from_secs(90),
            new_last_uid: 20,
            highest_modseq: None,
        }
    );

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_idle_manager_end_to_end_heartbeat_and_event_sync() {
    let (client_stream, mut server_stream) = duplex(8192);
    let mut session = ImapSession::new(client_stream);
    let sink = IntegrationSink::new();

    let mut cfg = IdleConfig::new("acct-idle", "INBOX");
    cfg.heartbeat_interval = Duration::from_millis(30);

    let mut manager = IdleManager::new(cfg, 777, 1, Some(10));
    let caps = ServerCapabilities::from_tokens(["IMAP4rev1", "IDLE", "CONDSTORE"]);

    let server_task = tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        // Cycle 1: SELECT + IDLE -> heartbeat timeout -> DONE
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"* 1 EXISTS\r\n* OK [UIDVALIDITY 777]\r\nA0001 OK SELECT completed\r\n")
            .await
            .unwrap();

        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream.write_all(b"+ idling\r\n").await.unwrap();

        let n = server_stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"DONE\r\n");
        server_stream
            .write_all(b"A0002 OK IDLE terminated\r\n")
            .await
            .unwrap();

        // Cycle 2: SELECT + IDLE -> server emits * 2 EXISTS -> DONE -> sync
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"* 1 EXISTS\r\n* OK [UIDVALIDITY 777]\r\nA0003 OK SELECT completed\r\n")
            .await
            .unwrap();

        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"+ idling\r\n* 2 EXISTS\r\n")
            .await
            .unwrap();

        let n = server_stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"DONE\r\n");
        server_stream
            .write_all(b"A0004 OK IDLE terminated\r\n")
            .await
            .unwrap();

        // Incremental sync SELECT
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(
                b"* 2 EXISTS\r\n* OK [UIDVALIDITY 777]\r\n* OK [UIDNEXT 3]\r\n* OK [HIGHESTMODSEQ 12]\r\nA0005 OK SELECT completed\r\n",
            )
            .await
            .unwrap();

        // UID FETCH 2:*
        let _ = server_stream.read(&mut buf).await.unwrap();
        let hdr = b"From: news@example.com\r\nTo: reader@example.org\r\nSubject: Digest\r\nMessage-ID: <d2@example.com>\r\n\r\n";
        let txt = b"Digest content.\r\n";
        let line1 = format!(
            "* 2 FETCH (UID 2 MODSEQ (12) FLAGS () BODY[HEADER] {{{}}}\r\n",
            hdr.len()
        );
        server_stream.write_all(line1.as_bytes()).await.unwrap();
        server_stream.write_all(hdr).await.unwrap();
        let line2 = format!(" BODY[TEXT] {{{}}}\r\n", txt.len());
        server_stream.write_all(line2.as_bytes()).await.unwrap();
        server_stream.write_all(txt).await.unwrap();
        server_stream
            .write_all(b")\r\nA0006 OK UID FETCH completed\r\n")
            .await
            .unwrap();

        // CHANGEDSINCE 10
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"A0007 OK FETCH completed\r\n")
            .await
            .unwrap();
    });

    let c1 = manager
        .run_session_cycle(&mut session, &caps, &sink)
        .await
        .unwrap();
    assert_eq!(c1, IdleCycleOutcome::HeartbeatRenewed);

    let c2 = manager
        .run_session_cycle(&mut session, &caps, &sink)
        .await
        .unwrap();
    assert_eq!(
        c2,
        IdleCycleOutcome::SyncedOnEvent {
            notification: IdleNotification::Exists(2),
            synced_count: 1,
            new_last_uid: 2,
            highest_modseq: Some(12),
        }
    );
    assert_eq!(manager.local_last_uid(), 2);
    assert_eq!(manager.local_last_modseq(), Some(12));
    server_task.await.unwrap();
}

#[tokio::test]
async fn test_rfc8058_unsubscribe_executor_and_validation() {
    // Verify RFC 2606 synthetic HTTPS URI passes strict validation
    let valid_url =
        validate_rfc8058_uri("https://lists.example.org/unsubscribe?member=1001", false).unwrap();
    assert_eq!(valid_url.host_str(), Some("lists.example.org"));

    // Verify non-2xx HTTP status is surfaced as EngineError::UnsubscribeFailed
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await.unwrap();
        let response =
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\nConnection: close\r\n\r\nError";
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let client = Rfc8058UnsubscribeClient::new(Rfc8058ClientConfig {
        timeout: Duration::from_secs(5),
        allow_loopback_http_for_tests: true,
    })
    .unwrap();

    let desc = UnsubscribeDescriptor {
        https_uri: Some(format!("http://{}/unsub", addr)),
        mailto_uri: None,
        supports_rfc8058_one_click: true,
        raw_list_unsubscribe: None,
        raw_list_unsubscribe_post: Some("List-Unsubscribe=One-Click".to_string()),
    };

    let err = client.execute_one_click(&desc).await.unwrap_err();
    assert!(matches!(err, EngineError::UnsubscribeFailed(_)));
}
