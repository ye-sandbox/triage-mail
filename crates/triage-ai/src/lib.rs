#![forbid(unsafe_code)]

//! 3-Tier Local-First Triage & AI Pipeline contracts:
//! - Tier 1 (Deterministic, Zero Cost): Regex/heuristics for 2FA/OTP, FEBRABAN Boletos (exact BRL
//!   amount + due date), Pix EMV/keys, NF-e metadata, package tracking, and RFC 8058 One-Click Unsubscribe.
//! - Tier 2 (Local CPU Embeddings): Quantized ONNX embeddings (`all-MiniLM-L6-v2` / `bge-small`)
//!   cached on demand in the user's local data directory (`$XDG_DATA_HOME/triage-mail/models`).
//! - Tier 3 (Optional Generative LLM): Local Ollama (`http://127.0.0.1:11434`) or explicit BYOK.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
}
