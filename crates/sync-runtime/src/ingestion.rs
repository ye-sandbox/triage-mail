//! Synchronous local ingestion; hosts retain execution, ownership and post-commit hooks.
use mail_engine::{
    protocol::UidFlags, sync::IncrementalSyncResult, EngineError, ParsedArchiveMessage,
    ParsedInboundMessage, DEFAULT_SYNC_UID_BATCH_SIZE,
};
use storage::{
    AttachmentRecord, FolderRecord, FolderSyncState, MailRepository, MessageRecord, SqliteMailStore,
};
use triage_ai::{DefaultDeterministicTriage, DeterministicTriage};

/// Fixed error classifications; storage/serialization details never cross this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestError {
    InvalidRequest,
    StorageUnavailable,
    Internal,
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidRequest => "invalid sync batch",
            Self::StorageUnavailable => "sync storage unavailable",
            Self::Internal => "sync ingestion failed",
        })
    }
}
impl std::error::Error for IngestError {}

/// Summary of a completed local `.mbox` or `.eml` archive ingestion pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveIngestSummary {
    pub parsed_count: usize,
    pub batches_committed: usize,
    pub last_uid: u32,
}

/// Streams parsed local archive messages through [`ingest_batch`] in bounded chunks.
///
/// Assigns deterministic monotonic UIDs starting from the folder's current `uidnext`
/// (or `1`), maps `X-Gmail-Labels` (`Unread` / `Starred`) to local flags, and sets
/// `server_uidnext = 0` so historical archive imports never trigger live rule/review queues.
pub fn ingest_archive_stream<I>(
    store: &SqliteMailStore,
    account_id: &str,
    folder_name: &str,
    messages: I,
    mut now: impl FnMut() -> i64,
    mut on_batch_committed: impl FnMut(&[MessageRecord]) -> Result<(), IngestError>,
) -> Result<ArchiveIngestSummary, IngestError>
where
    I: IntoIterator<Item = Result<ParsedArchiveMessage, EngineError>>,
{
    if account_id.is_empty() || folder_name.is_empty() {
        return Err(IngestError::InvalidRequest);
    }

    let existing_state = store
        .get_folder_sync_state(account_id, folder_name)
        .map_err(|_| IngestError::StorageUnavailable)?;
    let uidvalidity = existing_state
        .as_ref()
        .map(|s| s.uidvalidity)
        .filter(|&v| v > 0)
        .unwrap_or(1);
    let mut next_uid = existing_state
        .as_ref()
        .map(|s| s.uidnext.max(1))
        .unwrap_or(1);

    let role = if folder_name.eq_ignore_ascii_case("INBOX") {
        "inbox"
    } else {
        "other"
    };
    let display_name = if role == "inbox" {
        "Inbox".to_string()
    } else {
        folder_name.to_string()
    };
    store
        .save_folders(
            account_id,
            &[FolderRecord {
                account_id: account_id.to_string(),
                folder_name: folder_name.to_string(),
                display_name,
                delimiter: None,
                role: role.to_string(),
                is_selectable: true,
                total_messages: None,
                unread_messages: None,
                updated_at_epoch_ms: now(),
            }],
        )
        .map_err(|_| IngestError::StorageUnavailable)?;

    let batch_cap = DEFAULT_SYNC_UID_BATCH_SIZE as usize;
    let mut batch_messages = Vec::with_capacity(batch_cap);
    let mut batch_flags = Vec::new();
    let mut parsed_count = 0usize;
    let mut batches_committed = 0usize;
    let mut last_uid = next_uid.saturating_sub(1);

    for item in messages {
        let mut archive_msg = item.map_err(|_| IngestError::InvalidRequest)?;
        let uid = next_uid;
        next_uid = next_uid.checked_add(1).ok_or(IngestError::InvalidRequest)?;
        last_uid = uid;
        parsed_count = parsed_count
            .checked_add(1)
            .ok_or(IngestError::InvalidRequest)?;

        if !archive_msg.gmail_labels.is_empty() {
            let mut flags = Vec::new();
            if !archive_msg.has_unread_label() {
                flags.push("\\Seen".to_string());
            }
            if archive_msg.has_starred_label() {
                flags.push("\\Flagged".to_string());
            }
            if !flags.is_empty() {
                batch_flags.push(UidFlags { uid, flags });
            }
        }

        archive_msg.message.uid = uid;
        archive_msg.message.modseq = None;
        batch_messages.push(archive_msg.message);

        if batch_messages.len() >= batch_cap {
            commit_archive_chunk(
                store,
                account_id,
                folder_name,
                uidvalidity,
                uid,
                std::mem::take(&mut batch_messages),
                std::mem::take(&mut batch_flags),
                &mut now,
                &mut on_batch_committed,
            )?;
            batches_committed += 1;
        }
    }

    if !batch_messages.is_empty() {
        commit_archive_chunk(
            store,
            account_id,
            folder_name,
            uidvalidity,
            last_uid,
            batch_messages,
            batch_flags,
            &mut now,
            &mut on_batch_committed,
        )?;
        batches_committed += 1;
    }

    Ok(ArchiveIngestSummary {
        parsed_count,
        batches_committed,
        last_uid,
    })
}

#[allow(clippy::too_many_arguments)]
fn commit_archive_chunk(
    store: &SqliteMailStore,
    account_id: &str,
    folder_name: &str,
    uidvalidity: u32,
    last_uid: u32,
    new_messages: Vec<ParsedInboundMessage>,
    flag_deltas: Vec<UidFlags>,
    now: &mut impl FnMut() -> i64,
    on_batch_committed: &mut impl FnMut(&[MessageRecord]) -> Result<(), IngestError>,
) -> Result<(), IngestError> {
    let state = FolderSyncState {
        account_id: account_id.to_string(),
        folder_name: folder_name.to_string(),
        uidvalidity,
        uidnext: last_uid.saturating_add(1),
        highest_modseq: None,
    };
    let result = IncrementalSyncResult {
        folder: folder_name.to_string(),
        uidvalidity,
        uidvalidity_reset: false,
        // INVARIANT: server_uidnext = 0 sets rule/review frontiers to u32::MAX so
        // historical archive imports never enqueue automated rule/alert/review storms.
        server_uidnext: 0,
        new_last_uid: last_uid,
        highest_modseq: None,
        new_messages,
        flag_deltas,
    };
    let committed = ingest_batch(store, &state, result, now)?;
    on_batch_committed(&committed)?;
    Ok(())
}

/// Establish arrival frontiers and commit messages/flags/checkpoint, returning canonical records.
///
/// The host supplies a millisecond clock, retains writer ownership and serialization,
/// and runs this synchronous work on its blocking executor. No hooks, events, workers,
/// transport, credential access or learned inference run here. Independent fixture
/// stores may be borrowed directly; this function does not acquire process ownership.
pub fn ingest_batch(
    store: &SqliteMailStore,
    state: &FolderSyncState,
    result: IncrementalSyncResult,
    mut now: impl FnMut() -> i64,
) -> Result<Vec<MessageRecord>, IngestError> {
    // INVARIANT: Establish rule then review frontiers before conversion, even for
    // empty/reset/invalid-message batches. These writes are separate transactions;
    // a later conversion/commit failure must not imply their rollback.
    store
        .establish_rule_baseline(
            &state.account_id,
            &state.folder_name,
            state.uidvalidity,
            result.server_uidnext,
        )
        .map_err(|_| IngestError::StorageUnavailable)?;
    store
        .establish_review_frontier(
            &state.account_id,
            &state.folder_name,
            state.uidvalidity,
            result.server_uidnext,
        )
        .map_err(|_| IngestError::StorageUnavailable)?;
    let records = result
        .new_messages
        .into_iter()
        .map(|message| message_record(message, state, &mut now))
        .collect::<Result<Vec<_>, _>>()?;
    let flags = message_flags(result.flag_deltas);
    // INVARIANT: Return only after the atomic batch commit; canonical IDs returned
    // by storage (including cross-folder deduplication) are authoritative for hooks.
    store
        .commit_sync_batch(&records, &flags, state, result.uidvalidity_reset)
        .map_err(|_| IngestError::StorageUnavailable)
}

fn message_record(
    message: ParsedInboundMessage,
    state: &FolderSyncState,
    now: &mut impl FnMut() -> i64,
) -> Result<MessageRecord, IngestError> {
    if message.uid == 0 {
        return Err(IngestError::InvalidRequest);
    }
    let id = serde_json::to_string(&(
        &state.account_id,
        &state.folder_name,
        state.uidvalidity,
        message.uid,
    ))
    .map_err(|_| IngestError::Internal)?;
    let unsubscribe = message.unsubscribe;
    let supported = unsubscribe
        .as_ref()
        .is_some_and(|u| u.supports_rfc8058_one_click);
    let triage = DefaultDeterministicTriage.analyze(
        &message.subject,
        &message.sender,
        message.text_plain.as_deref().unwrap_or_default(),
        supported,
    );
    let attachments = message
        .attachments
        .into_iter()
        .map(|a| {
            Ok(AttachmentRecord {
                id: format!("{id}:{}", a.part_id),
                message_id: id.clone(),
                part_id: a.part_id,
                filename: a.filename,
                content_type: a.mime_type,
                size_bytes: a
                    .size_bytes
                    .try_into()
                    .map_err(|_| IngestError::InvalidRequest)?,
                content_id: a.content_id,
                is_inline: a.is_inline,
            })
        })
        .collect::<Result<Vec<_>, IngestError>>()?;
    Ok(MessageRecord {
        id,
        account_id: state.account_id.clone(),
        folder: state.folder_name.clone(),
        uid: message.uid,
        message_id_header: message.message_id_header,
        in_reply_to: message.in_reply_to,
        references_header: message.references_header,
        subject: message.subject,
        sender: message.sender,
        recipients: message.recipients.join(", "),
        received_at_epoch_ms: message.date_epoch_ms.unwrap_or_else(&mut *now),
        is_read: false,
        has_attachments: !attachments.is_empty(),
        category_badge: Some(format!("{:?}", triage.category)),
        triage_badges: triage.badges,
        raw_headers: message.raw_headers.unwrap_or_default(),
        text_body: message.text_plain,
        html_body: message.html_raw,
        list_unsubscribe: unsubscribe
            .as_ref()
            .and_then(|u| u.raw_list_unsubscribe.clone()),
        list_unsubscribe_post: unsubscribe.and_then(|u| u.raw_list_unsubscribe_post),
        one_click_unsubscribe_supported: supported,
        created_at_epoch_ms: now(),
        attachments,
        tags: Vec::new(),
        is_starred: false,
        snooze_until_epoch_ms: None,
        provider_message_id: message.provider_message_id,
    })
}

fn message_flags(deltas: Vec<mail_engine::protocol::UidFlags>) -> Vec<storage::MessageFlagsDelta> {
    deltas
        .into_iter()
        .map(|delta| storage::MessageFlagsDelta {
            uid: delta.uid,
            is_read: delta
                .flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("\\Seen")),
            is_starred: delta
                .flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("\\Flagged")),
        })
        .collect()
}

#[cfg(test)]
mod tests;
