use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use triage_ai::{
    cosine_similarity_int8, resolve_xdg_data_dir, BackgroundEmbeddingIndexer, EmbeddingDocument,
    EmbeddingModelConfig, EmbeddingProvider, LocalCpuEmbeddingProvider, ModelFetcher, TriageError,
    DEFAULT_EMBEDDING_DIMENSION, MODEL_ALL_MINILM_L6_V2, MODEL_BGE_SMALL_EN_V1_5,
    REQUIRED_MODEL_FILES,
};

struct MockArtifactServer {
    fetch_count: Arc<AtomicUsize>,
    offline: bool,
}

impl ModelFetcher for MockArtifactServer {
    fn fetch_artifact(&self, model_name: &str, filename: &str) -> Result<Vec<u8>, TriageError> {
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        if self.offline {
            return Err(TriageError::Embedding(
                "offline: simulated network absence".to_string(),
            ));
        }
        Ok(format!("mock-onnx-weights-v1:{model_name}:{filename}").into_bytes())
    }
}

fn scratch_xdg_dir(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("triage-mail-tier2-{test_name}-{nanos}"))
}

#[test]
fn tier2_xdg_cache_provisioning_and_batch_semantic_search_ranking() {
    let xdg_root = scratch_xdg_dir("xdg-batch");
    let data_dir =
        resolve_xdg_data_dir(Some(xdg_root.to_str().unwrap()), None).expect("resolve xdg");
    assert_eq!(data_dir, xdg_root.join("triage-mail"));

    let config = EmbeddingModelConfig::with_data_dir(MODEL_ALL_MINILM_L6_V2, data_dir.clone());
    assert_eq!(
        config.cache_dir,
        xdg_root.join("triage-mail").join("models")
    );

    let fetch_count = Arc::new(AtomicUsize::new(0));
    let fetcher = MockArtifactServer {
        fetch_count: Arc::clone(&fetch_count),
        offline: false,
    };

    let provider = LocalCpuEmbeddingProvider::new(config.clone()).with_fetcher(Box::new(fetcher));
    assert!(!provider.is_cached());

    let indexer = BackgroundEmbeddingIndexer::new(provider, 4);

    let documents = vec![
        EmbeddingDocument {
            message_id: "<inv-01@example.com>".to_string(),
            subject: "Fatura de energia elétrica e boleto bancário".to_string(),
            body_text: "Olá cliente@example.com, segue o boleto para pagamento da fatura de abril."
                .to_string(),
        },
        EmbeddingDocument {
            message_id: "<inv-02@example.org>".to_string(),
            subject: "Boleto da mensalidade e nota fiscal".to_string(),
            body_text: "Prezado usuário, sua fatura mensal já está disponível para pagamento."
                .to_string(),
        },
        EmbeddingDocument {
            message_id: "<sec-01@example.com>".to_string(),
            subject: "Security Alert: New sign-in code".to_string(),
            body_text: "Use verification code 491022 to complete two-factor authentication."
                .to_string(),
        },
    ];

    let indexed = indexer
        .index_documents(&documents)
        .expect("batch indexing should provision model once and embed all docs");

    assert_eq!(indexed.len(), 3);
    assert_eq!(
        fetch_count.load(Ordering::SeqCst),
        REQUIRED_MODEL_FILES.len(),
        "model files must be downloaded exactly once on demand"
    );

    // Verify files exist on disk under $XDG_DATA_HOME/triage-mail/models/all-MiniLM-L6-v2
    let model_dir = config.model_dir().expect("valid model dir");
    for required in REQUIRED_MODEL_FILES {
        assert!(
            model_dir.join(required).is_file(),
            "expected cached file {}",
            required
        );
    }

    // Verify 384-dimension int8 quantization & semantic clustering
    for item in &indexed {
        assert_eq!(item.embedding_int8.len(), DEFAULT_EMBEDDING_DIMENSION);
        let max_magnitude = item
            .embedding_int8
            .iter()
            .map(|&x| x.unsigned_abs())
            .max()
            .unwrap_or(0);
        assert_eq!(
            max_magnitude, 127,
            "absmax int8 quantization must use full [-127, 127] dynamic range"
        );
    }

    let sim_invoices =
        cosine_similarity_int8(&indexed[0].embedding_int8, &indexed[1].embedding_int8);
    let sim_cross = cosine_similarity_int8(&indexed[0].embedding_int8, &indexed[2].embedding_int8);
    assert!(
        sim_invoices > sim_cross,
        "expected invoice pair similarity ({sim_invoices}) > invoice-vs-security ({sim_cross})"
    );

    let _ = fs::remove_dir_all(&xdg_root);
}

#[test]
fn tier2_offline_resilience_when_network_unavailable() {
    let xdg_root = scratch_xdg_dir("offline-resilience");
    let data_dir =
        resolve_xdg_data_dir(Some(xdg_root.to_str().unwrap()), None).expect("resolve xdg");
    let config = EmbeddingModelConfig::with_data_dir(MODEL_BGE_SMALL_EN_V1_5, data_dir);

    // 1. Strict mode fails cleanly with a descriptive error when offline and uncached
    let strict_provider =
        LocalCpuEmbeddingProvider::new(config.clone()).with_fetcher(Box::new(MockArtifactServer {
            fetch_count: Arc::new(AtomicUsize::new(0)),
            offline: true,
        }));
    let strict_res = strict_provider.embed_int8("Order shipped via Correios");
    assert!(strict_res.is_err());

    // 2. Offline fallback mode continues operating deterministically without panicking
    let resilient_provider = LocalCpuEmbeddingProvider::new(config)
        .with_fetcher(Box::new(MockArtifactServer {
            fetch_count: Arc::new(AtomicUsize::new(0)),
            offline: true,
        }))
        .with_offline_fallback(true);

    let batch = resilient_provider
        .embed_batch_int8(&[
            "Rastreamento do seu pedido nos Correios",
            "Código de rastreio da entrega nos Correios",
        ])
        .expect("offline fallback batch embedding should succeed");

    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0].len(), DEFAULT_EMBEDDING_DIMENSION);
    assert_eq!(batch[1].len(), DEFAULT_EMBEDDING_DIMENSION);
    assert!(cosine_similarity_int8(&batch[0], &batch[1]) > 0.25);

    let _ = fs::remove_dir_all(&xdg_root);
}
