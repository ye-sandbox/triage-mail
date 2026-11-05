use crate::{DesktopService, IpcError, SearchQueryDto, TriageActionDto, TriageMutationDto};
use std::sync::Arc;
use storage::conversations::*;
fn error(e: storage::StorageError) -> IpcError {
    match e {
        storage::StorageError::InvalidSearchQuery(_) => IpcError::InvalidRequest,
        _ => IpcError::StorageUnavailable,
    }
}
impl DesktopService {
    fn conversation_query(
        &self,
        mut query: SearchQueryDto,
        context: &ConversationQuery,
    ) -> Result<storage::HybridQuery, IpcError> {
        if matches!(context.source, ConversationSource::Saved { .. }) {
            if context.scope.is_some() {
                return Err(IpcError::InvalidRequest);
            }
        } else if context.scope != query.scope {
            return Err(IpcError::InvalidRequest);
        }
        if let ConversationSource::Saved { id, revision } = &context.source {
            let view = self.store.saved_view(id, *revision).map_err(error)?;
            query.query = view.query;
            query.mode = view.mode;
            query.scope = None;
        }
        if query.query.len() > 4096 {
            return Err(IpcError::InvalidRequest);
        }
        let embedding = if query.mode == storage::SearchMode::Lexical {
            None
        } else {
            if !self.read_stored_settings()?.experimental_semantic_search {
                return Err(IpcError::InvalidRequest);
            }
            Some(self.query_embedding(&query.query)?)
        };
        query
            .into_hybrid_query(embedding)
            .map_err(|_| IpcError::InvalidRequest)
    }
    pub async fn conversation_progress(
        self: &Arc<Self>,
        account_id: Option<String>,
    ) -> Result<ConversationProgress, IpcError> {
        self.blocking(move |s| {
            s.store
                .conversation_progress_for(account_id.as_deref())
                .map_err(error)
        })
        .await
    }
    pub async fn backfill_conversations(self: &Arc<Self>) -> Result<usize, IpcError> {
        let (count, changed) = self
            .blocking(|s| {
                let before = s.store.conversation_progress().map_err(error)?;
                let count = s.store.backfill_conversations().map_err(error)?;
                let after = s.store.conversation_progress().map_err(error)?;
                Ok((count, !before.ready && after.ready))
            })
            .await?;
        if changed {
            crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        }
        Ok(count)
    }
    pub async fn search_conversations(
        self: &Arc<Self>,
        query: SearchQueryDto,
        context: ConversationQuery,
    ) -> Result<ConversationPage, IpcError> {
        self.blocking(move |s| {
            let query = s.conversation_query(query, &context)?;
            s.store
                .search_conversations(&query, &context)
                .map_err(error)
        })
        .await
    }
    pub async fn conversation_members(
        self: &Arc<Self>,
        query: SearchQueryDto,
        context: ConversationQuery,
        id: String,
        offset: usize,
    ) -> Result<ConversationMembers, IpcError> {
        self.blocking(move |s| {
            let query = s.conversation_query(query, &context)?;
            s.store
                .conversation_members(&query, &context, &id, offset)
                .map_err(error)
        })
        .await
    }
    pub async fn preview_conversation(
        self: &Arc<Self>,
        query: SearchQueryDto,
        context: ConversationQuery,
        id: String,
        all: bool,
        action: ConversationAction,
    ) -> Result<ConversationPreview, IpcError> {
        self.blocking(move |s| {
            let query = s.conversation_query(query, &context)?;
            s.store
                .preview_conversation(&query, &context, &id, all, action)
                .map_err(error)
        })
        .await
    }
    pub async fn apply_conversation(
        self: &Arc<Self>,
        token: String,
    ) -> Result<Vec<String>, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| {
            let (generation, ids, action) = s.store.conversation_preview(&token).map_err(error)?;
            let action = match action {
                ConversationAction::MarkRead => TriageActionDto::MarkRead,
                ConversationAction::MarkUnread => TriageActionDto::MarkUnread,
                ConversationAction::Archive => TriageActionDto::Archive,
                ConversationAction::Tag(tag) => TriageActionDto::Tag(tag),
            };
            s.enqueue_mutation_guarded(
                TriageMutationDto {
                    message_ids: ids.clone(),
                    action,
                },
                Some((&generation, &token)),
            )?;
            Ok(ids)
        })
        .await
    }
    pub async fn conversation_status(
        self: &Arc<Self>,
        ids: Vec<String>,
    ) -> Result<Vec<MutationStatus>, IpcError> {
        self.blocking(move |s| s.store.conversation_mutation_status(&ids).map_err(error))
            .await
    }
}
