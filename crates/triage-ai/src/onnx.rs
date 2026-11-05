//! Pinned CPU-only learned embeddings. Neither construction nor inference downloads files.
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use ort::{session::Session, value::Tensor};
use sha2::{Digest, Sha256};
use tokenizers::{Tokenizer, TruncationParams};

use crate::{
    embeddings::quantize_f32_to_int8, EmbeddingModelConfig, EmbeddingProvider, TriageError,
};

pub const MODEL_SHA256: &str = "afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1";
pub const TOKENIZER_SHA256: &str =
    "da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0";
/// Identity binds weights, tokenizer, truncation, pooling, normalization and int8 conversion.
pub const GENERATION: &str = "minilm384-afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1-da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0-256-mean-l2-absmax-v1";
pub const MAX_INPUT_BYTES: usize = 32 * 1024;
pub const MAX_TOKENS: usize = 256;

pub(crate) fn invalid() -> TriageError {
    TriageError::Embedding("local ONNX model unavailable or invalid".into())
}

/// Returns bounded, checksum-verified artifact bytes; rejects symlinks and other file types.
pub fn verified_file(path: &Path, expected: &str, limit: u64) -> Result<Vec<u8>, TriageError> {
    let meta = fs::symlink_metadata(path).map_err(|_| invalid())?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > limit {
        return Err(invalid());
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| invalid())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > limit || format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err(invalid());
    }
    Ok(bytes)
}

pub fn verify_model(config: &EmbeddingModelConfig) -> Result<(Vec<u8>, Vec<u8>), TriageError> {
    if config.model_name != crate::embeddings::MODEL_ALL_MINILM_L6_V2 {
        return Err(invalid());
    }
    let dir = config.model_dir()?;
    Ok((
        verified_file(&dir.join("model.onnx"), MODEL_SHA256, 128 * 1024 * 1024)?,
        verified_file(
            &dir.join("tokenizer.json"),
            TOKENIZER_SHA256,
            8 * 1024 * 1024,
        )?,
    ))
}

/// The separately installed, pinned native CPU library is never fetched by the application.
pub fn runtime_path(config: &EmbeddingModelConfig) -> PathBuf {
    config
        .cache_dir
        .join("onnxruntime-1.22.0")
        .join(runtime_spec().map(|s| s.0).unwrap_or("unsupported"))
}
/// Filename and SHA-256 of the official Microsoft CPU release for this target.
pub fn runtime_spec() -> Option<(&'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some((
            "libonnxruntime.so.1.22.0",
            "3da6146e14e7b8aaec625dde11d6114c7457c87a5f93d744897da8781e35c673",
        )),
        ("linux", "aarch64") => Some((
            "libonnxruntime.so.1.22.0",
            "0afd69a0ae38c5099fd0e8604dda398ac43dee67cd9c6394b5142b19e82528de",
        )),
        ("macos", "x86_64") => Some((
            "libonnxruntime.1.22.0.dylib",
            "283e595e61cf65df7a6b1d59a1616cbd35c8b6399dd90d799d99b71a3ff83160",
        )),
        ("macos", "aarch64") => Some((
            "libonnxruntime.1.22.0.dylib",
            "2b885992d3d6fa4130d39ec84a80d7504ff52750027c547bb22c86165f19406a",
        )),
        ("windows", "x86_64") => Some((
            "onnxruntime.dll",
            "579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559",
        )),
        _ => None,
    }
}

fn initialize_runtime(path: &Path) -> Result<(), TriageError> {
    // INVARIANT: Never load native code from an unverified cache or ambient library path.
    let (_, checksum) = runtime_spec().ok_or_else(invalid)?;
    verified_file(path, checksum, 64 * 1024 * 1024)?;
    static INITIALIZED: OnceLock<bool> = OnceLock::new();
    if !*INITIALIZED.get_or_init(|| {
        // ort's dynamic loader can panic on missing dependent system libraries.
        std::panic::catch_unwind(|| {
            ort::init_from(path.to_string_lossy())
                .with_telemetry(false)
                .commit()
                .is_ok()
        })
        .unwrap_or(false)
    }) {
        return Err(invalid());
    }
    Ok(())
}

struct Runtime {
    session: Session,
    tokenizer: Tokenizer,
}

/// Retains one CPU session and serializes inference; missing files can be installed later.
pub struct OnnxEmbeddingProvider {
    config: EmbeddingModelConfig,
    runtime: Mutex<Option<Runtime>>,
}
impl OnnxEmbeddingProvider {
    pub fn new(config: EmbeddingModelConfig) -> Self {
        Self {
            config,
            runtime: Mutex::new(None),
        }
    }
    pub fn prepare(&self) -> Result<(), TriageError> {
        let mut runtime = self.runtime.lock().map_err(|_| invalid())?;
        if runtime.is_none() {
            *runtime = Some(self.load()?);
        }
        Ok(())
    }
    fn load(&self) -> Result<Runtime, TriageError> {
        let (model, tokenizer) = verify_model(&self.config)?;
        initialize_runtime(&runtime_path(&self.config))?;
        let mut tokenizer = Tokenizer::from_bytes(tokenizer).map_err(|_| invalid())?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_TOKENS,
                ..Default::default()
            }))
            .map_err(|_| invalid())?;
        tokenizer.with_padding(None);
        let session = Session::builder()
            .map_err(|_| invalid())?
            .with_intra_threads(2)
            .map_err(|_| invalid())?
            .with_inter_threads(1)
            .map_err(|_| invalid())?
            .commit_from_memory(&model)
            .map_err(|_| invalid())?;
        Ok(Runtime { session, tokenizer })
    }
}
impl EmbeddingProvider for OnnxEmbeddingProvider {
    fn embed_int8(&self, text: &str) -> Result<Vec<i8>, TriageError> {
        self.prepare()?;
        let mut guard = self.runtime.lock().map_err(|_| invalid())?;
        let rt = guard.as_mut().ok_or_else(invalid)?;
        let text = bounded_text(text);
        if text.trim().is_empty() {
            return Err(invalid());
        }
        let encoded = rt.tokenizer.encode(text, true).map_err(|_| invalid())?;
        let ids: Vec<i64> = encoded.get_ids().iter().map(|n| i64::from(*n)).collect();
        let mask: Vec<i64> = encoded
            .get_attention_mask()
            .iter()
            .map(|n| i64::from(*n))
            .collect();
        let types: Vec<i64> = encoded
            .get_type_ids()
            .iter()
            .map(|n| i64::from(*n))
            .collect();
        let len = ids.len();
        if len == 0 || len > MAX_TOKENS {
            return Err(invalid());
        }
        let tensor = |v: Vec<i64>| Tensor::from_array(([1usize, len], v)).map_err(|_| invalid());
        let output = rt.session.run(ort::inputs!["input_ids" => tensor(ids)?, "attention_mask" => tensor(mask.clone())?, "token_type_ids" => tensor(types)?]).map_err(|_| invalid())?;
        let (shape, values) = output
            .get("last_hidden_state")
            .ok_or_else(invalid)?
            .try_extract_tensor::<f32>()
            .map_err(|_| invalid())?;
        if shape.as_ref() != [1, len as i64, 384] {
            return Err(invalid());
        }
        pool_quantize(values, &mask)
    }
}

pub fn bounded_text(text: &str) -> &str {
    let mut end = text.len().min(MAX_INPUT_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn pool_quantize(values: &[f32], mask: &[i64]) -> Result<Vec<i8>, TriageError> {
    if values.len() != mask.len() * 384 || !values.iter().all(|v| v.is_finite()) {
        return Err(invalid());
    }
    let count = mask.iter().filter(|v| **v == 1).count();
    if count == 0 {
        return Err(invalid());
    }
    let mut pooled = vec![0f32; 384];
    for (token, selected) in values.as_chunks::<384>().0.iter().zip(mask) {
        if *selected == 1 {
            for (sum, value) in pooled.iter_mut().zip(token) {
                *sum += value / count as f32;
            }
        }
    }
    let norm = pooled.iter().map(|v| v * v).sum::<f32>().sqrt();
    if !norm.is_finite() || norm <= f32::EPSILON {
        return Err(invalid());
    }
    for v in &mut pooled {
        *v /= norm;
    }
    Ok(quantize_f32_to_int8(&pooled))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pooling_is_masked_finite_and_nonzero() {
        let mut data = vec![1f32; 768];
        data[384..].fill(100.);
        assert_eq!(pool_quantize(&data, &[1, 0]).unwrap(), vec![127; 384]);
        data[1] = f32::NAN;
        assert!(pool_quantize(&data, &[1, 1]).is_err());
        assert!(pool_quantize(&[0.; 384], &[1]).is_err());
        assert!(pool_quantize(&[], &[1]).is_err());
        assert!(pool_quantize(&[1.; 384], &[0]).is_err());
    }
    #[test]
    fn input_bound_preserves_utf8() {
        let text = "á".repeat(MAX_INPUT_BYTES);
        assert_eq!(bounded_text(&text).len(), MAX_INPUT_BYTES);
        let text = format!("a{}", text);
        assert_eq!(bounded_text(&text).len(), MAX_INPUT_BYTES - 1);
    }
}

/// Replaces obsolete/corrupt cache files only after both new artifacts pass the manifest.
pub fn install_model(
    config: &EmbeddingModelConfig,
    model: &[u8],
    tokenizer: &[u8],
) -> Result<(), TriageError> {
    if config.model_name != crate::embeddings::MODEL_ALL_MINILM_L6_V2
        || format!("{:x}", Sha256::digest(model)) != MODEL_SHA256
        || format!("{:x}", Sha256::digest(tokenizer)) != TOKENIZER_SHA256
    {
        return Err(invalid());
    }
    let dir = config.model_dir()?;
    fs::create_dir_all(&dir).map_err(|_| invalid())?;
    for (name, bytes) in [("model.onnx", model), ("tokenizer.json", tokenizer)] {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| invalid())?
            .as_nanos();
        let tmp = dir.join(format!("{name}.{}.{unique}.installing", std::process::id()));
        use std::io::Write;
        // INVARIANT: create_new refuses abandoned files and symlinks; publication is atomic.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|_| invalid())?;
        let result = (|| {
            file.write_all(bytes).map_err(|_| invalid())?;
            file.sync_all().map_err(|_| invalid())?;
            fs::rename(&tmp, dir.join(name)).map_err(|_| invalid())
        })();
        if result.is_err() {
            let _ = fs::remove_file(tmp);
        }
        result?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "onnx/tests.rs"]
mod integration_tests;
