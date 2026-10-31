#![forbid(unsafe_code)]

//! 3-Tier Local-First Triage & AI Pipeline contracts:
//! - Tier 1 (Deterministic, Zero Cost): Regex/heuristics for 2FA/OTP, FEBRABAN Boletos (exact BRL
//!   amount + due date), Pix EMV/keys, NF-e metadata, package tracking, and RFC 8058 One-Click Unsubscribe.
//! - Tier 2 (Experimental Local CPU Semantic Projection): Deterministic token/n-gram feature-hash
//!   projection (`int8[384]`) seeded from cached model headers or offline fallback.
//!   NOTE: v0.1 does not execute neural ONNX weights; true learned ONNX inference (`ort` / `fastembed-rs`)
//!   is scheduled for v0.2.
//! - Tier 3 (Optional Generative LLM): Local Ollama (`http://127.0.0.1:11434`) or explicit BYOK.

pub mod boleto;
pub mod classifier;
pub mod embeddings;
pub mod llm;
pub mod nfe;
pub mod otp;
pub mod pix;
pub mod tracking;

use std::path::PathBuf;

pub use boleto::{extract_boletos, modulo10, modulo11_barcode, BoletoDetails};
pub use classifier::classify_message;
pub use embeddings::{
    cosine_similarity_int8, l2_normalize, quantize_f32_to_int8, resolve_xdg_data_dir,
    validate_model_name, BackgroundEmbeddingIndexer, EmbeddingDocument, IndexedEmbedding,
    LocalCpuEmbeddingProvider, ModelCacheManager, ModelFetcher, DEFAULT_EMBEDDING_DIMENSION,
    MODEL_ALL_MINILM_L6_V2, MODEL_BGE_SMALL_EN_V1_5, REQUIRED_MODEL_FILES,
};
pub use llm::{
    build_user_prompt, truncate_utf8_chars, validate_llm_endpoint, HttpLlmSummarizer,
    LlmClientConfig, LlmProviderConfig, RedactedApiKey, DEFAULT_LLM_TIMEOUT_SECS,
    DEFAULT_MAX_INPUT_CHARS, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_SUMMARY_SYSTEM_PROMPT,
};
pub use nfe::extract_nfe_keys;
pub use otp::extract_otp;
pub use pix::{compute_crc16_ccitt_false, extract_pix_payload};
use serde::{Deserialize, Serialize};
use thiserror::Error;
pub use tracking::{extract_tracking_codes, TrackingDetails};

/// Errors returned by deterministic extractors, ONNX embedding providers, or optional LLM clients.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TriageError {
    #[error("embedding model error: {0}")]
    Embedding(String),
    #[error("LLM summarization error: {0}")]
    Llm(String),
    #[error("post-triage hook error: {0}")]
    Hook(String),
}

/// High-level triage category assigned to an inbound message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriageCategory {
    Security,
    Financial,
    Shopping,
    Newsletter,
    Personal,
}

/// Structured badges extracted deterministically in Tier 1 (zero LLM cost).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriageBadge {
    /// 2FA / OTP verification code (`code`).
    OtpCode { code: String },
    /// Brazilian FEBRABAN Boleto with deterministic BRL value (`amount_cents`) and ISO due date (`YYYY-MM-DD`).
    FebrabanBoleto {
        digitable_line: String,
        amount_cents: Option<u64>,
        due_date_iso: Option<String>,
    },
    /// Brazilian Pix Copy-and-Paste EMV payload or Pix key.
    PixPayload { payload: String },
    /// Brazilian Electronic Invoice (NF-e / NFS-e) access key or number.
    NfeInvoice { access_key: String },
    /// Logistics / postal package tracking code.
    TrackingCode { carrier: String, code: String },
    /// Unsubscribe capability (`supports_one_click` indicates RFC 8058 HTTP POST support).
    UnsubscribeAvailable { supports_one_click: bool },
}

/// Output of Tier 1 deterministic analysis for a single message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicTriageResult {
    pub category: TriageCategory,
    pub badges: Vec<TriageBadge>,
}

/// On-demand cache configuration for Tier 2 local CPU ONNX embedding models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingModelConfig {
    pub model_name: String,
    pub cache_dir: PathBuf,
}

impl EmbeddingModelConfig {
    /// Creates a model cache configuration rooted at the specified user data directory.
    pub fn with_data_dir(model_name: impl Into<String>, data_dir: PathBuf) -> Self {
        Self {
            model_name: model_name.into(),
            cache_dir: data_dir.join("models"),
        }
    }
}

/// Tier 1 deterministic extractor contract.
pub trait DeterministicTriage: Send + Sync {
    /// Evaluates subject, sender, and body text in sub-millisecond time without external calls.
    fn analyze(
        &self,
        subject: &str,
        sender: &str,
        body_text: &str,
        has_rfc8058_one_click: bool,
    ) -> DeterministicTriageResult;
}

/// Default Tier 1 deterministic triage pipeline implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultDeterministicTriage;

impl DefaultDeterministicTriage {
    pub fn new() -> Self {
        Self
    }
}

impl DeterministicTriage for DefaultDeterministicTriage {
    fn analyze(
        &self,
        subject: &str,
        sender: &str,
        body_text: &str,
        has_rfc8058_one_click: bool,
    ) -> DeterministicTriageResult {
        let mut badges = Vec::new();

        // 1. Surface RFC 8058 / Unsubscribe badge
        if has_rfc8058_one_click {
            badges.push(TriageBadge::UnsubscribeAvailable {
                supports_one_click: true,
            });
        }

        // 2. Extract 2FA / OTP
        if let Some(code) = extract_otp(subject, body_text) {
            badges.push(TriageBadge::OtpCode { code });
        }

        // 3. Extract FEBRABAN Boletos
        let combined_text = format!("{subject}\n{body_text}");
        for boleto in extract_boletos(&combined_text) {
            badges.push(TriageBadge::FebrabanBoleto {
                digitable_line: boleto.digitable_line,
                amount_cents: boleto.amount_cents,
                due_date_iso: boleto.due_date_iso,
            });
        }

        // 4. Extract Pix payloads / keys
        if let Some(payload) = extract_pix_payload(&combined_text) {
            badges.push(TriageBadge::PixPayload { payload });
        }

        // 5. Extract NF-e access keys
        for access_key in extract_nfe_keys(&combined_text) {
            badges.push(TriageBadge::NfeInvoice { access_key });
        }

        // 6. Extract Tracking codes
        for tracking in extract_tracking_codes(&combined_text) {
            badges.push(TriageBadge::TrackingCode {
                carrier: tracking.carrier,
                code: tracking.code,
            });
        }

        // 7. Deterministic category classification
        let category = classify_message(subject, sender, body_text, &badges, has_rfc8058_one_click);

        DeterministicTriageResult { category, badges }
    }
}

/// Tier 2 local CPU ONNX embedding provider contract (`int8` quantized vector output for `sqlite-vec`).
pub trait EmbeddingProvider: Send + Sync {
    /// Generates a scalar-quantized `int8` embedding vector for semantic indexing or query matching.
    fn embed_int8(&self, text: &str) -> Result<Vec<i8>, TriageError>;
}

/// Tier 3 optional generative LLM summarizer contract (Local Ollama or BYOK).
pub trait LlmSummarizer: Send + Sync {
    /// Summarizes a thread or message body on explicit user request.
    fn summarize(&self, subject: &str, body_text: &str) -> Result<String, TriageError>;
}

/// Clean post-triage observer hook for UI notifications and future local/webhook automations.
pub trait TriageEventHook: Send + Sync {
    /// Invoked after a message is classified and badges are extracted.
    fn on_message_triaged(
        &self,
        message_id: &str,
        result: &DeterministicTriageResult,
    ) -> Result<(), TriageError>;
}

/// Default no-op post-triage observer implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultTriageEventHook;

impl TriageEventHook for DefaultTriageEventHook {
    fn on_message_triaged(
        &self,
        _message_id: &str,
        _result: &DeterministicTriageResult,
    ) -> Result<(), TriageError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_on_demand_model_cache_path() {
        let cfg = EmbeddingModelConfig::with_data_dir(
            "all-MiniLM-L6-v2",
            PathBuf::from("/tmp/triage-mail-test"),
        );
        assert_eq!(cfg.model_name, "all-MiniLM-L6-v2");
        assert_eq!(cfg.cache_dir, PathBuf::from("/tmp/triage-mail-test/models"));
    }

    #[test]
    fn default_triage_pipeline_extracts_badges_and_classifies_security() {
        let triage = DefaultDeterministicTriage::new();
        let result = triage.analyze(
            "Código de verificação para login",
            "security@example.com",
            "Olá! Use o código de verificação: 582914 para autenticar seu dispositivo.",
            false,
        );

        assert_eq!(result.category, TriageCategory::Security);
        assert_eq!(
            result.badges,
            vec![TriageBadge::OtpCode {
                code: "582914".to_string(),
            }]
        );
    }

    #[test]
    fn default_triage_event_hook_executes_successfully() {
        let hook = DefaultTriageEventHook;
        let triage = DefaultDeterministicTriage::new();
        let result = triage.analyze("Hello", "friend@example.org", "Meeting tomorrow", false);
        assert!(hook.on_message_triaged("msg-123", &result).is_ok());
    }
}
