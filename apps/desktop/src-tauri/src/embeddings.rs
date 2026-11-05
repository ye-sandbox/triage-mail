//! Learned index lifecycle. All work is local and opt-in, never a delivery/replay path.
use crate::{DesktopService, IpcError};
use serde::Serialize;
use std::sync::Arc;
use triage_ai::{onnx::GENERATION, EmbeddingProvider};

#[derive(Debug, Serialize)]
pub struct EmbeddingState {
    pub status: &'static str,
    pub indexed: u64,
    pub pending: u64,
    pub failed: u64,
}
impl DesktopService {
    /// Production composition explicitly opts into generation-managed inference.
    pub(crate) fn new_learned(
        store: impl Into<crate::service::ServiceStore>,
        vault: Arc<dyn vault::SecretStore>,
        remote: Arc<dyn crate::remote::RemoteMail>,
        provider: Option<Arc<triage_ai::onnx::OnnxEmbeddingProvider>>,
    ) -> Result<Arc<Self>, IpcError> {
        let store = store.into();
        store
            .begin_embedding_generation(GENERATION, false)
            .map_err(|_| IpcError::StorageUnavailable)?;
        let mut service = Self::new_with_store(
            store,
            vault,
            remote,
            provider.clone().map(|p| p as Arc<dyn EmbeddingProvider>),
        );
        Arc::get_mut(&mut service)
            .ok_or(IpcError::Internal)?
            .learned = provider;
        Ok(service)
    }
    pub async fn embedding_state(self: &Arc<Self>) -> Result<EmbeddingState, IpcError> {
        self.blocking(|s| {
            let p = s
                .store
                .embedding_progress()
                .map_err(|_| IpcError::StorageUnavailable)?;
            let status = if !s.read_stored_settings()?.experimental_semantic_search {
                "Disabled"
            } else if s.learned.as_ref().is_none_or(|p| p.prepare().is_err()) {
                "MissingModelOrRuntime"
            } else if p.failed > 0 {
                "Failed"
            } else if p.pending > 0 {
                "Rebuilding"
            } else {
                "Ready"
            };
            Ok(EmbeddingState {
                status,
                indexed: p.indexed,
                pending: p.pending,
                failed: p.failed,
            })
        })
        .await
    }
    pub async fn rebuild_embeddings(self: &Arc<Self>, consent: bool) -> Result<(), IpcError> {
        if !consent {
            return Err(IpcError::InvalidRequest);
        }
        self.blocking(|s| {
            let _guard = s.settings_gate.lock().map_err(|_| IpcError::Internal)?;
            if !s.read_stored_settings()?.experimental_semantic_search {
                return Err(IpcError::InvalidRequest);
            }
            s.learned
                .as_ref()
                .ok_or(IpcError::EmbeddingUnavailable)?
                .prepare()
                .map_err(|_| IpcError::EmbeddingUnavailable)?;
            s.store
                .begin_embedding_generation(GENERATION, true)
                .map_err(|_| IpcError::StorageUnavailable)
        })
        .await?;
        crate::worker::status(self, "", crate::SyncState::ResyncRequired);
        Ok(())
    }
    pub(crate) async fn index_embeddings(self: &Arc<Self>) -> Result<usize, IpcError> {
        self.blocking(|s| {
            if s.learned.is_none() || !s.read_stored_settings()?.experimental_semantic_search {
                return Ok(0);
            }
            let provider = s.learned.as_ref().ok_or(IpcError::EmbeddingUnavailable)?;
            provider
                .prepare()
                .map_err(|_| IpcError::EmbeddingUnavailable)?;
            let jobs = s
                .store
                .embedding_work(GENERATION, 8)
                .map_err(|_| IpcError::StorageUnavailable)?;
            let mut failed = 0;
            for job in &jobs {
                let text = if job.text.trim().is_empty() {
                    "[empty message]"
                } else {
                    &job.text
                };
                let result = provider.embed_int8(text);
                // INVARIANT: Disable/rebuild wins over an in-flight computation at commit.
                let _guard = s.settings_gate.lock().map_err(|_| IpcError::Internal)?;
                if !s.read_stored_settings()?.experimental_semantic_search {
                    return Ok(0);
                }
                let committed = s
                    .store
                    .finish_embedding(GENERATION, job, result.as_deref().ok())
                    .map_err(|_| IpcError::StorageUnavailable)?;
                if committed && result.is_err() {
                    failed += 1;
                }
            }
            if failed > 0 {
                s.events.publish(crate::SyncProgressEvent {
                    account_id: String::new(),
                    state: crate::SyncState::MutationFailed,
                    new_messages_count: 0,
                    triage_alerts: Vec::new(),
                    embedding_errors_count: failed,
                });
                return Err(IpcError::EmbeddingUnavailable);
            }
            Ok(jobs.len())
        })
        .await
    }
    pub(crate) fn query_embedding(&self, text: &str) -> Result<Vec<i8>, IpcError> {
        if !self.read_stored_settings()?.experimental_semantic_search {
            return Err(IpcError::InvalidRequest);
        }
        if self.learned.is_some()
            && !self
                .store
                .embedding_ready()
                .map_err(|_| IpcError::StorageUnavailable)?
        {
            return Err(IpcError::EmbeddingUnavailable);
        }
        self.embedding
            .as_ref()
            .ok_or(IpcError::EmbeddingUnavailable)?
            .embed_int8(text)
            .map_err(|_| IpcError::EmbeddingUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn missing_runtime_keeps_lexical_startup_and_consent_safe() {
        let store = Arc::new(
            storage::SqliteMailStore::open_in_memory(
                &storage::DatabaseKey::from_bytes([85; 32]),
                Default::default(),
            )
            .unwrap(),
        );
        let vault: Arc<dyn vault::SecretStore> =
            Arc::new(vault::EncryptedFallbackStore::in_memory(Arc::new(
                vault::Argon2AesGcmCipher::from_key([86; 32]),
            )));
        let remote = Arc::new(crate::remote::NativeRemoteMail::new(vault.clone()));
        let service = DesktopService::new_learned(store.clone(), vault, remote, None).unwrap();
        assert_eq!(service.embedding_state().await.unwrap().status, "Disabled");
        assert_eq!(service.index_embeddings().await.unwrap(), 0);
        assert_eq!(
            service.rebuild_embeddings(false).await,
            Err(IpcError::InvalidRequest)
        );
        assert_eq!(
            service.rebuild_embeddings(true).await,
            Err(IpcError::InvalidRequest)
        );
        service
            .set_experimental_semantic_search(true)
            .await
            .unwrap();
        assert_eq!(
            service.embedding_state().await.unwrap().status,
            "MissingModelOrRuntime"
        );
        assert_eq!(
            service.rebuild_embeddings(true).await,
            Err(IpcError::EmbeddingUnavailable)
        );
        assert_eq!(
            service.query_embedding("synthetic"),
            Err(IpcError::EmbeddingUnavailable)
        );
        use storage::SearchEngine;
        assert!(store
            .search(&storage::HybridQuery {
                query_text: String::new(),
                mode: storage::SearchMode::Lexical,
                query_embedding_int8: None,
                account_filter: None,
                limit: 10,
                offset: 0
            })
            .unwrap()
            .is_empty());
        service
            .set_experimental_semantic_search(false)
            .await
            .unwrap();
        assert_eq!(
            service.query_embedding("synthetic"),
            Err(IpcError::InvalidRequest)
        );
    }
}
