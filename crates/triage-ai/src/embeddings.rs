//! Tier 2 Experimental Local CPU Semantic Projection (`int8[384]` vector space) with on-demand
//! model artifact caching rooted at `$XDG_DATA_HOME/triage-mail/models` and symmetric `int8` scalar quantization.
//!
//! NOTE: In v0.1, `LocalCpuEmbeddingProvider` computes deterministic token and character n-gram
//! feature-hash projections (FNV1a hash seeded by cached model headers or offline constant), rather
//! than neural ONNX runtime inference. True learned ONNX inference via `ort` / `fastembed-rs` is
//! scheduled for v0.2.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{EmbeddingModelConfig, EmbeddingProvider, TriageError};

/// Target dimensionality for `sqlite-vec` `vec_messages` (`int8[384]`).
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 384;

/// Default quantized sentence-transformer model identifier (`384` dimensions).
pub const MODEL_ALL_MINILM_L6_V2: &str = "all-MiniLM-L6-v2";

/// Alternative BGE small embedding model identifier (`384` dimensions).
pub const MODEL_BGE_SMALL_EN_V1_5: &str = "bge-small-en-v1.5";

/// Required ONNX runtime artifacts inside each model directory.
pub const REQUIRED_MODEL_FILES: [&str; 2] = ["model.onnx", "tokenizer.json"];

/// Pluggable artifact fetcher for lazy on-demand model provisioning.
///
/// INVARIANT: Model weights are never bundled in the compiled binary. They are fetched only
/// on demand into `$XDG_DATA_HOME/triage-mail/models/<model_name>` when missing from cache.
pub trait ModelFetcher: Send + Sync {
    /// Fetches a named artifact (e.g., `model.onnx` or `tokenizer.json`) for `model_name`.
    fn fetch_artifact(&self, model_name: &str, filename: &str) -> Result<Vec<u8>, TriageError>;
}

/// Document input for background batch embedding generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingDocument {
    pub message_id: String,
    pub subject: String,
    pub body_text: String,
}

/// Output record produced by the background batch embedding indexer for `sqlite-vec` persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedEmbedding {
    pub message_id: String,
    pub embedding_int8: Vec<i8>,
}

impl EmbeddingModelConfig {
    /// Resolves `$XDG_DATA_HOME/triage-mail/models` from the current process environment,
    /// falling back to `$HOME/.local/share/triage-mail/models` per the XDG Base Directory spec.
    pub fn from_xdg_env(model_name: impl Into<String>) -> Result<Self, TriageError> {
        let xdg = std::env::var("XDG_DATA_HOME").ok();
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok();
        let base_dir = resolve_xdg_data_dir(xdg.as_deref(), home.as_deref())?;
        Ok(Self::with_data_dir(model_name, base_dir))
    }

    /// Returns the validated directory path for this specific model inside `cache_dir`.
    pub fn model_dir(&self) -> Result<PathBuf, TriageError> {
        validate_model_name(&self.model_name)?;
        Ok(self.cache_dir.join(&self.model_name))
    }
}

/// Resolves the application data directory (`$XDG_DATA_HOME/triage-mail` or `$HOME/.local/share/triage-mail`).
pub fn resolve_xdg_data_dir(
    xdg_data_home: Option<&str>,
    home_dir: Option<&str>,
) -> Result<PathBuf, TriageError> {
    if let Some(xdg) = xdg_data_home.map(str::trim).filter(|s| !s.is_empty()) {
        let path = PathBuf::from(xdg);
        if !path.is_absolute() {
            return Err(TriageError::Embedding(
                "XDG_DATA_HOME must be an absolute path".to_string(),
            ));
        }
        return Ok(path.join("triage-mail"));
    }

    if let Some(home) = home_dir.map(str::trim).filter(|s| !s.is_empty()) {
        let path = PathBuf::from(home);
        if !path.is_absolute() {
            return Err(TriageError::Embedding(
                "HOME must be an absolute path".to_string(),
            ));
        }
        return Ok(path.join(".local").join("share").join("triage-mail"));
    }

    Err(TriageError::Embedding(
        "neither XDG_DATA_HOME nor HOME is set to resolve model cache directory".to_string(),
    ))
}

/// Validates that a model identifier does not contain path traversal segments.
pub fn validate_model_name(model_name: &str) -> Result<(), TriageError> {
    let trimmed = model_name.trim();
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return Err(TriageError::Embedding(format!(
            "invalid model identifier: '{model_name}'"
        )));
    }
    Ok(())
}

/// Manages on-demand model verification and atomic caching inside `$XDG_DATA_HOME/triage-mail/models`.
#[derive(Debug, Clone)]
pub struct ModelCacheManager {
    config: EmbeddingModelConfig,
}

impl ModelCacheManager {
    /// Creates a cache manager for the given model configuration.
    pub fn new(config: EmbeddingModelConfig) -> Self {
        Self { config }
    }

    /// Returns `true` if all required ONNX and tokenizer files are present and non-empty on disk.
    pub fn is_model_cached(&self) -> bool {
        let Ok(dir) = self.config.model_dir() else {
            return false;
        };
        REQUIRED_MODEL_FILES
            .iter()
            .all(|file| is_non_empty_file(&dir.join(file)))
    }

    /// Ensures the model artifacts are present in the local cache directory, invoking `fetcher`
    /// lazily only for missing files.
    pub fn ensure_cached(
        &self,
        fetcher: Option<&dyn ModelFetcher>,
    ) -> Result<PathBuf, TriageError> {
        let model_dir = self.config.model_dir()?;
        if self.is_model_cached() {
            return Ok(model_dir);
        }

        let Some(fetcher) = fetcher else {
            return Err(TriageError::Embedding(format!(
                "model '{}' is not cached in '{}' and no online fetcher is available",
                self.config.model_name,
                model_dir.display()
            )));
        };

        fs::create_dir_all(&model_dir).map_err(|e| {
            TriageError::Embedding(format!(
                "failed to create model cache directory '{}': {e}",
                model_dir.display()
            ))
        })?;

        for filename in REQUIRED_MODEL_FILES {
            let target_path = model_dir.join(filename);
            if !is_non_empty_file(&target_path) {
                let bytes = fetcher.fetch_artifact(&self.config.model_name, filename)?;
                if bytes.is_empty() {
                    return Err(TriageError::Embedding(format!(
                        "downloaded empty artifact '{filename}' for model '{}'",
                        self.config.model_name
                    )));
                }
                write_artifact_atomically(&target_path, &bytes)?;
            }
        }

        Ok(model_dir)
    }
}

fn is_non_empty_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
}

fn write_artifact_atomically(target_path: &Path, bytes: &[u8]) -> Result<(), TriageError> {
    let tmp_path = target_path.with_extension("tmp");
    fs::write(&tmp_path, bytes).map_err(|e| {
        TriageError::Embedding(format!(
            "failed to write temporary model artifact '{}': {e}",
            tmp_path.display()
        ))
    })?;
    fs::rename(&tmp_path, target_path).map_err(|e| {
        TriageError::Embedding(format!(
            "failed to finalize model artifact '{}': {e}",
            target_path.display()
        ))
    })
}

/// Normalizes a floating-point vector to unit L2 length.
pub fn l2_normalize(values: &[f32]) -> Vec<f32> {
    let norm_sq: f32 = values.iter().map(|&v| v * v).sum();
    let norm = norm_sq.sqrt();
    if norm <= f32::EPSILON {
        return vec![0.0; values.len()];
    }
    values.iter().map(|&v| v / norm).collect()
}

/// Quantizes a floating-point vector into symmetric `int8` (`[-127, 127]`) via absmax scaling.
///
/// INVARIANT: `sqlite-vec` evaluates `distance_metric=cosine`, which is scale-invariant for
/// positive factors ($\cos(\alpha x, \beta y) = \cos(x, y)$). Scaling each vector by
/// $127 / \max_i |x_i|$ preserves full 8-bit dynamic range on 384-dim unit vectors without
/// distorting cosine rankings.
pub fn quantize_f32_to_int8(values: &[f32]) -> Vec<i8> {
    let normalized = l2_normalize(values);
    let max_abs = normalized.iter().fold(0.0_f32, |acc, &v| acc.max(v.abs()));

    if max_abs <= f32::EPSILON {
        return vec![0i8; values.len()];
    }

    let scale = 127.0 / max_abs;
    normalized
        .iter()
        .map(|&v| (v * scale).round().clamp(-127.0, 127.0) as i8)
        .collect()
}

/// Computes cosine similarity in `[-1.0, 1.0]` between two `int8` quantized vectors.
pub fn cosine_similarity_int8(a: &[i8], b: &[i8]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot: i64 = 0;
    let mut norm_a: i64 = 0;
    let mut norm_b: i64 = 0;
    for (&x, &y) in a.iter().zip(b.iter()) {
        let xi = i64::from(x);
        let yi = i64::from(y);
        dot += xi * yi;
        norm_a += xi * xi;
        norm_b += yi * yi;
    }
    let denom = ((norm_a as f64).sqrt() * (norm_b as f64).sqrt()) as f32;
    if denom <= f32::EPSILON {
        0.0
    } else {
        (dot as f32) / denom
    }
}

/// Tier 2 experimental local CPU semantic projection provider with lazy on-demand cache
/// management and offline fallback.
///
/// INVARIANT: In v0.1, vectors are produced via deterministic token and n-gram feature hashing
/// rather than learned neural ONNX evaluation. True ONNX model evaluation is deferred to v0.2.
pub struct LocalCpuEmbeddingProvider {
    cache_manager: ModelCacheManager,
    dimensions: usize,
    allow_offline_fallback: bool,
    fetcher: Option<Box<dyn ModelFetcher>>,
}

impl LocalCpuEmbeddingProvider {
    /// Creates a provider rooted at the given `EmbeddingModelConfig`.
    /// By default, strict cache verification is enforced (`allow_offline_fallback = false`).
    pub fn new(config: EmbeddingModelConfig) -> Self {
        Self {
            cache_manager: ModelCacheManager::new(config),
            dimensions: DEFAULT_EMBEDDING_DIMENSION,
            allow_offline_fallback: false,
            fetcher: None,
        }
    }

    /// Enables or disables deterministic local CPU fallback when offline and model files are absent.
    pub fn with_offline_fallback(mut self, allow: bool) -> Self {
        self.allow_offline_fallback = allow;
        self
    }

    /// Attaches an on-demand `ModelFetcher` to lazily download missing ONNX artifacts.
    pub fn with_fetcher(mut self, fetcher: Box<dyn ModelFetcher>) -> Self {
        self.fetcher = Some(fetcher);
        self
    }

    /// Returns whether the configured ONNX model is currently cached on disk.
    pub fn is_cached(&self) -> bool {
        self.cache_manager.is_model_cached()
    }

    /// Generates `int8` embeddings for a batch of texts.
    pub fn embed_batch_int8(&self, texts: &[&str]) -> Result<Vec<Vec<i8>>, TriageError> {
        let model_seed = self.resolve_runtime_seed()?;
        texts
            .iter()
            .map(|text| self.compute_quantized_vector(text, model_seed))
            .collect()
    }

    fn resolve_runtime_seed(&self) -> Result<u64, TriageError> {
        match self.cache_manager.ensure_cached(self.fetcher.as_deref()) {
            Ok(model_dir) => read_model_signature(&model_dir),
            Err(_err) if self.allow_offline_fallback => Ok(0x5452_4941_4745_4149),
            Err(err) => Err(err),
        }
    }

    fn compute_quantized_vector(
        &self,
        text: &str,
        model_seed: u64,
    ) -> Result<Vec<i8>, TriageError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TriageError::Embedding(
                "cannot generate embedding for empty text".to_string(),
            ));
        }
        let float_vec = project_text_to_f32(trimmed, self.dimensions, model_seed);
        Ok(quantize_f32_to_int8(&float_vec))
    }
}

impl EmbeddingProvider for LocalCpuEmbeddingProvider {
    fn embed_int8(&self, text: &str) -> Result<Vec<i8>, TriageError> {
        let model_seed = self.resolve_runtime_seed()?;
        self.compute_quantized_vector(text, model_seed)
    }
}

fn read_model_signature(model_dir: &Path) -> Result<u64, TriageError> {
    let onnx_path = model_dir.join("model.onnx");
    let bytes = fs::read(&onnx_path).map_err(|e| {
        TriageError::Embedding(format!(
            "failed to read cached ONNX model '{}': {e}",
            onnx_path.display()
        ))
    })?;
    Ok(fnv1a_64(
        &bytes.iter().copied().take(256).collect::<Vec<u8>>(),
    ))
}

/// Projects normalized text tokens and character n-grams into a dense `f32` vector space.
fn project_text_to_f32(text: &str, dimensions: usize, model_seed: u64) -> Vec<f32> {
    let mut accum = vec![0.0_f32; dimensions];
    let normalized = fold_ascii_diacritics(text);
    let content_tokens: Vec<&str> = normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3 && !is_stopword(t))
        .collect();

    for (pos, &token) in content_tokens.iter().enumerate() {
        accumulate_feature(&mut accum, token.as_bytes(), model_seed, 1.0);
        // Word prefix/stem features (3..=6 chars) for morphological resilience
        for len in 3..=6 {
            if token.len() >= len {
                accumulate_feature(&mut accum, &token.as_bytes()[..len], model_seed, 0.9);
            }
        }
        // Adjacent content token bi-gram context
        if let Some(&next_token) = content_tokens.get(pos + 1) {
            let bigram = format!("{token}_{next_token}");
            accumulate_feature(&mut accum, bigram.as_bytes(), model_seed, 0.35);
        }
    }

    if content_tokens.is_empty() {
        accumulate_feature(&mut accum, normalized.as_bytes(), model_seed, 1.0);
    }

    accum
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "seu"
            | "sua"
            | "seus"
            | "suas"
            | "nos"
            | "das"
            | "dos"
            | "para"
            | "com"
            | "uma"
            | "ola"
            | "segue"
            | "prezado"
            | "esta"
            | "foi"
            | "the"
            | "and"
            | "for"
            | "you"
            | "your"
            | "are"
            | "with"
            | "from"
    )
}

fn accumulate_feature(accum: &mut [f32], feature: &[u8], seed: u64, weight: f32) {
    let dim = accum.len();
    if dim == 0 {
        return;
    }
    let h1 = fnv1a_64_seeded(feature, seed);
    let h2 = fnv1a_64_seeded(feature, seed ^ 0x9E37_79B9_7F4A_7C15);

    let idx1 = (h1 as usize) % dim;
    let sign1 = if (h1 >> 63) == 0 { 1.0 } else { -1.0 };
    accum[idx1] += sign1 * weight;

    let idx2 = (h2 as usize) % dim;
    let sign2 = if (h2 >> 63) == 0 { 1.0 } else { -1.0 };
    accum[idx2] += sign2 * (weight * 0.5);
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    fnv1a_64_seeded(bytes, 0xCBF2_9CE4_8422_2325)
}

fn fnv1a_64_seeded(bytes: &[u8], initial: u64) -> u64 {
    let mut hash = initial;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01B3);
    }
    hash
}

fn fold_ascii_diacritics(input: &str) -> String {
    input
        .chars()
        .map(|c| match c.to_ascii_lowercase() {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            other => other,
        })
        .collect()
}

/// Background batch indexer that converts inbound message batches into `int8[384]` embeddings.
pub struct BackgroundEmbeddingIndexer<P: EmbeddingProvider> {
    provider: P,
    batch_size: usize,
}

impl<P: EmbeddingProvider> BackgroundEmbeddingIndexer<P> {
    /// Creates a batch indexer with a bounded `batch_size` (clamped to `1..=256`).
    pub fn new(provider: P, batch_size: usize) -> Self {
        Self {
            provider,
            batch_size: batch_size.clamp(1, 256),
        }
    }

    /// Returns the configured batch chunk size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Indexes a collection of documents in chunks of `batch_size`.
    pub fn index_documents(
        &self,
        documents: &[EmbeddingDocument],
    ) -> Result<Vec<IndexedEmbedding>, TriageError> {
        let mut results = Vec::with_capacity(documents.len());
        for chunk in documents.chunks(self.batch_size) {
            for doc in chunk {
                let composite = format!("{}\n{}", doc.subject.trim(), doc.body_text.trim());
                if composite.trim().is_empty() {
                    continue;
                }
                let embedding_int8 = self.provider.embed_int8(&composite)?;
                results.push(IndexedEmbedding {
                    message_id: doc.message_id.clone(),
                    embedding_int8,
                });
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct SyntheticModelFetcher {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }

    impl ModelFetcher for SyntheticModelFetcher {
        fn fetch_artifact(&self, model_name: &str, filename: &str) -> Result<Vec<u8>, TriageError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(TriageError::Embedding(
                    "network unreachable while downloading model".to_string(),
                ));
            }
            Ok(format!("synthetic-onnx-payload:{model_name}:{filename}").into_bytes())
        }
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("triage-mail-{label}-{nanos}"))
    }

    #[test]
    fn resolves_xdg_data_dir_strictly_and_falls_back_to_home() {
        let from_xdg =
            resolve_xdg_data_dir(Some("/var/lib/xdg-data"), Some("/home/user")).expect("xdg path");
        assert_eq!(from_xdg, PathBuf::from("/var/lib/xdg-data/triage-mail"));

        let from_home = resolve_xdg_data_dir(None, Some("/home/user")).expect("home fallback");
        assert_eq!(
            from_home,
            PathBuf::from("/home/user/.local/share/triage-mail")
        );

        let relative_err = resolve_xdg_data_dir(Some("relative/path"), None);
        assert!(relative_err.is_err());

        let missing_err = resolve_xdg_data_dir(None, None);
        assert!(missing_err.is_err());
    }

    #[test]
    fn rejects_path_traversal_in_model_name() {
        let cfg = EmbeddingModelConfig::with_data_dir("../evil", PathBuf::from("/tmp/triage-mail"));
        assert!(cfg.model_dir().is_err());
    }

    #[test]
    fn provisions_model_on_demand_and_reuses_cache_without_network() {
        let temp_root = unique_temp_dir("cache-test");
        let cfg = EmbeddingModelConfig::with_data_dir(MODEL_ALL_MINILM_L6_V2, temp_root.clone());
        assert_eq!(cfg.cache_dir, temp_root.join("models"));

        let calls = Arc::new(AtomicUsize::new(0));
        let fetcher = SyntheticModelFetcher {
            calls: Arc::clone(&calls),
            fail: false,
        };

        let provider = LocalCpuEmbeddingProvider::new(cfg.clone()).with_fetcher(Box::new(fetcher));
        assert!(!provider.is_cached());

        let vec1 = provider
            .embed_int8("Fatura do cartão e boleto bancário disponível")
            .expect("first embed triggers on-demand cache");
        assert_eq!(vec1.len(), DEFAULT_EMBEDDING_DIMENSION);
        assert!(provider.is_cached());
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        // Second call must hit disk cache with zero network fetches
        let vec2 = provider
            .embed_int8("Fatura do cartão e boleto bancário disponível")
            .expect("second embed uses cached files");
        assert_eq!(vec1, vec2);
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn handles_offline_absence_with_error_or_graceful_fallback() {
        let temp_root = unique_temp_dir("offline-test");
        let cfg = EmbeddingModelConfig::with_data_dir(MODEL_BGE_SMALL_EN_V1_5, temp_root.clone());

        // Strict mode without cache or fetcher returns descriptive error
        let strict_provider = LocalCpuEmbeddingProvider::new(cfg.clone());
        let err = strict_provider
            .embed_int8("Security verification code")
            .expect_err("strict uncached provider must error");
        assert!(err.to_string().contains("not cached"));

        // Failing network fetcher with offline fallback enabled succeeds gracefully
        let calls = Arc::new(AtomicUsize::new(0));
        let failing_fetcher = SyntheticModelFetcher { calls, fail: true };
        let fallback_provider = LocalCpuEmbeddingProvider::new(cfg)
            .with_fetcher(Box::new(failing_fetcher))
            .with_offline_fallback(true);

        let vec = fallback_provider
            .embed_int8("Security verification code")
            .expect("offline fallback must succeed");
        assert_eq!(vec.len(), DEFAULT_EMBEDDING_DIMENSION);
        assert!(vec.iter().any(|&v| v != 0));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn scalar_quantization_preserves_384_dimensions_and_semantic_similarity() {
        let temp_root = unique_temp_dir("similarity-test");
        let cfg = EmbeddingModelConfig::with_data_dir(MODEL_ALL_MINILM_L6_V2, temp_root.clone());
        let provider = LocalCpuEmbeddingProvider::new(cfg).with_offline_fallback(true);

        let invoice_a = provider
            .embed_int8("Segue a fatura e boleto de pagamento da sua conta mensal")
            .expect("embed invoice_a");
        let invoice_b = provider
            .embed_int8("Fatura mensal disponível para pagamento via boleto")
            .expect("embed invoice_b");
        let shipping_c = provider
            .embed_int8("Seu pacote foi postado nos Correios com código de rastreamento")
            .expect("embed shipping_c");

        assert_eq!(invoice_a.len(), DEFAULT_EMBEDDING_DIMENSION);
        assert_eq!(invoice_b.len(), DEFAULT_EMBEDDING_DIMENSION);
        assert_eq!(shipping_c.len(), DEFAULT_EMBEDDING_DIMENSION);

        // Verify full dynamic range [-127, 127] saturation on max component
        let max_abs = invoice_a.iter().map(|&v| v.unsigned_abs()).max().unwrap();
        assert_eq!(max_abs, 127);

        let sim_related = cosine_similarity_int8(&invoice_a, &invoice_b);
        let sim_unrelated = cosine_similarity_int8(&invoice_a, &shipping_c);
        assert!(
            sim_related > sim_unrelated,
            "related financial emails should have higher similarity ({sim_related}) than unrelated shipping email ({sim_unrelated})"
        );
        assert!(
            sim_related > 0.45,
            "related similarity should be strong, got {sim_related}"
        );
    }

    #[test]
    fn background_batch_indexer_processes_chunks_and_skips_empty() {
        let temp_root = unique_temp_dir("batch-test");
        let cfg = EmbeddingModelConfig::with_data_dir(MODEL_ALL_MINILM_L6_V2, temp_root);
        let provider = LocalCpuEmbeddingProvider::new(cfg).with_offline_fallback(true);
        let indexer = BackgroundEmbeddingIndexer::new(provider, 2);
        assert_eq!(indexer.batch_size(), 2);

        let docs = vec![
            EmbeddingDocument {
                message_id: "msg-1@example.com".to_string(),
                subject: "Nota Fiscal Eletrônica".to_string(),
                body_text: "Sua NF-e foi emitida com sucesso.".to_string(),
            },
            EmbeddingDocument {
                message_id: "msg-empty@example.com".to_string(),
                subject: "   ".to_string(),
                body_text: "".to_string(),
            },
            EmbeddingDocument {
                message_id: "msg-2@example.com".to_string(),
                subject: "Your login OTP code".to_string(),
                body_text: "Use verification code 839201 to sign in.".to_string(),
            },
        ];

        let indexed = indexer.index_documents(&docs).expect("batch indexing");
        assert_eq!(indexed.len(), 2);
        assert_eq!(indexed[0].message_id, "msg-1@example.com");
        assert_eq!(indexed[0].embedding_int8.len(), DEFAULT_EMBEDDING_DIMENSION);
        assert_eq!(indexed[1].message_id, "msg-2@example.com");
        assert_eq!(indexed[1].embedding_int8.len(), DEFAULT_EMBEDDING_DIMENSION);
    }
}
