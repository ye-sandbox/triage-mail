use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use triage_ai::{
    HttpLlmSummarizer, LlmClientConfig, LlmProviderConfig, LlmSummarizer, RedactedApiKey,
    TriageError,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ollama_summarizer_trait_integration_and_truncation() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).await.unwrap();
        let raw_req = String::from_utf8_lossy(&buf[..n]).to_string();

        assert!(raw_req.starts_with("POST /api/generate HTTP/1.1\r\n"));
        assert!(raw_req.contains("\"model\":\"qwen2.5:3b\""));
        assert!(raw_req.contains("\"stream\":false"));
        // Verify truncation bounded the body and omitted the trailing marker
        assert!(!raw_req.contains("TRAILING_MARKER_SHOULD_BE_TRUNCATED"));

        let body = r#"{"model":"qwen2.5:3b","response":"- Security alert: new login from Linux.\n- Action: review session settings.","done":true}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(resp.as_bytes()).await.unwrap();
    });

    let summarizer: Box<dyn LlmSummarizer> = Box::new(
        HttpLlmSummarizer::new(LlmClientConfig {
            enabled: true,
            provider: LlmProviderConfig::Ollama {
                endpoint: format!("http://{}", addr),
                model: "qwen2.5:3b".to_string(),
            },
            timeout: Duration::from_secs(5),
            max_input_chars: 90,
            allow_loopback_http_for_byok_tests: false,
        })
        .unwrap(),
    );

    let long_body = format!(
        "New sign-in detected on your account from Linux desktop. {} TRAILING_MARKER_SHOULD_BE_TRUNCATED",
        "x".repeat(200)
    );
    let summary = summarizer
        .summarize("Security Alert: New Sign-In", &long_body)
        .unwrap();

    assert_eq!(
        summary,
        "- Security alert: new login from Linux.\n- Action: review session settings."
    );
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_byok_http_error_redacts_secrets_and_pii() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf).await.unwrap();

        let err_json = r#"{"error":{"message":"Invalid API key sk-synthetic-secret-777"}}"#;
        let resp = format!(
            "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            err_json.len(),
            err_json
        );
        stream.write_all(resp.as_bytes()).await.unwrap();
    });

    let api_key_str = "sk-synthetic-secret-777";
    let pii_body = "Patient record ID 9918273645 confidential";
    let summarizer = HttpLlmSummarizer::new(LlmClientConfig {
        enabled: true,
        provider: LlmProviderConfig::openai_compatible(
            format!("http://{}/v1/chat/completions", addr),
            "gpt-4o-mini",
            RedactedApiKey::new(api_key_str),
        ),
        timeout: Duration::from_secs(5),
        max_input_chars: 1000,
        allow_loopback_http_for_byok_tests: true,
    })
    .unwrap();

    let err = summarizer
        .summarize("Medical Lab Report", pii_body)
        .unwrap_err();
    assert_eq!(
        err,
        TriageError::Llm("BYOK LLM endpoint returned HTTP 401".to_string())
    );
    assert!(!err.to_string().contains(api_key_str));
    assert!(!err.to_string().contains(pii_body));
    server.await.unwrap();
}
