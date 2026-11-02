// INVARIANT: Incremental folder sync must gracefully handle UIDVALIDITY resets,
// fetch only lightweight headers and text bodies (never raw binary attachments eagerly),
// and branch dynamically between RFC 7162 CONDSTORE and fallback flag reconciliation.

use tokio::io::{AsyncRead, AsyncWrite};

use crate::protocol::{ImapSession, UidFlags};
use crate::provider::ServerCapabilities;
use crate::{EngineError, ParsedInboundMessage, SyncEventSink, SyncProgressEvent};

/// Outcome of an incremental folder synchronization pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalSyncResult {
    pub folder: String,
    pub uidvalidity: u32,
    pub uidvalidity_reset: bool,
    /// SELECT snapshot frontier for excluding initial history from automatic rules.
    pub server_uidnext: u32,
    pub new_last_uid: u32,
    pub highest_modseq: Option<u64>,
    pub new_messages: Vec<ParsedInboundMessage>,
    pub flag_deltas: Vec<UidFlags>,
}

/// Parameters specifying which folder and watermarks to synchronize incrementally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncrementalSyncQuery<'a> {
    pub account_id: &'a str,
    pub folder: &'a str,
    pub local_uidvalidity: u32,
    pub local_last_uid: u32,
    pub local_last_modseq: Option<u64>,
}

/// Executes incremental synchronization over an active IMAP session.
pub async fn execute_incremental_sync<S: AsyncRead + AsyncWrite + Unpin + Send>(
    session: &mut ImapSession<S>,
    query: IncrementalSyncQuery<'_>,
    capabilities: &ServerCapabilities,
    sink: &dyn SyncEventSink,
) -> Result<IncrementalSyncResult, EngineError> {
    let folder_status = session.select_folder(query.folder).await?;

    // 1. Detect UIDVALIDITY reset (RFC 3501 Section 2.3.1.1)
    if query.local_uidvalidity > 0 && folder_status.uidvalidity != query.local_uidvalidity {
        return Ok(IncrementalSyncResult {
            folder: query.folder.to_string(),
            uidvalidity: folder_status.uidvalidity,
            uidvalidity_reset: true,
            server_uidnext: folder_status.uidnext,
            new_last_uid: 0,
            highest_modseq: folder_status.highest_modseq,
            new_messages: Vec::new(),
            flag_deltas: Vec::new(),
        });
    }

    let mut parsed_messages = Vec::new();
    let mut max_uid = query.local_last_uid;
    let mut fetched_flags = Vec::new();

    // 2. Fetch new messages (Lightweight Invariant: headers + selective body text only)
    let has_new_messages = if folder_status.uidnext > 0 {
        folder_status.uidnext > query.local_last_uid.saturating_add(1) || query.local_last_uid == 0
    } else {
        folder_status.exists > 0
    };

    if has_new_messages {
        let raw_messages = session
            .fetch_new_messages_two_phase(query.local_last_uid, crate::DEFAULT_SYNC_UID_BATCH_SIZE)
            .await?;

        for raw in raw_messages {
            fetched_flags.push(UidFlags {
                uid: raw.uid,
                flags: raw.flags.clone(),
            });
            if raw.uid > max_uid {
                max_uid = raw.uid;
            }

            let extracted_bs = raw
                .bodystructure
                .as_deref()
                .and_then(|bs| crate::bodystructure::parse_bodystructure_str(bs).ok())
                .map(|s| crate::bodystructure::extract_body_structure_metadata(&s));

            let fallback_body = if raw.text_bytes.is_empty() {
                if raw.raw_payload.is_empty() {
                    None
                } else {
                    Some(raw.raw_payload.as_slice())
                }
            } else {
                Some(raw.text_bytes.as_slice())
            };

            if let Ok(mut parsed) = crate::bodystructure::assemble_parsed_inbound_message(
                raw.uid,
                raw.modseq,
                &raw.header_bytes,
                extracted_bs.as_ref(),
                &raw.sections,
                fallback_body,
            ) {
                if parsed.provider_message_id.is_none() {
                    parsed.provider_message_id = raw.provider_message_id;
                }
                parsed_messages.push(parsed);
            }
        }
    }

    // 3. Flag / deletion reconciliation: CONDSTORE vs Fallback
    let mut flag_deltas = if capabilities.supports_condstore() {
        if let Some(modseq) = query.local_last_modseq {
            if modseq > 0 {
                session.fetch_flags_changed_since(modseq).await?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        // Fallback for Outlook or generic servers without CONDSTORE
        session.fetch_all_uid_flags().await?
    };

    fetched_flags.append(&mut flag_deltas);
    let flag_deltas = fetched_flags;

    // 4. Notify decoupled observers
    let progress = SyncProgressEvent {
        account_id: query.account_id.to_string(),
        folder: query.folder.to_string(),
        synced_count: parsed_messages.len(),
        uidvalidity: folder_status.uidvalidity,
        highest_modseq: folder_status.highest_modseq,
    };
    let _ = sink.on_sync_progress(&progress);

    Ok(IncrementalSyncResult {
        folder: query.folder.to_string(),
        uidvalidity: folder_status.uidvalidity,
        uidvalidity_reset: false,
        server_uidnext: folder_status.uidnext,
        new_last_uid: max_uid,
        highest_modseq: folder_status.highest_modseq,
        new_messages: parsed_messages,
        flag_deltas,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

    struct TestSink {
        notified_count: AtomicUsize,
    }

    impl SyncEventSink for TestSink {
        fn on_sync_progress(&self, event: &SyncProgressEvent) -> Result<(), EngineError> {
            self.notified_count
                .fetch_add(event.synced_count, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn detects_uidvalidity_reset_and_flags_resync() {
        let (client_stream, mut server_stream) = duplex(4096);
        let mut session = ImapSession::new(client_stream);
        let sink = TestSink {
            notified_count: AtomicUsize::new(0),
        };

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = server_stream.read(&mut buf).await.unwrap();

            // Return new UIDVALIDITY 9999 when local expects 1000
            let resp = "* 5 EXISTS\r\n* OK [UIDVALIDITY 9999] Reset!\r\n* OK [UIDNEXT 10]\r\nA0001 OK SELECT completed\r\n";
            server_stream.write_all(resp.as_bytes()).await.unwrap();
        });

        let caps = ServerCapabilities::default();
        let query = IncrementalSyncQuery {
            account_id: "acct-01",
            folder: "INBOX",
            local_uidvalidity: 1000,
            local_last_uid: 50,
            local_last_modseq: None,
        };
        let result = execute_incremental_sync(&mut session, query, &caps, &sink)
            .await
            .unwrap();

        assert!(result.uidvalidity_reset);
        assert_eq!(result.uidvalidity, 9999);
        assert_eq!(result.new_messages.len(), 0);
    }
}
