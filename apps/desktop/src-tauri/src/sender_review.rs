//! Local review never gates sync, rules, reminders or alert delivery.
use crate::{DesktopService, IpcError, MessageRowDto, SearchQueryDto};
use std::sync::Arc;
use storage::sender_review::{ReviewPreferences, ReviewState, SenderChoice};
fn error(e: storage::StorageError) -> IpcError {
    match e {
        storage::StorageError::InvalidSearchQuery(_) => IpcError::InvalidRequest,
        _ => IpcError::StorageUnavailable,
    }
}
impl DesktopService {
    pub async fn review_preferences(
        self: &Arc<Self>,
        account: String,
    ) -> Result<ReviewPreferences, IpcError> {
        self.blocking(move |s| s.store.review_preferences(&account).map_err(error))
            .await
    }
    pub async fn save_review_preferences(
        self: &Arc<Self>,
        value: ReviewPreferences,
    ) -> Result<ReviewPreferences, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        let result = self
            .blocking(move |s| s.store.save_review_preferences(value).map_err(error))
            .await?;
        crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        Ok(result)
    }
    pub async fn review_metadata(
        self: &Arc<Self>,
        id: String,
    ) -> Result<Option<SenderChoice>, IpcError> {
        self.blocking(move |s| s.store.review_metadata(&id).map_err(error))
            .await
    }
    pub async fn list_sender_choices(
        self: &Arc<Self>,
        account: String,
        offset: usize,
    ) -> Result<Vec<SenderChoice>, IpcError> {
        self.blocking(move |s| s.store.list_sender_choices(&account, offset).map_err(error))
            .await
    }
    pub async fn save_sender_choice(
        self: &Arc<Self>,
        value: SenderChoice,
    ) -> Result<SenderChoice, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        let result = self
            .blocking(move |s| s.store.save_sender_choice(value).map_err(error))
            .await?;
        crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        Ok(result)
    }
    pub async fn backfill_sender_review(self: &Arc<Self>) -> Result<usize, IpcError> {
        let count = self
            .blocking(|s| s.store.backfill_sender_review().map_err(error))
            .await?;
        if count > 0 {
            crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        }
        Ok(count)
    }
    pub async fn search_sender_review(
        self: &Arc<Self>,
        query: SearchQueryDto,
        state: ReviewState,
    ) -> Result<Vec<MessageRowDto>, IpcError> {
        if query.query.len() > 4096 {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(move |s| {
            // Review uses an account selection, never a mailbox-membership restriction.
            let account = match &query.scope {
                Some(storage::MailboxScope::AccountView { account_id, .. }) => {
                    Some(account_id.clone())
                }
                _ => None,
            };
            let embedding = if query.mode == storage::SearchMode::Lexical {
                None
            } else {
                if !s.read_stored_settings()?.experimental_semantic_search {
                    return Err(IpcError::InvalidRequest);
                }
                Some(s.query_embedding(&query.query)?)
            };
            let mut query = query
                .into_hybrid_query(embedding)
                .map_err(|_| IpcError::InvalidRequest)?;
            query.account_filter = account;
            s.store.search_sender_review(&query, state).map_err(error)
        })
        .await
    }
}
