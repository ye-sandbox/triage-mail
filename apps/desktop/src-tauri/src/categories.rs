//! Category commands are local-only and never enqueue rules, reminders or remote mutations.
use crate::{DesktopService, IpcError, MessageRowDto, SearchQueryDto};
use std::sync::Arc;
use storage::categories::{CategoryMetadata, SenderOverride};
use triage_ai::TriageCategory;
fn error(e: storage::StorageError) -> IpcError {
    match e {
        storage::StorageError::InvalidSearchQuery(_) => IpcError::InvalidRequest,
        _ => IpcError::StorageUnavailable,
    }
}
impl DesktopService {
    pub async fn category_metadata(
        self: &Arc<Self>,
        id: String,
    ) -> Result<CategoryMetadata, IpcError> {
        self.blocking(move |s| s.store.category_metadata(&id).map_err(error))
            .await
    }
    pub async fn correct_category(
        self: &Arc<Self>,
        id: String,
        revision: u32,
        category: Option<TriageCategory>,
    ) -> Result<CategoryMetadata, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        let value = self
            .blocking(move |s| {
                s.store
                    .correct_category(&id, revision, category)
                    .map_err(error)
            })
            .await?;
        crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        Ok(value)
    }
    pub async fn list_category_overrides(
        self: &Arc<Self>,
        offset: usize,
    ) -> Result<Vec<SenderOverride>, IpcError> {
        self.blocking(move |s| s.store.list_category_overrides(offset).map_err(error))
            .await
    }
    pub async fn save_category_override(
        self: &Arc<Self>,
        value: SenderOverride,
    ) -> Result<SenderOverride, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        let value = self
            .blocking(move |s| s.store.save_category_override(value).map_err(error))
            .await?;
        crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        Ok(value)
    }
    pub async fn backfill_categories(self: &Arc<Self>) -> Result<usize, IpcError> {
        let count = self
            .blocking(|s| s.store.backfill_categories(300).map_err(error))
            .await?;
        if count > 0 {
            crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        }
        Ok(count)
    }
    pub async fn search_category(
        self: &Arc<Self>,
        query: SearchQueryDto,
        category: TriageCategory,
    ) -> Result<Vec<MessageRowDto>, IpcError> {
        if query.query.len() > 4096
            || query.limit == 0
            || query.limit > 500
            || query
                .offset
                .checked_add(query.limit)
                .is_none_or(|n| i64::try_from(n).is_err())
        {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(move |s| {
            let embedding = if query.mode == storage::SearchMode::Lexical {
                None
            } else {
                if !s.read_stored_settings()?.experimental_semantic_search {
                    return Err(IpcError::InvalidRequest);
                }
                Some(
                    s.embedding
                        .as_ref()
                        .ok_or(IpcError::EmbeddingUnavailable)?
                        .embed_int8(&query.query)
                        .map_err(|_| IpcError::EmbeddingUnavailable)?,
                )
            };
            let scope = query.scope.clone();
            let query = query
                .into_hybrid_query(embedding)
                .map_err(|_| IpcError::InvalidRequest)?;
            s.store
                .search_category(&query, scope.as_ref(), category)
                .map_err(error)
        })
        .await
    }
}
