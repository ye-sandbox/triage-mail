//! Saved searches resolve persisted definitions before embedding and recheck revisions on execution.
use crate::{DesktopService, IpcError, MessageRowDto};
use std::sync::Arc;
use storage::{saved_views::SavedView, SearchMode, StorageError};

fn error(e: StorageError) -> IpcError {
    match e {
        StorageError::InvalidSearchQuery(_) => IpcError::InvalidRequest,
        _ => IpcError::StorageUnavailable,
    }
}

impl DesktopService {
    pub async fn list_saved_views(self: &Arc<Self>) -> Result<Vec<SavedView>, IpcError> {
        self.blocking(|s| s.store.list_saved_views().map_err(error))
            .await
    }
    pub async fn save_saved_view(self: &Arc<Self>, view: SavedView) -> Result<SavedView, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.save_saved_view(view).map_err(error))
            .await
    }
    pub async fn delete_saved_view(
        self: &Arc<Self>,
        id: String,
        revision: u32,
    ) -> Result<(), IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.delete_saved_view(&id, revision).map_err(error))
            .await
    }
    pub async fn search_saved_view(
        self: &Arc<Self>,
        id: String,
        revision: u32,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MessageRowDto>, IpcError> {
        if limit == 0
            || limit > 500
            || offset
                .checked_add(limit)
                .is_none_or(|end| i64::try_from(end).is_err())
        {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(move |s| {
            let view = s.store.saved_view(&id, revision).map_err(error)?;
            let embedding = if view.mode == SearchMode::Lexical {
                None
            } else {
                if !s.read_stored_settings()?.experimental_semantic_search {
                    return Err(IpcError::InvalidRequest);
                }
                Some(s.query_embedding(&view.query)?)
            };
            s.store
                .search_saved_view(&id, revision, embedding, limit, offset)
                .map_err(error)
        })
        .await
    }
}
