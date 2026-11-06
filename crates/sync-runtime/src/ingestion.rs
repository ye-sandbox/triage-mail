//! Synchronous local ingestion; hosts retain execution, ownership and post-commit hooks.
use mail_engine::{sync::IncrementalSyncResult, ParsedInboundMessage};
use storage::{AttachmentRecord, FolderSyncState, MessageRecord, SqliteMailStore};
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
