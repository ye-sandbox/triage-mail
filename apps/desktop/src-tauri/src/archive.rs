//! Local offline `.mbox` and `.eml` archive ingestion orchestration.
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use mail_engine::{
    parse_eml_reader, EngineError, LocalArchiveFormat, MboxReadConfig, MboxReader,
    ParsedArchiveMessage,
};
use sync_runtime::ingestion::{ingest_archive_stream, IngestError};
use triage_ai::{DeterministicTriageResult, TriageCategory, TriageEventHook};

use crate::dto::{LocalArchiveImportDto, SyncProgressEvent, SyncState};
use crate::events::BatchEvents;
use crate::service::{now_ms, DesktopService, IpcError};

pub type ProgressCallback = Arc<dyn Fn(SyncProgressEvent) + Send + Sync>;

impl DesktopService {
    pub async fn import_local_archive(
        self: &Arc<Self>,
        path: PathBuf,
        email_display: String,
        format: LocalArchiveFormat,
        on_progress: Option<ProgressCallback>,
    ) -> Result<LocalArchiveImportDto, IpcError> {
        let clean_email = email_display.trim().to_string();
        if clean_email.is_empty()
            || clean_email.len() > 320
            || clean_email.chars().any(char::is_control)
        {
            return Err(IpcError::InvalidRequest);
        }

        let _mutation_guard = self.mutation_gate.lock().await;
        self.blocking(move |s| import_sync(&s, path, clean_email, format, on_progress))
            .await
    }
}

fn import_sync(
    service: &DesktopService,
    path: PathBuf,
    clean_email: String,
    format: LocalArchiveFormat,
    on_progress: Option<ProgressCallback>,
) -> Result<LocalArchiveImportDto, IpcError> {
    let file = crate::backup::open_local_input(&path)?;
    let cfg = MboxReadConfig::default();

    let messages: Vec<Result<ParsedArchiveMessage, EngineError>>;
    let mut mbox_iter;

    // Pre-validate the archive stream before creating any LocalArchive account record.
    let account_id: String;
    let tz_offset = crate::reminders::local_tz_offset_seconds();

    let summary = match format {
        LocalArchiveFormat::Eml => {
            let parsed = parse_eml_reader(file, 1, cfg.max_message_bytes)
                .map_err(|_| IpcError::InvalidRequest)?;
            account_id = service.ensure_local_archive_account(&clean_email)?;
            messages = vec![Ok(parsed)];
            run_ingest_stream(service, &account_id, tz_offset, messages, &on_progress)?
        }
        LocalArchiveFormat::Mbox => {
            let reader = MboxReader::with_config(BufReader::new(file), cfg)
                .map_err(|_| IpcError::InvalidRequest)?;
            let mut peekable = reader.peekable();
            if let Some(Err(_)) = peekable.peek() {
                return Err(IpcError::InvalidRequest);
            }
            account_id = service.ensure_local_archive_account(&clean_email)?;
            mbox_iter = peekable;
            run_ingest_stream(
                service,
                &account_id,
                tz_offset,
                &mut mbox_iter,
                &on_progress,
            )?
        }
    };

    let done_event = SyncProgressEvent {
        account_id: account_id.clone(),
        state: SyncState::Idle,
        new_messages_count: 0,
        triage_alerts: Vec::new(),
        embedding_errors_count: 0,
        diagnostic: None,
    };
    service.events.publish(done_event.clone());
    if let Some(ref callback) = on_progress {
        callback(done_event);
    }

    Ok(LocalArchiveImportDto {
        account_id,
        imported_count: summary.parsed_count,
        batches_committed: summary.batches_committed,
        last_uid: summary.last_uid,
    })
}

fn run_ingest_stream<I>(
    service: &DesktopService,
    account_id: &str,
    tz_offset: i32,
    messages: I,
    on_progress: &Option<ProgressCallback>,
) -> Result<sync_runtime::ingestion::ArchiveIngestSummary, IpcError>
where
    I: IntoIterator<Item = Result<ParsedArchiveMessage, EngineError>>,
{
    ingest_archive_stream(
        &service.store,
        account_id,
        "INBOX",
        messages,
        now_ms,
        |records| {
            let batch_events = BatchEvents::new(account_id.to_string(), service.events.clone());
            for record in records {
                let _ = service.store.schedule_reminders_for_message(
                    &record.id,
                    storage::now_epoch_ms(),
                    tz_offset,
                );
                let category = record
                    .category_badge
                    .as_deref()
                    .and_then(|c| match c {
                        "Security" => Some(TriageCategory::Security),
                        "Financial" => Some(TriageCategory::Financial),
                        "Shopping" => Some(TriageCategory::Shopping),
                        "Newsletter" => Some(TriageCategory::Newsletter),
                        "Personal" => Some(TriageCategory::Personal),
                        "Notifications" => Some(TriageCategory::Notifications),
                        _ => None,
                    })
                    .unwrap_or(TriageCategory::Personal);
                let triage = DeterministicTriageResult {
                    category,
                    badges: record.triage_badges.clone(),
                };
                let _ = batch_events.on_message_triaged(&record.id, &triage);
            }
            if crate::worker::index_injected_provider(service, records, &batch_events).is_err() {
                return Err(IngestError::StorageUnavailable);
            }
            let mut event = batch_events.snapshot_with_state(SyncState::Syncing);
            event.new_messages_count = records.len();
            service.events.publish(event.clone());
            if let Some(ref callback) = on_progress {
                callback(event);
            }
            Ok(())
        },
    )
    .map_err(|err| match err {
        IngestError::InvalidRequest => IpcError::InvalidRequest,
        IngestError::StorageUnavailable => IpcError::StorageUnavailable,
        IngestError::Internal => IpcError::Internal,
    })
}
