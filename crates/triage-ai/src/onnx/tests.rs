use super::*;

#[test]
fn cache_requires_exact_manifest_and_refuses_bad_installs() {
    let root = std::env::temp_dir().join(format!("triage-onnx-invalid-{}", std::process::id()));
    let config = EmbeddingModelConfig::with_data_dir(
        crate::embeddings::MODEL_ALL_MINILM_L6_V2,
        root.clone(),
    );
    fs::create_dir_all(config.model_dir().unwrap()).unwrap();
    fs::write(
        config.model_dir().unwrap().join("model.onnx"),
        b"not an ONNX model",
    )
    .unwrap();
    fs::write(config.model_dir().unwrap().join("tokenizer.json"), b"{}").unwrap();
    assert!(verify_model(&config).is_err());
    assert!(install_model(&config, b"invalid", b"{}").is_err());
    assert!(OnnxEmbeddingProvider::new(config)
        .embed_int8("synthetic text")
        .is_err());
    fs::remove_dir_all(root).unwrap();
}

fn varint(mut n: u64) -> Vec<u8> {
    let mut b = Vec::new();
    while n >= 128 {
        b.push((n as u8) | 128);
        n >>= 7;
    }
    b.push(n as u8);
    b
}
fn integer(field: u8, n: u64) -> Vec<u8> {
    let mut b = vec![field << 3];
    b.extend(varint(n));
    b
}
fn field(field: u8, data: impl AsRef<[u8]>) -> Vec<u8> {
    let data = data.as_ref();
    let mut b = vec![(field << 3) | 2];
    b.extend(varint(data.len() as u64));
    b.extend(data);
    b
}
fn value_info(name: &str) -> Vec<u8> {
    let shape = [field(1, integer(1, 1)), field(1, integer(1, 384))].concat();
    let tensor = [integer(1, 1), field(2, shape)].concat();
    [field(1, name), field(2, field(1, tensor))].concat()
}
fn identity_graph() -> Vec<u8> {
    // INVARIANT: This is a tiny synthetic Identity graph, not downloaded model weights.
    let node = [field(1, "x"), field(2, "y"), field(4, "Identity")].concat();
    let graph = [
        field(1, node),
        field(2, "synthetic"),
        field(11, value_info("x")),
        field(12, value_info("y")),
    ]
    .concat();
    [integer(1, 8), field(7, graph), field(8, integer(2, 13))].concat()
}

#[test]
#[ignore = "requires explicitly provisioned pinned CPU runtime and model; see docs/onnx.md"]
fn real_model_and_synthetic_graph_execute_offline() {
    let root = PathBuf::from(
        std::env::var("TRIAGE_ONNX_TEST_DATA").expect("explicit test data directory"),
    );
    let config =
        EmbeddingModelConfig::with_data_dir(crate::embeddings::MODEL_ALL_MINILM_L6_V2, root);
    initialize_runtime(&runtime_path(&config)).unwrap();
    let mut session = Session::builder()
        .unwrap()
        .with_intra_threads(2)
        .unwrap()
        .commit_from_memory(&identity_graph())
        .unwrap();
    let expected: Vec<f32> = (0..384).map(|n| n as f32 / 384.).collect();
    let output = session
        .run(ort::inputs!["x"=>Tensor::from_array(([1usize,384],expected.clone())).unwrap()])
        .unwrap();
    assert_eq!(output["y"].try_extract_tensor::<f32>().unwrap().1, expected);
    assert!(Session::builder()
        .unwrap()
        .commit_from_memory(b"invalid synthetic graph")
        .is_err());
    let start = std::time::Instant::now();
    let provider = OnnxEmbeddingProvider::new(config);
    provider.prepare().unwrap();
    let cold = start.elapsed();
    let documents = [
        "Your invoice payment is due tomorrow.",
        "The parcel has shipped and delivery is scheduled.",
        "A fatura vence amanhã. O pagamento está pendente.",
        "Sua encomenda foi enviada e será entregue amanhã.",
    ];
    let mut vectors = Vec::new();
    let start = std::time::Instant::now();
    for text in documents {
        let v = provider.embed_int8(text).unwrap();
        assert_eq!(v.len(), 384);
        assert!(v.iter().any(|n| *n != 0));
        vectors.push(v);
    }
    let warm = start.elapsed();
    assert_ne!(vectors[0], vectors[1]);
    assert_eq!(provider.embed_int8(documents[0]).unwrap(), vectors[0]);
    assert!(provider.embed_int8("").is_err());
    eprintln!("cold_prepare_ms={:.3}; four_documents_ms={:.3}; en_pair_cosine={:.4}; pt_pair_cosine={:.4}",cold.as_secs_f64()*1000.,warm.as_secs_f64()*1000.,crate::embeddings::cosine_similarity_int8(&vectors[0],&vectors[1]),crate::embeddings::cosine_similarity_int8(&vectors[2],&vectors[3]));
}
