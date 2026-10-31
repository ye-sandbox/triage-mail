//! Explicit, bounded artifact downloads. No background fetcher is installed.
use std::{sync::Arc, time::Duration};

use serde::Deserialize;
use triage_ai::{
    embeddings::{ModelCacheManager, ModelFetcher, MODEL_ALL_MINILM_L6_V2},
    EmbeddingModelConfig, TriageError,
};

use crate::{service::DesktopService, IpcError};

#[derive(Deserialize)]
pub struct ModelDownloadRequest {
    pub model_url: String,
    pub tokenizer_url: String,
    pub consent: bool,
}

pub(crate) fn model_config() -> Result<EmbeddingModelConfig, IpcError> {
    EmbeddingModelConfig::from_xdg_env(MODEL_ALL_MINILM_L6_V2)
        .map_err(|_| IpcError::EmbeddingUnavailable)
}

fn artifact_url(raw: &str) -> Result<url::Url, IpcError> {
    let url = url::Url::parse(raw).map_err(|_| IpcError::InvalidRequest)?;
    if raw.len() > 4096
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(IpcError::InvalidRequest);
    }
    Ok(url)
}

async fn fetch(client: &reqwest::Client, url: url::Url, limit: usize) -> Result<Vec<u8>, IpcError> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| IpcError::RemoteUnavailable)?;
    if !response.status().is_success()
        || response.content_length().is_some_and(|n| n > limit as u64)
    {
        return Err(IpcError::RemoteUnavailable);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| IpcError::RemoteUnavailable)?
    {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err(IpcError::RemoteUnavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(IpcError::RemoteUnavailable);
    }
    Ok(bytes)
}

struct Artifacts {
    model: Vec<u8>,
    tokenizer: Vec<u8>,
}
impl ModelFetcher for Artifacts {
    fn fetch_artifact(&self, _: &str, filename: &str) -> Result<Vec<u8>, TriageError> {
        match filename {
            "model.onnx" => Ok(self.model.clone()),
            "tokenizer.json" => Ok(self.tokenizer.clone()),
            _ => Err(TriageError::Embedding("unsupported artifact".into())),
        }
    }
}

impl DesktopService {
    pub async fn download_model(
        self: &Arc<Self>,
        request: ModelDownloadRequest,
    ) -> Result<(), IpcError> {
        if !request.consent {
            return Err(IpcError::InvalidRequest);
        }
        let model_url = artifact_url(&request.model_url)?;
        let tokenizer_url = artifact_url(&request.tokenizer_url)?;
        let _guard = self
            .download_gate
            .try_lock()
            .map_err(|_| IpcError::InvalidRequest)?;
        let config = model_config()?;
        let cached = ModelCacheManager::new(config.clone());
        if cached.is_model_cached() {
            return Ok(());
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|_| IpcError::Internal)?;
        // INVARIANT: No request happens before consent and both URLs are validated.
        let model = fetch(&client, model_url, 128 * 1024 * 1024).await?;
        let tokenizer = fetch(&client, tokenizer_url, 8 * 1024 * 1024).await?;
        serde_json::from_slice::<serde_json::Value>(&tokenizer)
            .map_err(|_| IpcError::InvalidRequest)?;
        self.blocking(move |_| {
            ModelCacheManager::new(config)
                .ensure_cached(Some(&Artifacts { model, tokenizer }))
                .map(|_| ())
                .map_err(|_| IpcError::EmbeddingUnavailable)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_urls_reject_ambient_credentials_and_non_https() {
        for url in [
            "http://models.example.com/model.onnx",
            "https://user:pass@models.example.com/m",
            "https://models.example.com/m?token=private",
            "https://models.example.com/m#fragment",
        ] {
            assert!(artifact_url(url).is_err());
        }
        assert!(artifact_url("https://models.example.com/model.onnx").is_ok());
    }

    #[tokio::test]
    async fn artifact_transport_enforces_size_and_redirect_policy() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for (response, limit, success) in [
            ("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc", 3, true),
            ("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc", 2, false),
            ("HTTP/1.1 302 Found\r\nLocation: https://never-request.example.com\r\nContent-Length: 0\r\n\r\n", 3, false),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap()).parse().unwrap();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0; 1024];
                let received = socket.read(&mut bytes).await.unwrap();
                assert!(received > 0);
                assert!(bytes[..received].starts_with(b"GET / "));
                socket.write_all(response.as_bytes()).await.unwrap();
            });
            let client = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap();
            assert_eq!(fetch(&client, url, limit).await.is_ok(), success);
            server.await.unwrap();
        }
    }
}
