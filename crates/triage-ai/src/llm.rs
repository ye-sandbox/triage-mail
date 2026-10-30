//! Tier 3 Optional Generative LLM Summarizer client supporting local Ollama (`http://127.0.0.1:11434`)
//! and explicit user-configured BYOK OpenAI-compatible HTTP endpoints.
//!
//! INVARIANT: Tier 3 summarization is strictly opt-in (`enabled == true`). Zero unsolicited
//! background LLM calls are ever performed during email sync or deterministic triage.
//! INVARIANT: Local Ollama endpoints MUST bind to loopback (`127.0.0.1`, `::1`, or `localhost`).
//! Remote BYOK endpoints MUST use `https` with a public DNS domain name and redirects disabled
//! to prevent SSRF and cleartext token/email leakage.
//! INVARIANT: API keys (`RedactedApiKey`) and raw email bodies MUST NEVER appear in `Debug`,
//! `Display`, or `TriageError` outputs.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::{Host, Url};

use crate::{LlmSummarizer, TriageError};

/// Default loopback endpoint for a local Ollama server.
pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://127.0.0.1:11434";

/// Default HTTP request timeout for LLM summarization (15 seconds).
pub const DEFAULT_LLM_TIMEOUT_SECS: u64 = 15;

/// Default maximum number of Unicode characters sent to the LLM per summarization call.
pub const DEFAULT_MAX_INPUT_CHARS: usize = 8_000;

/// System instruction governing concise, factual email triage summarization.
pub const DEFAULT_SUMMARY_SYSTEM_PROMPT: &str = "You are a local-first email triage assistant. \
Summarize the email concisely in 2 to 3 bullet points focusing on key actions, dates, amounts, \
and sender intent. Do not invent facts or include external links.";

/// Wrapper around a BYOK bearer API key that redacts its contents in `Debug` and `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedApiKey(String);

impl RedactedApiKey {
    /// Wraps a raw API key string.
    pub fn new(secret: impl Into<String>) -> Self {
        Self(secret.into())
    }

    /// Returns `true` if the underlying secret is empty or whitespace-only.
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    /// Exposes the raw secret string strictly for attaching the `Authorization` header.
    pub(crate) fn expose_secret(&self) -> &str {
        self.0.trim()
    }
}

impl fmt::Debug for RedactedApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"[REDACTED]\"")
    }
}

impl fmt::Display for RedactedApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Provider configuration for Tier 3 LLM summarization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProviderConfig {
    /// Local Ollama instance (must resolve to loopback `127.0.0.1`, `::1`, or `localhost`).
    Ollama { endpoint: String, model: String },
    /// Explicit BYOK OpenAI-compatible chat completions endpoint (`https` required in production).
    OpenAiCompatible {
        endpoint: String,
        model: String,
        api_key: RedactedApiKey,
    },
}

impl LlmProviderConfig {
    /// Constructs a local Ollama configuration using [`DEFAULT_OLLAMA_ENDPOINT`].
    pub fn ollama_default(model: impl Into<String>) -> Self {
        Self::Ollama {
            endpoint: DEFAULT_OLLAMA_ENDPOINT.to_string(),
            model: model.into(),
        }
    }

    /// Constructs a BYOK OpenAI-compatible provider configuration.
    pub fn openai_compatible(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: RedactedApiKey,
    ) -> Self {
        Self::OpenAiCompatible {
            endpoint: endpoint.into(),
            model: model.into(),
            api_key,
        }
    }
}

/// Runtime policy and transport settings for [`HttpLlmSummarizer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmClientConfig {
    /// Explicit user opt-in flag. Defaults to `false`.
    pub enabled: bool,
    /// Target LLM provider (local Ollama or BYOK OpenAI-compatible).
    pub provider: LlmProviderConfig,
    /// Per-request HTTP timeout.
    pub timeout: Duration,
    /// Maximum Unicode scalar values from the email subject + body sent to the model.
    pub max_input_chars: usize,
    /// Strictly for local loopback HTTP mock testing of `OpenAiCompatible`. Must be `false` in production.
    pub allow_loopback_http_for_byok_tests: bool,
}

impl LlmClientConfig {
    /// Creates an explicitly disabled default configuration for local Ollama (`llama3.2`).
    pub fn disabled_default() -> Self {
        Self {
            enabled: false,
            provider: LlmProviderConfig::ollama_default("llama3.2"),
            timeout: Duration::from_secs(DEFAULT_LLM_TIMEOUT_SECS),
            max_input_chars: DEFAULT_MAX_INPUT_CHARS,
            allow_loopback_http_for_byok_tests: false,
        }
    }
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self::disabled_default()
    }
}

/// Returns `true` if the parsed URL host is strictly loopback (`localhost`, `127.0.0.0/8`, `::1`).
fn is_loopback_host(host: Option<Host<&str>>) -> bool {
    match host {
        Some(Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// Validates and normalizes an Ollama loopback endpoint URL.
fn validate_ollama_url(mut parsed: Url, model: &str) -> Result<Url, TriageError> {
    if model.trim().is_empty() {
        return Err(TriageError::Llm(
            "Ollama model identifier must not be empty".to_string(),
        ));
    }
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(TriageError::Llm(format!(
            "unsupported Ollama URL scheme `{}`",
            parsed.scheme()
        )));
    }
    if !is_loopback_host(parsed.host()) {
        return Err(TriageError::Llm(
            "local Ollama endpoint must target a loopback host (127.0.0.1, ::1, or localhost)"
                .to_string(),
        ));
    }
    if parsed.path().is_empty() || parsed.path() == "/" {
        parsed.set_path("/api/generate");
    }
    Ok(parsed)
}

/// Validates a public domain name for BYOK HTTPS endpoints against SSRF and internal suffixes.
fn validate_public_byok_domain(domain: &str) -> Result<(), TriageError> {
    let lower = domain.trim().to_ascii_lowercase();
    if lower == "localhost"
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || !lower.contains('.')
    {
        return Err(TriageError::Llm(format!(
            "rejected non-public BYOK domain `{domain}`"
        )));
    }
    if lower
        .split('.')
        .any(|label| label.is_empty() || label.starts_with('-') || label.ends_with('-'))
    {
        return Err(TriageError::Llm(format!(
            "malformed domain labels in `{domain}`"
        )));
    }
    Ok(())
}

/// Normalizes the path of an OpenAI-compatible endpoint to `/v1/chat/completions` when omitted.
fn normalize_openai_path(parsed: &mut Url) {
    let path = parsed.path().trim_end_matches('/');
    if path.is_empty() {
        parsed.set_path("/v1/chat/completions");
    } else if path.ends_with("/v1") {
        let new_path = format!("{path}/chat/completions");
        parsed.set_path(&new_path);
    }
}

/// Validates and normalizes a BYOK OpenAI-compatible endpoint URL.
fn validate_byok_url(
    mut parsed: Url,
    model: &str,
    api_key: &RedactedApiKey,
    allow_loopback_http_for_byok_tests: bool,
) -> Result<Url, TriageError> {
    if model.trim().is_empty() {
        return Err(TriageError::Llm(
            "BYOK model identifier must not be empty".to_string(),
        ));
    }
    if api_key.is_empty() {
        return Err(TriageError::Llm(
            "BYOK API key must not be empty".to_string(),
        ));
    }

    if allow_loopback_http_for_byok_tests && is_loopback_host(parsed.host()) {
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(TriageError::Llm(format!(
                "unsupported BYOK URL scheme `{}`",
                parsed.scheme()
            )));
        }
        normalize_openai_path(&mut parsed);
        return Ok(parsed);
    }

    if parsed.scheme() != "https" {
        return Err(TriageError::Llm(format!(
            "BYOK endpoint requires `https` scheme, got `{}`",
            parsed.scheme()
        )));
    }

    match parsed.host() {
        Some(Host::Domain(domain)) => validate_public_byok_domain(domain)?,
        Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => {
            return Err(TriageError::Llm(
                "BYOK endpoint must use a public DNS domain name, not an IP literal".to_string(),
            ))
        }
        None => {
            return Err(TriageError::Llm(
                "BYOK endpoint URL is missing a host".to_string(),
            ))
        }
    }

    normalize_openai_path(&mut parsed);
    Ok(parsed)
}

/// Validates an [`LlmProviderConfig`] against SSRF, scheme, credential, and opt-in safety rules.
pub fn validate_llm_endpoint(
    provider: &LlmProviderConfig,
    allow_loopback_http_for_byok_tests: bool,
) -> Result<Url, TriageError> {
    let raw_url = match provider {
        LlmProviderConfig::Ollama { endpoint, .. } => endpoint.trim(),
        LlmProviderConfig::OpenAiCompatible { endpoint, .. } => endpoint.trim(),
    };

    let parsed = Url::parse(raw_url)
        .map_err(|e| TriageError::Llm(format!("invalid LLM endpoint URL: {e}")))?;

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(TriageError::Llm(
            "LLM endpoint URL must not contain embedded credentials".to_string(),
        ));
    }

    match provider {
        LlmProviderConfig::Ollama { model, .. } => validate_ollama_url(parsed, model),
        LlmProviderConfig::OpenAiCompatible { model, api_key, .. } => {
            validate_byok_url(parsed, model, api_key, allow_loopback_http_for_byok_tests)
        }
    }
}

/// Truncates `input` to at most `max_chars` Unicode scalar values on a valid UTF-8 boundary.
pub fn truncate_utf8_chars(input: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }
    match input.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &input[..byte_idx],
        None => input,
    }
}

/// Builds a bounded user prompt from `subject` and `body_text` respecting `max_input_chars`.
pub fn build_user_prompt(
    subject: &str,
    body_text: &str,
    max_input_chars: usize,
) -> Result<String, TriageError> {
    let clean_subject = subject.trim();
    let clean_body = body_text.trim();
    if clean_subject.is_empty() && clean_body.is_empty() {
        return Err(TriageError::Llm(
            "cannot summarize an email with both empty subject and empty body".to_string(),
        ));
    }
    if max_input_chars == 0 {
        return Err(TriageError::Llm(
            "max_input_chars must be greater than zero".to_string(),
        ));
    }

    let formatted = format!("Subject: {clean_subject}\n\nBody:\n{clean_body}");
    Ok(truncate_utf8_chars(&formatted, max_input_chars).to_string())
}

#[derive(Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    system: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: [OpenAiMessage<'a>; 2],
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
}

/// Maps transport errors into PII-safe and secret-safe `TriageError::Llm` descriptions.
fn map_reqwest_error(err: &reqwest::Error) -> TriageError {
    if err.is_timeout() {
        TriageError::Llm("LLM summarization request timed out".to_string())
    } else if err.is_connect() {
        TriageError::Llm("failed connecting to configured LLM endpoint".to_string())
    } else {
        TriageError::Llm("HTTP transport error during LLM summarization".to_string())
    }
}

/// HTTP client implementing [`LlmSummarizer`] for local Ollama and BYOK OpenAI-compatible APIs.
pub struct HttpLlmSummarizer {
    config: LlmClientConfig,
    target_url: Url,
    http: reqwest::Client,
}

impl fmt::Debug for HttpLlmSummarizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpLlmSummarizer")
            .field("config", &self.config)
            .field("target_url", &self.target_url.as_str())
            .finish()
    }
}

impl HttpLlmSummarizer {
    /// Creates a validated [`HttpLlmSummarizer`] with redirects disabled and strict timeouts.
    pub fn new(config: LlmClientConfig) -> Result<Self, TriageError> {
        let target_url =
            validate_llm_endpoint(&config.provider, config.allow_loopback_http_for_byok_tests)?;

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|_| TriageError::Llm("failed initializing LLM HTTP client".to_string()))?;

        Ok(Self {
            config,
            target_url,
            http,
        })
    }

    /// Asynchronously summarizes `subject` and `body_text` if `config.enabled` is `true`.
    pub async fn summarize_async(
        &self,
        subject: &str,
        body_text: &str,
    ) -> Result<String, TriageError> {
        if !self.config.enabled {
            return Err(TriageError::Llm(
                "Tier 3 LLM summarization is disabled; explicit user opt-in is required"
                    .to_string(),
            ));
        }

        let user_prompt = build_user_prompt(subject, body_text, self.config.max_input_chars)?;

        match &self.config.provider {
            LlmProviderConfig::Ollama { model, .. } => {
                self.execute_ollama(model.trim(), &user_prompt).await
            }
            LlmProviderConfig::OpenAiCompatible { model, api_key, .. } => {
                self.execute_openai(model.trim(), api_key, &user_prompt)
                    .await
            }
        }
    }

    async fn execute_ollama(&self, model: &str, prompt: &str) -> Result<String, TriageError> {
        let payload = OllamaGenerateRequest {
            model,
            system: DEFAULT_SUMMARY_SYSTEM_PROMPT,
            prompt,
            stream: false,
        };
        let body_bytes = serde_json::to_vec(&payload)
            .map_err(|_| TriageError::Llm("failed serializing Ollama request".to_string()))?;

        let response = self
            .http
            .post(self.target_url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| map_reqwest_error(&e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(TriageError::Llm(format!(
                "Ollama endpoint returned HTTP {}",
                status.as_u16()
            )));
        }

        let bytes = response.bytes().await.map_err(|e| map_reqwest_error(&e))?;
        let parsed: OllamaGenerateResponse = serde_json::from_slice(&bytes)
            .map_err(|_| TriageError::Llm("malformed JSON response from Ollama".to_string()))?;

        let trimmed = parsed.response.trim().to_string();
        if trimmed.is_empty() {
            return Err(TriageError::Llm(
                "Ollama returned an empty summary response".to_string(),
            ));
        }
        Ok(trimmed)
    }

    async fn execute_openai(
        &self,
        model: &str,
        api_key: &RedactedApiKey,
        prompt: &str,
    ) -> Result<String, TriageError> {
        let payload = OpenAiChatRequest {
            model,
            messages: [
                OpenAiMessage {
                    role: "system",
                    content: DEFAULT_SUMMARY_SYSTEM_PROMPT,
                },
                OpenAiMessage {
                    role: "user",
                    content: prompt,
                },
            ],
            temperature: 0.2,
        };
        let body_bytes = serde_json::to_vec(&payload)
            .map_err(|_| TriageError::Llm("failed serializing BYOK request".to_string()))?;

        let auth_value = format!("Bearer {}", api_key.expose_secret());
        let response = self
            .http
            .post(self.target_url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::AUTHORIZATION, auth_value)
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| map_reqwest_error(&e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(TriageError::Llm(format!(
                "BYOK LLM endpoint returned HTTP {}",
                status.as_u16()
            )));
        }

        let bytes = response.bytes().await.map_err(|e| map_reqwest_error(&e))?;
        let parsed: OpenAiChatResponse = serde_json::from_slice(&bytes).map_err(|_| {
            TriageError::Llm("malformed JSON response from BYOK endpoint".to_string())
        })?;

        let summary = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                TriageError::Llm("BYOK endpoint returned an empty summary choice".to_string())
            })?;

        Ok(summary)
    }
}

impl LlmSummarizer for HttpLlmSummarizer {
    fn summarize(&self, subject: &str, body_text: &str) -> Result<String, TriageError> {
        // INVARIANT: Execute async request on an isolated scoped single-thread Tokio runtime so
        // synchronous `LlmSummarizer::summarize` callers never panic even when invoked inside an
        // existing outer Tokio runtime.
        std::thread::scope(|s| {
            s.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| {
                        TriageError::Llm("failed building local async runtime".to_string())
                    })?;
                rt.block_on(self.summarize_async(subject, body_text))
            })
            .join()
            .map_err(|_| TriageError::Llm("LLM summarizer worker thread panicked".to_string()))?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn redacted_api_key_never_leaks_in_debug_or_display() {
        let raw_secret = "sk-synthetic-test-secret-000000";
        let key = RedactedApiKey::new(raw_secret);
        let cfg = LlmClientConfig {
            enabled: true,
            provider: LlmProviderConfig::openai_compatible(
                "https://llm.example.com/v1/chat/completions",
                "gpt-4o-mini",
                key.clone(),
            ),
            timeout: Duration::from_secs(5),
            max_input_chars: 1000,
            allow_loopback_http_for_byok_tests: false,
        };

        assert_eq!(format!("{key}"), "[REDACTED]");
        assert_eq!(format!("{key:?}"), "\"[REDACTED]\"");
        let debug_cfg = format!("{cfg:?}");
        assert!(!debug_cfg.contains(raw_secret));
        assert!(debug_cfg.contains("[REDACTED]"));
    }

    #[test]
    fn enforces_opt_in_policy_when_disabled() {
        let cfg = LlmClientConfig::disabled_default();
        assert!(!cfg.enabled);
        let summarizer = HttpLlmSummarizer::new(cfg).unwrap();
        let err = summarizer
            .summarize("Invoice renewal", "Your subscription renews on 2026-11-15.")
            .unwrap_err();
        assert_eq!(
            err,
            TriageError::Llm(
                "Tier 3 LLM summarization is disabled; explicit user opt-in is required"
                    .to_string()
            )
        );
    }

    #[test]
    fn validates_ollama_loopback_and_rejects_remote_or_credential_urls() {
        let valid_ipv4 = LlmProviderConfig::ollama_default("llama3.2");
        let url = validate_llm_endpoint(&valid_ipv4, false).unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:11434/api/generate");

        let valid_ipv6 = LlmProviderConfig::Ollama {
            endpoint: "http://[::1]:11434".to_string(),
            model: "qwen2.5:3b".to_string(),
        };
        assert!(validate_llm_endpoint(&valid_ipv6, false).is_ok());

        let valid_localhost = LlmProviderConfig::Ollama {
            endpoint: "http://localhost:11434/api/generate".to_string(),
            model: "mistral".to_string(),
        };
        assert!(validate_llm_endpoint(&valid_localhost, false).is_ok());

        // Reject non-loopback Ollama endpoints
        let remote_ollama = LlmProviderConfig::Ollama {
            endpoint: "http://192.168.1.50:11434".to_string(),
            model: "llama3.2".to_string(),
        };
        assert!(validate_llm_endpoint(&remote_ollama, false).is_err());

        let domain_ollama = LlmProviderConfig::Ollama {
            endpoint: "https://ollama.example.com".to_string(),
            model: "llama3.2".to_string(),
        };
        assert!(validate_llm_endpoint(&domain_ollama, false).is_err());
    }

    #[test]
    fn validates_byok_https_and_rejects_insecure_http_or_ssrf_hosts() {
        let key = RedactedApiKey::new("sk-synthetic-token-123");

        let valid = LlmProviderConfig::openai_compatible(
            "https://api.example.com/v1",
            "gpt-4o-mini",
            key.clone(),
        );
        let normalized = validate_llm_endpoint(&valid, false).unwrap();
        assert_eq!(
            normalized.as_str(),
            "https://api.example.com/v1/chat/completions"
        );

        // Reject plain HTTP for remote BYOK
        let insecure_http = LlmProviderConfig::openai_compatible(
            "http://api.example.com/v1/chat/completions",
            "gpt-4o-mini",
            key.clone(),
        );
        assert!(validate_llm_endpoint(&insecure_http, false).is_err());

        // Reject loopback or private IP literals in production mode
        let loopback_byok = LlmProviderConfig::openai_compatible(
            "https://127.0.0.1:8443/v1/chat/completions",
            "gpt-4o-mini",
            key.clone(),
        );
        assert!(validate_llm_endpoint(&loopback_byok, false).is_err());

        let metadata_ssrf = LlmProviderConfig::openai_compatible(
            "https://169.254.169.254/latest/meta-data",
            "gpt-4o-mini",
            key.clone(),
        );
        assert!(validate_llm_endpoint(&metadata_ssrf, false).is_err());

        // Reject internal/local domains and embedded credentials
        let internal_domain = LlmProviderConfig::openai_compatible(
            "https://gateway.internal/v1/chat/completions",
            "gpt-4o-mini",
            key.clone(),
        );
        assert!(validate_llm_endpoint(&internal_domain, false).is_err());

        let embedded_creds = LlmProviderConfig::openai_compatible(
            "https://admin:secret@api.example.com/v1/chat/completions",
            "gpt-4o-mini",
            key,
        );
        assert!(validate_llm_endpoint(&embedded_creds, false).is_err());
    }

    #[test]
    fn truncates_utf8_prompt_safely_on_character_boundaries() {
        let subject = "Notificação de Fatura Cartão";
        let body = "Olá! Sua fatura no valor de R$ 1.250,00 vence em 15/11/2026. Pagamento via Pix disponível.";
        let prompt = build_user_prompt(subject, body, 42).unwrap();
        assert_eq!(prompt.chars().count(), 42);
        assert!(prompt.starts_with("Subject: Notificação"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn summarizes_via_local_ollama_mock_http_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let raw_req = String::from_utf8_lossy(&buf[..n]).to_string();

            assert!(raw_req.starts_with("POST /api/generate HTTP/1.1\r\n"));
            assert!(raw_req.contains("\"model\":\"llama3.2\""));
            assert!(raw_req.contains("\"stream\":false"));
            assert!(raw_req.contains("Boleto Condominio"));

            let json_body = r#"{"model":"llama3.2","response":"- Condo invoice due 2026-11-10 for R$ 850.00.","done":true}"#;
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        });

        let summarizer = HttpLlmSummarizer::new(LlmClientConfig {
            enabled: true,
            provider: LlmProviderConfig::Ollama {
                endpoint: format!("http://{}", addr),
                model: "llama3.2".to_string(),
            },
            timeout: Duration::from_secs(5),
            max_input_chars: 2000,
            allow_loopback_http_for_byok_tests: false,
        })
        .unwrap();

        let summary = summarizer
            .summarize(
                "Boleto Condominio",
                "Prezado morador, segue boleto no valor de R$ 850,00 com vencimento em 2026-11-10.",
            )
            .unwrap();

        assert_eq!(summary, "- Condo invoice due 2026-11-10 for R$ 850.00.");
        server_task.await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn summarizes_via_byok_openai_compatible_mock_http_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let raw_req = String::from_utf8_lossy(&buf[..n]).to_string();

            assert!(raw_req.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
            assert!(raw_req
                .to_ascii_lowercase()
                .contains("authorization: bearer sk-synthetic-byok-999\r\n"));
            assert!(raw_req.contains("\"model\":\"gpt-4o-mini\""));
            assert!(raw_req.contains("Server Maintenance Window"));

            let json_body = r#"{"choices":[{"message":{"role":"assistant","content":"- Scheduled maintenance on 2026-11-20 from 02:00 to 04:00 UTC."}}]}"#;
            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        });

        let summarizer = HttpLlmSummarizer::new(LlmClientConfig {
            enabled: true,
            provider: LlmProviderConfig::openai_compatible(
                format!("http://{}/v1", addr),
                "gpt-4o-mini",
                RedactedApiKey::new("sk-synthetic-byok-999"),
            ),
            timeout: Duration::from_secs(5),
            max_input_chars: 2000,
            allow_loopback_http_for_byok_tests: true,
        })
        .unwrap();

        let summary = summarizer
            .summarize(
                "Server Maintenance Window",
                "Maintenance will occur on 2026-11-20 between 02:00 and 04:00 UTC.",
            )
            .unwrap();

        assert_eq!(
            summary,
            "- Scheduled maintenance on 2026-11-20 from 02:00 to 04:00 UTC."
        );
        server_task.await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn enforces_timeout_and_redacts_pii_on_slow_or_failing_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(400)).await;
        });

        let secret = "sk-synthetic-never-leak-token";
        let sensitive_body = "CONFIDENTIAL-ACCOUNT-NUMBER-99887766";
        let summarizer = HttpLlmSummarizer::new(LlmClientConfig {
            enabled: true,
            provider: LlmProviderConfig::openai_compatible(
                format!("http://{}", addr),
                "gpt-4o-mini",
                RedactedApiKey::new(secret),
            ),
            timeout: Duration::from_millis(80),
            max_input_chars: 1000,
            allow_loopback_http_for_byok_tests: true,
        })
        .unwrap();

        let err = summarizer
            .summarize("Confidential Alert", sensitive_body)
            .unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("timed out"));
        assert!(!err_msg.contains(secret));
        assert!(!err_msg.contains(sensitive_body));
        server_task.await.unwrap();
    }
}
