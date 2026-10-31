//! Desktop settings are vault-backed; reads and saves never contact a provider.
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use storage::FtsTokenizerMode;
use triage_ai::llm::{LlmClientConfig, LlmProviderConfig, RedactedApiKey};
use vault::VaultKey;
use zeroize::{Zeroize, Zeroizing};

use crate::service::{DesktopService, IpcError};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AiProvider {
    #[default]
    Ollama,
    Byok,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    pub enabled: bool,
    pub provider: AiProvider,
    pub endpoint: String,
    pub model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: AiProvider::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            model: "llama3.2".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsDto {
    pub ai: AiSettings,
    pub tokenizer: FtsTokenizerMode,
    pub has_api_key: bool,
    pub model_cached: bool,
    pub experimental_semantic_search: bool,
}

/// INVARIANT: Deserialize failures must not echo key material across IPC.
pub struct IncomingApiKey(pub Zeroizing<String>);
impl<'de> Deserialize<'de> for IncomingApiKey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)
            .map(|s| Self(Zeroizing::new(s)))
            .map_err(|_| serde::de::Error::custom("invalid API key material"))
    }
}

pub(crate) fn vault_key(name: &str) -> Result<VaultKey, IpcError> {
    VaultKey::for_account("application", name).map_err(|_| IpcError::Internal)
}

impl AiSettings {
    pub(crate) fn client_config(&self, key: String) -> Result<LlmClientConfig, IpcError> {
        if self.endpoint.len() > 2048 || self.model.len() > 200 || self.model.trim().is_empty() {
            return Err(IpcError::InvalidRequest);
        }
        let url = url::Url::parse(&self.endpoint).map_err(|_| IpcError::InvalidRequest)?;
        if url.query().is_some() || url.fragment().is_some() {
            return Err(IpcError::InvalidRequest);
        }
        let provider = match self.provider {
            AiProvider::Ollama => LlmProviderConfig::Ollama {
                endpoint: self.endpoint.clone(),
                model: self.model.clone(),
            },
            AiProvider::Byok => LlmProviderConfig::openai_compatible(
                &self.endpoint,
                &self.model,
                RedactedApiKey::new(key),
            ),
        };
        triage_ai::llm::validate_llm_endpoint(&provider, false)
            .map_err(|_| IpcError::InvalidRequest)?;
        Ok(LlmClientConfig {
            enabled: self.enabled,
            provider,
            ..Default::default()
        })
    }
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct StoredSettings {
    pub(crate) ai: AiSettings,
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) experimental_semantic_search: bool,
}
impl Drop for StoredSettings {
    fn drop(&mut self) {
        self.api_key.zeroize();
    }
}

impl DesktopService {
    pub(crate) fn read_stored_settings(&self) -> Result<StoredSettings, IpcError> {
        match self.vault.get_secret(&vault_key("ai-settings")?) {
            Ok(bytes) => serde_json::from_slice(&Zeroizing::new(bytes))
                .map_err(|_| IpcError::VaultUnavailable),
            Err(vault::VaultError::NotFound(_)) => Ok(StoredSettings::default()),
            Err(_) => Err(IpcError::VaultUnavailable),
        }
    }

    pub(crate) fn read_ai_settings(&self) -> Result<AiSettings, IpcError> {
        Ok(self.read_stored_settings()?.ai.clone())
    }

    pub(crate) fn read_ai_key(&self) -> Result<Zeroizing<Vec<u8>>, IpcError> {
        Ok(Zeroizing::new(
            self.read_stored_settings()?.api_key.as_bytes().to_vec(),
        ))
    }

    pub async fn get_settings(self: &Arc<Self>) -> Result<SettingsDto, IpcError> {
        self.blocking(|s| {
            let _guard = s.settings_gate.lock().map_err(|_| IpcError::Internal)?;
            let stored = s.read_stored_settings()?;
            Ok(SettingsDto {
                ai: stored.ai.clone(),
                tokenizer: s.store.tokenizer_mode(),
                has_api_key: !stored.api_key.is_empty(),
                model_cached: crate::model_download::model_config()
                    .map(|c| triage_ai::embeddings::ModelCacheManager::new(c).is_model_cached())
                    .unwrap_or(false),
                experimental_semantic_search: stored.experimental_semantic_search,
            })
        })
        .await
    }

    pub async fn save_settings(
        self: &Arc<Self>,
        settings: AiSettings,
        api_key: IncomingApiKey,
    ) -> Result<(), IpcError> {
        self.blocking(move |s| {
            let _guard = s.settings_gate.lock().map_err(|_| IpcError::Internal)?;
            let old = s.read_stored_settings()?;
            let old_key = s.read_ai_key()?;
            // INVARIANT: A retained bearer key may only be reused at its original endpoint.
            let same = old.ai.endpoint == settings.endpoint && old.ai.provider == settings.provider;
            let key = if settings.provider == AiProvider::Ollama {
                Vec::new()
            } else if api_key.0.is_empty() && same {
                old_key.to_vec()
            } else {
                api_key.0.as_bytes().to_vec()
            };
            let key = Zeroizing::new(key);
            if key.len() > 8192 {
                return Err(IpcError::InvalidRequest);
            }
            // Disabled BYOK may be configured without a key; validation still checks the URL.
            let validation_key = if key.is_empty() && !settings.enabled {
                "validation-only".into()
            } else {
                String::from_utf8(key.to_vec()).map_err(|_| IpcError::InvalidRequest)?
            };
            settings.client_config(validation_key)?;
            // INVARIANT: Configuration and its endpoint-bound key are one atomic vault value.
            // A crash cannot associate a new key with an older destination.
            let stored = StoredSettings {
                ai: settings,
                api_key: String::from_utf8(key.to_vec()).map_err(|_| IpcError::InvalidRequest)?,
                experimental_semantic_search: old.experimental_semantic_search,
            };
            let bytes =
                Zeroizing::new(serde_json::to_vec(&stored).map_err(|_| IpcError::Internal)?);
            s.vault
                .set_secret(&vault_key("ai-settings")?, &bytes)
                .map_err(|_| IpcError::VaultUnavailable)?;
            Ok(())
        })
        .await
    }

    pub async fn set_experimental_semantic_search(
        self: &Arc<Self>,
        enabled: bool,
    ) -> Result<(), IpcError> {
        self.blocking(move |s| {
            let _guard = s.settings_gate.lock().map_err(|_| IpcError::Internal)?;
            let mut stored = s.read_stored_settings()?;
            stored.experimental_semantic_search = enabled;
            let bytes =
                Zeroizing::new(serde_json::to_vec(&stored).map_err(|_| IpcError::Internal)?);
            s.vault
                .set_secret(&vault_key("ai-settings")?, &bytes)
                .map_err(|_| IpcError::VaultUnavailable)?;
            Ok(())
        })
        .await
    }

    pub async fn set_tokenizer(
        self: &Arc<Self>,
        tokenizer: FtsTokenizerMode,
    ) -> Result<(), IpcError> {
        self.blocking(move |s| {
            s.store
                .set_tokenizer(tokenizer)
                .map_err(|_| IpcError::StorageUnavailable)
        })
        .await
    }

    pub async fn summarize_message(
        self: &Arc<Self>,
        id: String,
        consent: bool,
        expected: AiSettings,
    ) -> Result<String, IpcError> {
        if !consent {
            return Err(IpcError::InvalidRequest);
        }
        let (config, subject, body) = self
            .blocking(move |s| {
                let _guard = s.settings_gate.lock().map_err(|_| IpcError::Internal)?;
                let settings = s.read_ai_settings()?;
                if !settings.enabled || settings != expected {
                    return Err(IpcError::InvalidRequest);
                }
                let key = s.read_ai_key()?;
                let key =
                    String::from_utf8(key.to_vec()).map_err(|_| IpcError::VaultUnavailable)?;
                let config = settings.client_config(key)?;
                let message = s
                    .store
                    .get_message_by_id(&id)
                    .map_err(|_| IpcError::StorageUnavailable)?
                    .ok_or(IpcError::NotFound)?;
                Ok((
                    config,
                    message.subject,
                    message.text_body.unwrap_or_default(),
                ))
            })
            .await?;
        // INVARIANT: Only an explicitly requested stored message reaches the configured endpoint.
        triage_ai::llm::HttpLlmSummarizer::new(config)
            .map_err(|_| IpcError::InvalidRequest)?
            .summarize_async(&subject, &body)
            .await
            .map_err(|_| IpcError::RemoteUnavailable)
    }
}
