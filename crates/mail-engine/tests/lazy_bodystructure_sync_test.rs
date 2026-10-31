// INVARIANT: Integration tests must use synthetic RFC 2606 addresses (`user@example.com`,
// `user@example.org`) and never leak real email addresses or tokens.
// INVARIANT: Never fetch full RFC822 bodies or BODY.PEEK[TEXT] in bulk sync. Binary attachments
// are never downloaded during sync; only headers, flags, bodystructure, and leaf text sections are fetched.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

use mail_engine::protocol::ImapSession;
use mail_engine::provider::ServerCapabilities;
use mail_engine::sync::{execute_incremental_sync, IncrementalSyncQuery};
use mail_engine::{EngineError, SyncEventSink, SyncProgressEvent, DEFAULT_SYNC_UID_BATCH_SIZE};

struct TestSink {
    notified_count: AtomicUsize,
    last_event: Mutex<Option<SyncProgressEvent>>,
}

impl TestSink {
    fn new() -> Self {
        Self {
            notified_count: AtomicUsize::new(0),
            last_event: Mutex::new(None),
        }
    }
}

impl SyncEventSink for TestSink {
    fn on_sync_progress(&self, event: &SyncProgressEvent) -> Result<(), EngineError> {
        self.notified_count
            .fetch_add(event.synced_count, Ordering::SeqCst);
        let mut last = self.last_event.lock().unwrap();
        *last = Some(event.clone());
        Ok(())
    }
}

#[tokio::test]
async fn test_regression_synthetic_multipart_large_attachment_never_downloads_attachment() {
    let (client_stream, mut server_stream) = duplex(16384);
    let mut session = ImapSession::new(client_stream);
    let sink = TestSink::new();
    let wire_commands = Arc::new(Mutex::new(Vec::<String>::new()));
    let wire_cmds_server = wire_commands.clone();

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];

        // 1. SELECT "INBOX"
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
        wire_cmds_server.lock().unwrap().push(cmd);

        let select_resp = "* 1 EXISTS\r\n* OK [UIDVALIDITY 4000] UIDs valid\r\n* OK [UIDNEXT 102]\r\n* OK [HIGHESTMODSEQ 99]\r\nA0001 OK [READ-WRITE] SELECT completed\r\n";
        server_stream
            .write_all(select_resp.as_bytes())
            .await
            .unwrap();

        // 2. UID SEARCH UID 101:*
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
        wire_cmds_server.lock().unwrap().push(cmd);

        server_stream
            .write_all(b"* SEARCH 101\r\nA0002 OK UID SEARCH completed\r\n")
            .await
            .unwrap();

        // 3. Phase 1: UID FETCH 101 (UID MODSEQ FLAGS BODYSTRUCTURE BODY.PEEK[HEADER])
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
        wire_cmds_server.lock().unwrap().push(cmd);

        let headers = concat!(
            "From: billing@example.com\r\n",
            "To: customer@example.org\r\n",
            "Subject: Monthly Invoice & Backup\r\n",
            "Message-ID: <invoice-101@example.com>\r\n",
            "Content-Type: multipart/mixed; boundary=\"boundary-xyz\"\r\n\r\n"
        );
        // 50 MiB attachment in section 2
        let bs = r#"((("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 200 6) ("TEXT" "HTML" ("CHARSET" "UTF-8") NIL NIL "7BIT" 500 12) "ALTERNATIVE" ("BOUNDARY" "alt-1") NIL NIL) ("APPLICATION" "OCTET-STREAM" ("NAME" "large_backup.bin") "<backup-cid-101>" "50MB Backup" "BASE64" 52428800 NIL ("ATTACHMENT" ("FILENAME" "large_backup.bin")) NIL) "MIXED" ("BOUNDARY" "boundary-xyz") NIL NIL)"#;

        let phase1_resp = format!(
            "* 1 FETCH (UID 101 MODSEQ (99) FLAGS (\\Seen) BODYSTRUCTURE {} BODY[HEADER] {{{}}}\r\n{})\r\nA0003 OK UID FETCH completed\r\n",
            bs,
            headers.len(),
            headers
        );
        server_stream
            .write_all(phase1_resp.as_bytes())
            .await
            .unwrap();

        // 4. Phase 2: UID FETCH 101 for selective leaf text sections only (1.1 and 1.2)
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
        wire_cmds_server.lock().unwrap().push(cmd);

        let plain_mime = b"Content-Type: text/plain; charset=utf-8\r\n\r\n";
        let plain_body = b"Please find your monthly invoice attached.";
        let html_mime = b"Content-Type: text/html; charset=utf-8\r\n\r\n";
        let html_body = b"<p>Please find your monthly invoice attached.</p>";

        let phase2_resp = format!(
            concat!(
                "* 1 FETCH (UID 101 ",
                "BODY[1.1.MIME] {{{}}}\r\n{} ",
                "BODY[1.1] {{{}}}\r\n{} ",
                "BODY[1.2.MIME] {{{}}}\r\n{} ",
                "BODY[1.2] {{{}}}\r\n{})\r\n",
                "A0004 OK UID FETCH completed\r\n"
            ),
            plain_mime.len(),
            String::from_utf8_lossy(plain_mime),
            plain_body.len(),
            String::from_utf8_lossy(plain_body),
            html_mime.len(),
            String::from_utf8_lossy(html_mime),
            html_body.len(),
            String::from_utf8_lossy(html_body),
        );
        server_stream
            .write_all(phase2_resp.as_bytes())
            .await
            .unwrap();

        // 5. CONDSTORE CHANGEDSINCE 50
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]).to_string();
        wire_cmds_server.lock().unwrap().push(cmd);

        server_stream
            .write_all(b"A0005 OK UID FETCH completed\r\n")
            .await
            .unwrap();
    });

    let caps = ServerCapabilities::from_tokens(["IMAP4rev1", "CONDSTORE"]);
    let query = IncrementalSyncQuery {
        account_id: "acct-lazy",
        folder: "INBOX",
        local_uidvalidity: 4000,
        local_last_uid: 100,
        local_last_modseq: Some(50),
    };

    let result = execute_incremental_sync(&mut session, query, &caps, &sink)
        .await
        .unwrap();

    assert!(!result.uidvalidity_reset);
    assert_eq!(result.new_messages.len(), 1);

    let msg = &result.new_messages[0];
    assert_eq!(msg.uid, 101);
    assert_eq!(msg.subject, "Monthly Invoice & Backup");
    assert_eq!(
        msg.text_plain.as_deref(),
        Some("Please find your monthly invoice attached.")
    );
    assert_eq!(
        msg.html_raw.as_deref(),
        Some("<p>Please find your monthly invoice attached.</p>")
    );

    // Verify attachment metadata extracted strictly from BODYSTRUCTURE without downloading payload
    assert_eq!(msg.attachments.len(), 1);
    let att = &msg.attachments[0];
    assert_eq!(att.part_id, "2");
    assert_eq!(att.filename, "large_backup.bin");
    assert_eq!(att.mime_type, "application/octet-stream");
    assert_eq!(att.size_bytes, 52428800 * 3 / 4); // Estimated decoded base64 size
    assert_eq!(att.content_id.as_deref(), Some("backup-cid-101"));
    assert!(!att.is_inline);

    // Verify wire commands: NEVER request BODY.PEEK[TEXT], BODY[], or RFC822!
    let recorded = wire_commands.lock().unwrap();
    for cmd in recorded.iter() {
        assert!(
            !cmd.contains("BODY.PEEK[TEXT]"),
            "wire command must NEVER contain BODY.PEEK[TEXT]: {cmd}"
        );
        assert!(
            !cmd.contains("BODY[]"),
            "wire command must NEVER contain full BODY[]: {cmd}"
        );
        assert!(
            !cmd.contains("RFC822"),
            "wire command must NEVER contain RFC822: {cmd}"
        );
        assert!(
            !cmd.contains("BODY.PEEK[2]"),
            "wire command must NEVER eagerly request attachment section 2: {cmd}"
        );
    }
}

#[tokio::test]
async fn test_batched_sync_respects_batch_limits_and_resumes_interruption() {
    let (client_stream, mut server_stream) = duplex(32768);
    let mut session = ImapSession::new(client_stream);
    let sink = TestSink::new();

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];

        // 1. SELECT "INBOX"
        let _ = server_stream.read(&mut buf).await.unwrap();
        let select_resp = "* 650 EXISTS\r\n* OK [UIDVALIDITY 5000] UIDs valid\r\n* OK [UIDNEXT 651]\r\nA0001 OK SELECT completed\r\n";
        server_stream
            .write_all(select_resp.as_bytes())
            .await
            .unwrap();

        // 2. UID SEARCH UID 1:*
        let _ = server_stream.read(&mut buf).await.unwrap();
        // Server reports 650 UIDs: 1..=650
        let mut search_resp = String::from("* SEARCH");
        for u in 1..=650 {
            search_resp.push_str(&format!(" {u}"));
        }
        search_resp.push_str("\r\nA0002 OK UID SEARCH completed\r\n");
        server_stream
            .write_all(search_resp.as_bytes())
            .await
            .unwrap();

        // 3. Phase 1: Client requests first batch (UIDs 1:300)
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(
            cmd.contains("UID FETCH 1:300"),
            "client must query exactly the bounded batch 1:300, got: {cmd}"
        );

        let mut phase1_resp = String::new();
        let bs = r#"("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 20 1 NIL NIL NIL NIL)"#;
        for u in 1..=300 {
            let hdr = format!(
                "From: user{}@example.com\r\nSubject: Batch Msg {}\r\n\r\n",
                u, u
            );
            phase1_resp.push_str(&format!(
                "* {} FETCH (UID {} FLAGS (\\Seen) BODYSTRUCTURE {} BODY[HEADER] {{{}}}\r\n{})\r\n",
                u,
                u,
                bs,
                hdr.len(),
                hdr
            ));
        }
        phase1_resp.push_str("A0003 OK UID FETCH completed\r\n");
        server_stream
            .write_all(phase1_resp.as_bytes())
            .await
            .unwrap();

        // 4. Phase 2: Client requests BODY.PEEK[1] for batch 1:300
        let n = server_stream.read(&mut buf).await.unwrap();
        let cmd = String::from_utf8_lossy(&buf[..n]);
        assert!(
            cmd.contains("UID FETCH 1:300 (UID BODY.PEEK[1])"),
            "client must fetch selective section 1 for batch 1:300, got: {cmd}"
        );

        let mut phase2_resp = String::new();
        for u in 1..=300 {
            let body = format!("Body text {}\r\n", u);
            phase2_resp.push_str(&format!(
                "* {} FETCH (UID {} BODY[1] {{{}}}\r\n{})\r\n",
                u,
                u,
                body.len(),
                body
            ));
        }
        phase2_resp.push_str("A0004 OK UID FETCH completed\r\n");
        server_stream
            .write_all(phase2_resp.as_bytes())
            .await
            .unwrap();

        // 5. Fallback UID FLAGS
        let _ = server_stream.read(&mut buf).await.unwrap();
        server_stream
            .write_all(b"A0005 OK UID FETCH completed\r\n")
            .await
            .unwrap();
    });

    let caps = ServerCapabilities::from_tokens(["IMAP4rev1"]);
    let query = IncrementalSyncQuery {
        account_id: "acct-batched",
        folder: "INBOX",
        local_uidvalidity: 5000,
        local_last_uid: 0,
        local_last_modseq: None,
    };

    let result = execute_incremental_sync(&mut session, query, &caps, &sink)
        .await
        .unwrap();

    assert_eq!(
        result.new_messages.len(),
        DEFAULT_SYNC_UID_BATCH_SIZE as usize
    );
    assert_eq!(result.new_last_uid, 300);
    assert!(!result.uidvalidity_reset);

    // Verify first message in batch
    assert_eq!(result.new_messages[0].uid, 1);
    assert_eq!(result.new_messages[0].subject, "Batch Msg 1");

    // Verify last message in batch
    assert_eq!(result.new_messages[299].uid, 300);
    assert_eq!(result.new_messages[299].subject, "Batch Msg 300");
}
